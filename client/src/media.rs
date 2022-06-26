use crate::{
    json_templates::{MediaItem, Token},
    Config, Id, Passcode,
};
use futures::StreamExt;
use log::info;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

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
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
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
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
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

pub(crate) async fn get_media_items(
    config: &Config,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::new();

    let res = client
        .get(format!(
            "{}/download?reload=false&max_count=20",
            config.webserver_address
        ))
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
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

pub(crate) async fn download_item(
    config: &Config,
    item: &MediaItem,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    //TODO: check for file already existing BEFORE beginning download on it, that way if a download fails we'll know.
    let client = reqwest::Client::new();

    let file_name = format!("{}.....{}", &item.id, &item.filename);

    let param = match item.mimeType {
        Some(ref mime_type) if mime_type.contains("video") => "dv",
        _ => "d"
    };

    let url = format!("{}={}", &item.baseUrl, param);

    let res = client.get(url).send().await?;

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to download item",
        )));
    }

    let tmp_dir = tempfile::Builder::new().prefix("syncabull-").tempdir()?;
    let mut dest = {
        let fname = tmp_dir.path().join(&file_name);
        info!("will be located under: '{:?}'", fname);
        tokio::fs::File::create(fname).await?
    };

    let mut content = res.bytes_stream();

    while let Some(bytes) = content.next().await {
        match bytes {
            Ok(mut bytes) => {
                dest.write_buf(&mut bytes).await?;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    //TODO: set this up to attempt a rename, then fallback to copy.
    tokio::fs::copy(
        tmp_dir.path().join(&file_name),
        config.store_path.join(&file_name),
    )
    .await?;

    Ok(())
}
