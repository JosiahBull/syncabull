use std::{time::Duration, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{json_templates::{MediaItem, Token}, Config, Id, Passcode};

#[derive(Debug, Serialize, Deserialize)]
struct Register {
    id: Id,
    passcode: Passcode,
}

/// connect to the webserver and register an account, this will return an id and passcode
/// that we will need to peform further actions
pub(crate) async fn register(
    config: &Config,
) -> Result<(Id, Passcode), Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/register", config.webserver_address))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to register with api",
        )));
    }

    let body: Register = res.json().await?;
    Ok((body.id, body.passcode))
}

/// connect to the server and request a url to authenticate to, for the user to connect their google account
pub(crate) async fn get_auth_url(
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/auth_url", config.webserver_address))
        .basic_auth(config.local_id.as_ref().unwrap(), config.local_passcode.as_ref())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get auth url",
        )));
    }

    let body = res.text().await?;
    Ok(body)
}

/// connect to the api and await the user completing authentication
pub(crate) async fn await_user_authentication(
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/is_logged_in", config.webserver_address))
        .basic_auth(config.local_id.as_ref().unwrap(), config.local_passcode.as_ref())
        .timeout(Duration::from_secs(120))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to await user authentication",
        )));
    }

    Ok(())
}

pub(crate) async fn download_item(
    config: &Config,
    item: &MediaItem,
    save_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(&item.baseUrl)
        .basic_auth(config.local_id.as_ref().unwrap(), config.local_passcode.as_ref())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to download item",
        )));
    }

    //HACK: this reads the entire file into memory, we want to stream it ideally
    tokio::fs::write(save_path, res.bytes().await.unwrap()).await.unwrap();

    Ok(())
}

pub(crate) async fn get_media_items(
    config: &Config,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/media", config.webserver_address))
        .basic_auth(config.local_id.as_ref().unwrap(), config.local_passcode.as_ref())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get media items",
        )));
    }

    let body = res.json().await?;
    Ok(body)
}
