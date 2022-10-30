pub mod json_templates;
pub mod media;
pub mod cli;
pub mod config;

use std::{collections::VecDeque, process::exit, time::Duration};

use log::{error, info};
use ureq::Agent;

use crate::{json_templates::MediaItem, config::Config};

type Id = String;
type Passcode = String;


pub fn agent() -> Agent {
    Agent::new()
}

pub fn config(agent: &Agent) -> Config {
    let mut config = Config::load().expect("failed to load config");
    
    if config.local_id.is_none() {
        info!("client is not registered, registering with api...");

        let (id, passcode) = match media::register(&config, agent) {
            Ok(f) => f,
            Err(e) => {
                error!("unable to register with api {}", e);
                exit(1);
            }
        };

        config.local_id = Some(id);
        config.local_passcode = Some(passcode);

        info!("success!");
    }

    info!("Checking authentication....");
    if !config.authenticated {
        info!("client is not authenticated, getting authentication url now.");

        let auth_url = match media::get_auth_url(&config, agent) {
            Ok(f) => f,
            Err(e) => {
                error!("unable to get auth url {}", e);
                exit(1);
            }
        };

        info!(
            "please visit {} and complete authentication within 120 seconds",
            auth_url
        );

        // wait for the user to authenticate
        if let Err(e) = media::await_user_authentication(&config, agent) {
            error!("authentication failed {}", e);
            exit(1);
        }
        config.authenticated = true;

        info!("user authentication successful");
    }

    config
}

pub fn download_scan(config: &Config, agent: &Agent) {
    //TODO: handle failed media downloads, track fail count, and last request time. If we're > 1hr, remake the request to the api
    //TODO: if the api is unresponsive, we should practice exponential backoff until it responds.

    let mut download_queue: VecDeque<MediaItem> = VecDeque::with_capacity(100);

    loop {
        if download_queue.is_empty() {
            let items = match media::get_media_items(config, agent) {
                Ok(i) => i,
                Err(e) => {
                    error!(
                        "failed to collect media items for download due to error: {}",
                        e
                    );
                    std::thread::sleep(Duration::from_secs(10));
                    continue;
                }
            };
            download_queue.extend(items);
        }

        if let Some(item) = download_queue.pop_front() {
            //allow multiple downloads to occur at once, make it configurable.
            info!("downloading {}", item.baseUrl);
            match media::download_item(config, agent, &item) {
                Ok(_) => {
                    info!("download successful");
                }
                Err(e) => {
                    error!("failed to download media: {}", e);
                    download_queue.push_back(item);
                }
            }
        }
    }
}

pub fn run() {
    //XXX: max retries to download a file?
    //XXX: max file size to attempt a download of, we should create a .txt file of these to be attempted at a later date.
    //XXX: perhaps we should setup the api to allow single-file downloads for SUPER large files?
    //XXX: config option for file storage location
    //XXX: api rate limits, we need to be aware of them

    pretty_env_logger::init();
    let agent = agent();
    let config = config(&agent);
    download_scan(&config, &agent);
}
