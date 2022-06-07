mod media;
mod json_templates;
mod webserver;

use std::{path::PathBuf, sync::Arc, process::exit, time::Duration, collections::VecDeque};

use clap::{Parser, Subcommand};
use futures::future::join_all;
use log::{info, error};
use serde::{Serialize, Deserialize};
use tokio::{sync::{oneshot, RwLock}, select, time::{timeout_at, Instant}};

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

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {

}


#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let data_path = "./config/data";
    let config = tokio::fs::read(data_path).await.unwrap();
    let mut config: Config = bincode::deserialize(&config).unwrap();

    if config.local_id.is_none() {
        let (id, passcode) = match media::register(&config).await {
            Ok(f) => f,
            Err(e) => {
                error!("unable to register with api, is it running? {}", e);
                exit(1);
            }
        };
        config.local_id = Some(id);
        config.local_passcode = Some(passcode);
    }

    if !config.authenticated {
        let auth_url = match media::get_auth_url(&config).await {
            Ok(f) => f,
            Err(e) => {
                error!("unable to get auth url {}", e);
                exit(1);
            }
        };

        info!("please visit {} and complete authentication within 120 seconds", auth_url);

        // wait for the user to authenticate
        if let Err(e) = media::await_user_authentication(&config).await {
            error!("authentication failed {}", e);
            exit(1);
        } else {
            config.authenticated = true;
        }
    }

    let config = Arc::new(config);

    //Spawn webserver
    let (webserver_os, mut ws_rx) = oneshot::channel();
    let webserver_config = config.clone();
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
    let (media_os, mut md_rx) = oneshot::channel();
    let media_config = config.clone();
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

                //XXX: introduce some sort of failure mechanism, if a download is continually failing
                data = media::download_item(&media_config, &download_queue[0]), if !download_queue.is_empty() => {
                    if let Err(e) = data {
                        error!("failed to download media item");
                        //move item to back of queue
                        let item = download_queue.pop_front().unwrap();
                        download_queue.push_back(item);
                    }

                    // let data = data.unwrap();
                    // tokio::fs::write("", data).await;
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

    media_os.send(());
    webserver_os.send(());

    if let Err(_) = timeout_at(Instant::now() + Duration::from_secs(SHUTDOWN_TIMEOUT_SECONDS), join_all(vec![webserver, media_downloader])).await {
        error!("Failed to shutdown gracefully, force quitting");
        webserver.abort();
        media_downloader.abort();
    }
}
