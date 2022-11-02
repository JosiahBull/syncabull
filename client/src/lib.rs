pub mod config;
pub mod database;
pub mod media;
pub mod schema;

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use database::{establish_connection, run_migrations, DbConnection};
use log::{error, info, warn};
use shared_libs::json_templates::MediaItem;
use ureq::Agent;

use crate::config::Config;

type Id = String;
type Passcode = String;

pub fn agent() -> Agent {
    Agent::new()
}

/// Load new items from the server for download :)
pub fn load_new_items(
    config: &Config,
    agent: &Agent,
    connection: &Mutex<&mut DbConnection>,
    queue: &Mutex<VecDeque<MediaItem>>,
    processing: &AtomicBool,
    waiting: &AtomicBool,
) {
    let mut e_backoff = 1;
    let mut last_refresh_time = Instant::now();
    let mut reload = false;

    loop {
        if !processing.load(Ordering::Relaxed) && queue.lock().unwrap().is_empty() {
            let items = match media::get_media_items(config, agent, reload) {
                Ok(i) => i,
                Err(e) => {
                    error!(
                        "failed to collect media items for download due to error: {}",
                        e
                    );
                    error!("retrying in {} seconds", e_backoff);
                    std::thread::sleep(Duration::from_secs(e_backoff));
                    e_backoff *= 2;
                    if e_backoff > 1800 {
                        e_backoff = 1800;
                    }
                    continue;
                }
            };

            e_backoff = 1;
            last_refresh_time = Instant::now();
            waiting.store(false, Ordering::Relaxed);
            reload = false;

            if all_present(&items, *connection.lock().unwrap()) {
                if !config.initial_scan_complete() {
                    info!("all items are present in the database, initial scan complete");
                    config
                        .set_initial_scan_complete(*connection.lock().unwrap())
                        .expect("failed to set initial scan complete");
                } else {
                    info!("all items are present in the database, no new items to download - sleeping for 15 minutes");
                    waiting.store(true, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_secs(60 * 30));
                }
            }

            // if some videos are still processing, we need to wait for them to finish
            if !all_processed(&items) {
                info!("some videos are still processing, waiting 5 minutes");
                std::thread::sleep(Duration::from_secs(60 * 5));
                reload = true;
                continue;
            }

            queue.lock().unwrap().extend(items);
        }

        // if last refresh > 50 minutes, recollect all media items
        if last_refresh_time.elapsed().as_secs() > (60 * 50) {
            info!("refreshing media items");
            queue.lock().unwrap().clear();
            reload = true;
            continue;
        }
    }
}

/// extract all items that are still processing into a separate queue
pub fn all_processed(items: &[MediaItem]) -> bool {
    // check if all items are ready
    items.iter().all(|item| {
        if let Some(meta) = &item.mediaMetadata {
            if let Some(video_meta) = &meta.video {
                if let Some(status) = &video_meta.status {
                    match status {
                        shared_libs::json_templates::VideoProcessingStatus::UNSPECIFIED => {
                            warn!(
                                "video {} has unspecified status, assuming it isn't ready",
                                item.id
                            );
                            return false;
                        }
                        shared_libs::json_templates::VideoProcessingStatus::PROCESSING => {
                            warn!("video {} is still processing", item.baseUrl);
                            return false;
                        }
                        shared_libs::json_templates::VideoProcessingStatus::READY => return true,
                        shared_libs::json_templates::VideoProcessingStatus::FAILED => {
                            panic!("should have been filtered out");
                        }
                    }
                }
            }
        }
        true
    })
}

/// check if all items in this queue have already been downloaded
pub fn all_present(items: &[MediaItem], connection: &mut DbConnection) -> bool {
    for item in items {
        if !database::in_database(connection, &item.id).unwrap() {
            return false;
        }
    }

    true
}

/// Download items that are in the queue
pub fn download_items(
    config: &Config,
    agent: &Agent,
    connection: &Mutex<&mut DbConnection>,
    queue: &Mutex<VecDeque<MediaItem>>,
    processing: &AtomicBool,
    waiting: &AtomicBool,
) {
    loop {
        if !queue.lock().unwrap().is_empty() {
            processing.store(true, Ordering::Relaxed);
        } else {
            processing.store(false, Ordering::Relaxed);

            // if we are waiting for the download - wait 10 minutes, otherwise 5 seconds
            if waiting.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(60 * 10));
            } else {
                std::thread::sleep(Duration::from_secs(5));
            }
        }

        if let Some(mut item) = queue.lock().unwrap().pop_front() {
            if database::in_database(*connection.lock().unwrap(), &item.id).unwrap() {
                continue;
            }

            info!("downloading {}", item.baseUrl);
            item.download_attempts += 1;
            if media::download_item(config, agent, &item).is_ok() {
                info!("download successful");
                item.download_success = true;
            }

            match (item.download_success, item.download_attempts) {
                (true, _) | (false, 4) => {
                    if !item.download_success {
                        error!("failed to download item {} after 4 attempts", item.id);
                    }

                    match database::save_media_item(*connection.lock().unwrap(), &item) {
                        Ok(_) => info!("saved media item to database"),
                        Err(e) => error!("failed to save media item to database {}", e),
                    }
                }
                (false, _) => {
                    queue.lock().unwrap().push_back(item);
                }
            }
        }
    }
}

pub fn download_scan(config: &Config, agent: &Agent, database: &mut DbConnection) {
    let database = Mutex::new(database);
    let download_queue: Mutex<VecDeque<MediaItem>> = Mutex::new(VecDeque::with_capacity(50));
    let processing = AtomicBool::new(false);
    let waiting = AtomicBool::new(false);

    // create thread scope
    std::thread::scope(|scope| {
        // load new items
        scope.spawn(|| {
            load_new_items(
                config,
                agent,
                &database,
                &download_queue,
                &processing,
                &waiting,
            )
        });

        // download items
        scope.spawn(|| {
            download_items(
                config,
                agent,
                &database,
                &download_queue,
                &processing,
                &waiting,
            )
        });
    });
}

pub fn run() {
    //XXX: api rate limits, we need to be aware of them
    //XXX: adjustable scan times
    //XXX: Testing

    pretty_env_logger::init();
    let agent = agent();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut database = establish_connection(&database_url).expect("failed to connect to database");
    run_migrations(&mut database).expect("failed to run migrations");

    let config = Config::load(&agent, &mut database).expect("failed to load config");

    download_scan(&config, &agent, &mut database);
}
