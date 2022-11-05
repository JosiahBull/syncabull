use crate::{
    database::{self, DbConnection},
    media, Id, Passcode,
};
use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{error::Error, path::PathBuf, process::exit, sync::Mutex};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Temporary location to store media while downloading
    pub temp_path: PathBuf,
    /// The location to store downloaded media
    pub store_path: PathBuf,
    /// Whether we have been authenticated
    pub authenticated: bool,
    /// Our id as provided by the remote server
    pub local_id: Option<Id>,
    /// The passcode provided by the remote server
    pub local_passcode: Option<Passcode>,
    /// The address of the remote server
    pub webserver_address: String,
    /// The preshared key used to authenticate with the remote server
    pub preshared_key: String,
    /// Whether we have completed the initial scan for this account yet
    pub initial_scan_complete: Mutex<bool>,
    /// The maximum number of bytes/sec
    pub max_download_speed: u64,
}

impl Config {
    pub async fn load(
        agent: &Client,
        connection: &mut DbConnection,
    ) -> Result<Config, Box<dyn Error + Send + Sync + 'static>> {
        let mut config = database::load_config(connection)?;

        if config.local_id.is_none() {
            info!("client is not registered, registering with api...");

            let (id, passcode) = match media::register(&config, agent).await {
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

            let auth_url = match media::get_auth_url(&config, agent).await {
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
            if let Err(e) = media::await_user_authentication(&config, agent).await {
                error!("authentication failed {}", e);
                exit(1);
            }
            config.authenticated = true;

            info!("user authentication successful");
        }

        database::save_config(connection, &config).expect("failed to save config");

        Ok(config)
    }

    pub fn save(
        &self,
        connection: &mut DbConnection,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        database::save_config(connection, self)
    }

    pub fn set_initial_scan_complete(
        &self,
        connection: &mut DbConnection,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        *self.initial_scan_complete.lock().unwrap() = true;
        database::save_config(connection, self)
    }

    pub fn initial_scan_complete(&self) -> bool {
        *self.initial_scan_complete.lock().unwrap()
    }
}
