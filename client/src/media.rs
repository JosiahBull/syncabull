use std::time::Duration;

use serde::{Serialize, Deserialize};

use crate::{Config, Passcode, Id, json_templates::MediaItem};

#[derive(Debug, Serialize, Deserialize)]
struct Register {
    id: Id,
    passcode: Passcode,
}

/// connect to the webserver and register an account, this will return an id and passcode
/// that we will need to peform further actions
pub (crate) async fn register(config: &Config) -> Result<(Id, Passcode), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let res = client.get(format!("{}/register", config.webserver_address))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unable to register with api")));
    }

    let body: Register = res.json().await?;
    Ok((body.id, body.passcode))
}

/// connect to the server and request a url to authenticate to, for the user to connect their google account
pub (crate) async fn get_auth_url(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let res = client.get(format!("{}/auth_url", config.webserver_address))
        // .header(key, value)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unable to get auth url")));
    }

    let body = res.text().await?;
    Ok(body)
}

/// connect to the api and await the user completing authentication
pub (crate) async fn await_user_authentication(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let res = client.get(format!("{}/await_auth", config.webserver_address))
        // .header(key, value)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unable to await user authentication")));
    }

    Ok(())
}

pub (crate) async fn download_item(config: &Config, item: &MediaItem) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let res = client.get(&item.baseUrl)
        // .header(key, value)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unable to download item")));
    }

    Ok(res.bytes().await?.to_vec())
}

pub (crate) async fn get_media_items(config: &Config) -> Result<Vec<MediaItem>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let res = client.get(format!("{}/media", config.webserver_address))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unable to get media items")));
    }

    let body = res.json().await?;
    Ok(body)
}