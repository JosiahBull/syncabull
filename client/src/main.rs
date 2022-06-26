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
    sync::{
        oneshot::{self, error::TryRecvError},
        RwLock,
    },
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
            local_id: Some("2YhkLFWhU05cyMscoEIGrHlwNvybDkil".to_string()),
            local_passcode: Some("ZWJGjqIObF6iddfmWgoQgHJtY0hljMYw".to_string()),
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
    //XXX: Config options for logging, etc.

    //XXX: max retries to download a file?
    //XXX: max file size to attempt a download of, we should create a .txt file of these to be attempted at a later date.
    //XXX: perhaps we should setup the api to allow single-file downloads for SUPER large files?
    //XXX: config option for file storage location
    //XXX: api rate limits, we need to be aware of them
    //XXX: shutdown timeouts config

    //XXX: prevent logging from reqwest etc except when absolutely neeeded.
    //XXX: switch to fern logging?
    pretty_env_logger::init();

    info!("starting");

    let data_path = "./config/data";
    let mut config = match tokio::fs::read(data_path).await {
        Ok(d) => bincode::deserialize(&d).unwrap(),
        Err(e) => {
            warn!(
                "unable to read configuration from disk, falling back to defaults {}",
                e
            );
            Config::default()
        }
    };

    if config.local_id.is_none() {
        info!("client is not registered, registering with api...");

        let (id, passcode) = match media::register(&config).await {
            Ok(f) => f,
            Err(e) => {
                error!("unable to register with api {}", e);
                exit(1);
            }
        };

        // info!("id: {}", id);
        // info!("pass: {}", passcode);

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
                //TODO
            };
        }
    });

    //Spawn media downloader
    info!("spawning media downloader");
    let (media_os, mut md_rx) = oneshot::channel();
    let media_config = Arc::clone(&config);
    let media_downloader = tokio::task::spawn(async move {
        let mut download_queue: VecDeque<MediaItem> = VecDeque::with_capacity(100);

        //TODO: handle failed media downloads, track fail count, and last request time. If we're > 1hr, remake the request to the api
        //TODO: if the api is unresponsive, we should practice expoential backoff until it responds.

        loop {
            match md_rx.try_recv() {
                Ok(_) => {
                    info!("Shutting down media downloader");
                    break;
                }
                Err(TryRecvError::Closed) => {
                    error!("media downloader channel closed, shutting down downloader");
                    break;
                }
                _ => {}
            }

            if download_queue.is_empty() {
                let items = match media::get_media_items(&media_config).await {
                    Ok(i) => i,
                    Err(e) => {
                        error!(
                            "failed to collect media items for download due to error: {}",
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
                download_queue.extend(items);
            }

            if let Some(item) = download_queue.pop_front() {
                //allow multiple downloads to occur at once, make it configurable.
                info!("downloading {}", item.baseUrl);
                match media::download_item(
                    &media_config,
                    &item,
                )
                .await
                {
                    Ok(_) => {
                        info!("download successful");
                    }
                    Err(e) => {
                        error!("failed to download media: {}", e);
                        // download_queue.push_back(item);
                    }
                }
            }
        }
    });

    tokio::signal::ctrl_c().await.unwrap();

    info!("Ctrl-C recieved, shutting down");

    if let Err(_) = media_os.send(()) {
        error!("unable to send shutdown signal to media downloader, it may have crashed");
    }

    if let Err(_) = webserver_os.send(()) {
        error!("unable to send shutdown signal to webserver, it may have crashed");
    }

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
