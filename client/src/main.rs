mod json_templates;
mod media;
mod webserver;

use std::{collections::VecDeque, path::PathBuf, process::exit, sync::Arc, time::Duration};

use clap::{Parser, Subcommand};
use futures::future::join_all;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::{oneshot, RwLock},
    time::{timeout_at, Instant},
};

use crate::json_templates::MediaItem;

const SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;

type Id = String;
type Passcode = String;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// the location to store downloaded media
    store_path: PathBuf,
    /// whether we have been authenticated
    authenticated: bool,
    /// our id as provided by the remote server
    local_id: Option<String>,
    /// the passcode provided by the remote server
    local_passcode: Option<String>,
    /// the address of the remote server
    webserver_address: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_path: PathBuf::from("./test/"),
            authenticated: false,
            local_id: None,
            local_passcode: None,
            webserver_address: String::from("http://localhost:3000/api/1"),
        }
    }
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("starting");

    let data_path = "./config/data";
    let mut config = match tokio::fs::read(data_path).await {
        Ok(d) => bincode::deserialize(&d).unwrap(),
        Err(e) => {
            warn!("unable to read configuration from disk, falling back to defaults {}", e);
            Config::default()
        }
    };

    if config.local_id.is_none() {
        info!("client is not registered, registering with api...");

        let (id, passcode) = match media::register(&config).await {
            Ok(f) => f,
            Err(e) => {
                error!("unable to register with api, is it running? {}", e);
                exit(1);
            }
        };

        info!("id: {}", id);
        info!("pass: {}", passcode);

        config.local_id = Some(id);
        config.local_passcode = Some(passcode);

        info!("success!");
    }

    info!("Checking authentication....");

    if !config.authenticated {
        info!("client is not authenticated, getting authentication url now.");

        let auth_url = match media::get_auth_url(&config).await {
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
        if let Err(e) = media::await_user_authentication(&config).await {
            error!("authentication failed {}", e);
            exit(1);
        } else {
            config.authenticated = true;
        }

        info!("user authentication successful");
    }

    let config = Arc::new(config);

    //Spawn webserver
    info!("spawning webserver");
    let (webserver_os, mut ws_rx) = oneshot::channel();
    let webserver_config = Arc::clone(&config);
    let webserver = tokio::task::spawn(async move {
        loop {
            select! {
                _ = &mut ws_rx => {
                    info!("Shutting down webserver");
                    break;
                },

            };
        }
    });

    //Spawn media downloader
    info!("spawning media downloader");
    let (media_os, mut md_rx) = oneshot::channel();
    let media_config = Arc::clone(&config);
    let media_downloader = tokio::task::spawn(async move {
        let mut download_queue: VecDeque<MediaItem> = VecDeque::with_capacity(100);

        loop {
            select! {
                biased;

                _ = &mut md_rx => {
                    //XXX:
                    info!("Shutting down media downloader");
                    break;
                }

                //TODO: allow re-calling the endpoint to re-get some of the files

                //XXX: introduce some sort of failure mechanism, if a download is continually failing (*e.g. more than twice)
                // an example is an internet connection which is too slow to download a given file in time.
                // we should also introduce checks for out of space errors, etc
                data = media::download_item(&media_config, &download_queue[0], media_config.store_path.join(format!("{}.{}", &download_queue[0].id, &download_queue[0].filename))), if !download_queue.is_empty() => {
                    if let Err(e) = data {
                        error!("failed to download media item {}", e);
                        //move item to back of queue
                        let item = download_queue.pop_front().unwrap();
                        download_queue.push_back(item);
                        return;
                    }

                    //XXX: we do actually want to the store the information from the api alongside the files

                    download_queue.pop_front();
                }

                new_items = media::get_media_items(&media_config), if download_queue.is_empty() => {
                    match new_items {
                        Ok(new_items) => download_queue.extend(new_items),
                        Err(e) => error!("Unable to collect new media items {}", e),
                    }
                }

                else => {
                    error!("unable to get media items");
                }
            }
        }
    });

    tokio::signal::ctrl_c().await.unwrap();

    info!("Ctrl-C recieved, shutting down");

    media_os
        .send(())
        .expect("able to send shutdown to media_os");
    webserver_os
        .send(())
        .expect("able to send shutdown to webserver_os");

    if let Err(_) = timeout_at(
        Instant::now() + Duration::from_secs(SHUTDOWN_TIMEOUT_SECONDS),
        join_all(vec![webserver, media_downloader]),
    )
    .await
    {
        error!("Failed to shutdown gracefully, force quitting");
        exit(1);
    }

    exit(0);
}
