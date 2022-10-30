use std::{path::PathBuf, error::Error, process::exit};
use log::{info, error};
use serde::{Deserialize, Serialize};
use ureq::Agent;
use crate::{Id, Passcode, database::{DbConnection, self}, media};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
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
}

impl Config {
    pub fn load(agent: &Agent, connection: &mut DbConnection) -> Result<Config, Box<dyn Error>> {
        let mut config = database::load_config(connection)?;

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

        database::save_config(connection, &config).expect("failed to save config");

        Ok(config)
    }
}

