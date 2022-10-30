use std::path::PathBuf;
use log::warn;
use serde::{Deserialize, Serialize};
use crate::{Id, Passcode};

const DEFAULT_CONFIG_PATH: &str = ".config.toml";

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
    pub fn from_file() -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::read_to_string(DEFAULT_CONFIG_PATH)?;
        let config: Config = toml::from_str(&file)?;
        Ok(config)
    }

    pub fn to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file = toml::to_string(&self)?;
        std::fs::write(DEFAULT_CONFIG_PATH, file)?;
        Ok(())
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let mut res = match Self::from_file() {
            Ok(f) => f,
            Err(e) => {
                warn!("failed to load config from file, using default config {}", e);

                if PathBuf::from(DEFAULT_CONFIG_PATH).exists() {
                    let mut backup_path = PathBuf::from(DEFAULT_CONFIG_PATH);
                    backup_path.set_extension("bak");
                    std::fs::copy(DEFAULT_CONFIG_PATH, backup_path)?;
                }


                Config {
                    store_path: PathBuf::from("./media"),
                    authenticated: false,
                    local_id: None,
                    local_passcode: None,
                    webserver_address: "http://127.0.0.1:3000".to_string(),
                    preshared_key: "hunter42".to_string(),
                }
            }
        };

        // if any env vars are set, use them to override the config
        if let Ok(webserver_address) = std::env::var("WEB_SERVER_ADDRESS") {
            res.webserver_address = webserver_address;
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "WEB_SERVER_ADDRESS not set",
            )));
        }

        if let Ok(store_path) = std::env::var("STORE_PATH") {
            res.store_path = PathBuf::from(store_path);
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "STORE_PATH not set",
            )));
        }

        if let Ok(psk) = std::env::var("PRESHARED_KEY") {
            res.preshared_key = psk;
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "PRESHARED_KEY not set",
            )));
        }

        Ok(res)
    }
}

