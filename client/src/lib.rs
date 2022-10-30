pub mod database;
pub mod schema;
pub mod media;
pub mod cli;
pub mod config;

use std::{collections::VecDeque, time::Duration};

use database::{establish_connection, run_migrations, DbConnection};
use log::{error, info, debug};
use shared_libs::json_templates::MediaItem;
use ureq::Agent;

use crate::config::Config;

type Id = String;
type Passcode = String;


pub fn agent() -> Agent {
    Agent::new()
}

pub fn download_scan(config: &Config, agent: &Agent, database: &mut DbConnection) {
    let mut download_queue: VecDeque<MediaItem> = VecDeque::with_capacity(100);
    let mut wait_time = 1;

    loop {
        if download_queue.is_empty() {
            let items = match media::get_media_items(config, agent) {
                Ok(i) => {
                    wait_time = 1;
                    i
                },
                Err(e) => {
                    error!(
                        "failed to collect media items for download due to error: {}",
                        e
                    );
                    error!("retrying in {} seconds", wait_time);
                    std::thread::sleep(Duration::from_secs(wait_time));
                    wait_time *= 2;
                    if wait_time > 600 {
                        wait_time = 600;
                    }
                    continue;
                }
            };
            download_queue.extend(items);
        }


        if let Some(mut item) = download_queue.pop_front() {
            match database::in_database(database, &item.id) {
                Ok(true) => {
                    debug!("item {} already in database, skipping", item.id);
                    continue;
                }
                Ok(false) => {}
                Err(e) => {
                    error!("failed to check if item {} is in database due to error: {}", item.id, e);
                    continue;
                }
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

                    match database::save_media_item(database, &item) {
                        Ok(_) => info!("saved media item to database"),
                        Err(e) => error!("failed to save media item to database {}", e),
                    }
                }
                (false, _) => {
                    download_queue.push_back(item);
                }
            }
        }
    }
}

pub fn run() {
    //XXX: max file size to attempt a download of, we should create a .txt file of these to be attempted at a later date.
    //XXX: api rate limits, we need to be aware of them

    pretty_env_logger::init();
    let agent = agent();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut database = establish_connection(&database_url).expect("failed to connect to database");
    run_migrations(&mut database).expect("failed to run migrations");
    let config = Config::load(&agent, &mut database).expect("failed to load config");

    download_scan(&config, &agent, &mut database);
}
