pub mod config;
pub mod database;
pub mod media;
pub mod schema;

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use database::{establish_connection, run_migrations, DbConnection};
use log::{debug, error, info};
use reqwest::Client;
use shared_libs::json_templates::MediaItem;
use tokio::sync::Mutex;

use crate::config::Config;

type Id = String;
type Passcode = String;

pub fn agent() -> Client {
    Client::new()
}

/// Load new items from the server for download :)
pub async fn load_new_items(
    config: &Config,
    agent: &Client,
    connection: Arc<Mutex<DbConnection>>,
    queue: &Mutex<VecDeque<MediaItem>>,
    processing: &AtomicBool,
    waiting: &AtomicBool,
) {
    let mut e_backoff = 1;
    let mut last_refresh_time = Instant::now();
    // first request should always try to reload if possible, but only if the initial scan has
    // been completed
    let mut reload = config.initial_scan_complete();

    loop {
        if !processing.load(Ordering::Relaxed) && queue.lock().await.is_empty() {
            let items = match media::get_media_items(config, agent, reload).await {
                Ok(i) => i,
                Err(e) => {
                    error!(
                        "failed to collect media items for download due to error: {}",
                        e
                    );
                    error!("retrying in {} seconds", e_backoff);
                    tokio::time::sleep(Duration::from_secs(e_backoff)).await;
                    e_backoff *= 2;
                    if e_backoff > 1800 {
                        e_backoff = 1800;
                    }
                    continue;
                }
            };

            e_backoff = 1;
            last_refresh_time = Instant::now();

            if items.is_empty() {
                info!("api returned no new items to download");
                continue;
            }

            if all_present(&items, &mut *connection.lock().await) {
                if !config.initial_scan_complete() {
                    info!("all items are present in the database, initial scan complete");
                    config
                        .set_initial_scan_complete(&mut *connection.lock().await)
                        .expect("failed to set initial scan complete");
                } else {
                    info!("all items are present in the database, no new items to download - sleeping for 15 minutes");
                    waiting.store(true, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_secs(60 * 30)).await;
                }
            }

            queue.lock().await.extend(items);
            waiting.store(false, Ordering::Relaxed);
            reload = false;
        }

        // if last refresh > 50 minutes, recollect all media items
        if last_refresh_time.elapsed().as_secs() > (60 * 55) {
            info!("last refresh was more than 55 minutes ago, reloading all media items");

            let mut lock = queue.lock().await;
            if config.initial_scan_complete() {
                info!("refreshing media items");
                reload = true;
            } else {
                error!(
                    "initial scan not complete, but 50 minutes have passed, this should not happen"
                );
                for item in lock.iter() {
                    let db_conn = connection.clone();
                    let db_item = item.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        database::save_media_item(&mut *db_conn.blocking_lock(), &db_item)
                    });

                    match res.await {
                        Ok(_) => debug!("saved media item to database"),
                        Err(e) => error!("failed to save media item to database {}", e),
                    }
                }
                info!("saved all media items to database, initial scan complete");
            }

            lock.clear();
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
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
pub async fn download_items(
    config: &Config,
    agent: &Client,
    connection: Arc<Mutex<DbConnection>>,
    queue: &Mutex<VecDeque<MediaItem>>,
    processing: &AtomicBool,
    waiting: &AtomicBool,
) {
    loop {
        if !queue.lock().await.is_empty() {
            processing.store(true, Ordering::Relaxed);
        } else {
            processing.store(false, Ordering::Relaxed);

            // if we are waiting for the download - wait 10 minutes, otherwise 5 seconds
            if waiting.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_secs(60 * 10)).await;
            } else {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }

        {
            let mut locked = queue.lock().await;
            if let Some(mut item) = locked.pop_front() {
                if database::in_database(&mut *connection.lock().await, &item.id).unwrap() {
                    continue;
                }

                info!("downloading {}", item.baseUrl);
                item.download_success = false;
                item.download_attempts += 1;
                if media::download_item(config, agent, &item).await.is_ok() {
                    info!("download successful");
                    item.download_success = true;
                }

                match (item.download_success, item.download_attempts) {
                    (true, _) | (false, 4) => {
                        if !item.download_success {
                            error!("failed to download item {} after 4 attempts", item.id);
                        }

                        let db_conn = connection.clone();
                        let res = tokio::task::spawn_blocking(move || {
                            database::save_media_item(&mut *db_conn.blocking_lock(), &item)
                        });

                        match res.await {
                            Ok(_) => info!("saved media item to database"),
                            Err(e) => error!("failed to save media item to database {}", e),
                        }
                    }
                    (false, _) => {
                        locked.push_back(item);
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub async fn download_scan(config: &Config, agent: &Client, database: DbConnection) {
    let database = Arc::new(Mutex::new(database));
    let download_queue: Mutex<VecDeque<MediaItem>> = Mutex::new(VecDeque::with_capacity(50));
    let processing = AtomicBool::new(false);
    let waiting = AtomicBool::new(false);

    tokio_scoped::scope(|scope| {
        // load new items
        scope.spawn(load_new_items(
            config,
            agent,
            database.clone(),
            &download_queue,
            &processing,
            &waiting,
        ));

        // download items
        scope.spawn(download_items(
            config,
            agent,
            database,
            &download_queue,
            &processing,
            &waiting,
        ));
    })
}

#[tokio::main]
pub async fn run() {
    //XXX: adjustable scan times
    //XXX: Testing

    pretty_env_logger::init();
    let agent = agent();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut database = establish_connection(&database_url).expect("failed to connect to database");
    run_migrations(&mut database).expect("failed to run migrations");

    let config = Config::load(&agent, &mut database)
        .await
        .expect("failed to load config");

    download_scan(&config, &agent, database).await;
}
