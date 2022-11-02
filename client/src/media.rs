use crate::{config::Config, Id, Passcode};
use log::{error, info};
use serde::{Deserialize, Serialize};
use shared_libs::json_templates::MediaItem;
use std::{fs::File, time::Duration};
use ureq::Agent;

#[derive(Debug, Serialize, Deserialize)]
struct Register {
    id: Id,
    passcode: Passcode,
}

/// connect to the webserver and register an account, this will return an id and passcode
/// that we will need to peform further actions
pub(crate) fn register(
    config: &Config,
    agent: &Agent,
) -> Result<(Id, Passcode), Box<dyn std::error::Error>> {
    let res = agent
        .get(&format!("{}/register", config.webserver_address))
        .set("x-psk", &config.preshared_key)
        .call()?;

    if !(res.status() >= 200 && res.status() < 300) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to register with api",
        )));
    }

    let body: Register = res.into_json()?;
    Ok((body.id, body.passcode))
}

/// connect to the server and request a url to authenticate to, for the user to connect their google account
pub(crate) fn get_auth_url(
    config: &Config,
    agent: &Agent,
) -> Result<String, Box<dyn std::error::Error>> {
    let res = agent
        .get(&format!("{}/auth_url", config.webserver_address))
        .set(
            "authorization",
            &format!(
                "basic {}",
                &base64::encode(format!(
                    "{}:{}",
                    config.local_id.as_ref().unwrap(),
                    config.local_passcode.as_ref().unwrap()
                ))
            ),
        )
        .call()?;

    if !(res.status() >= 200 && res.status() < 300) {
        error!("unable to get auth url from api: {}", res.status());
        error!("body: {}", res.into_string()?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get auth url",
        )));
    }

    Ok(res.into_string()?)
}

/// connect to the api and await the user completing authentication
pub(crate) fn await_user_authentication(
    config: &Config,
    agent: &Agent,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = agent
        .get(&format!("{}/is_logged_in", config.webserver_address))
        .set(
            "authorization",
            &format!(
                "basic {}",
                &base64::encode(format!(
                    "{}:{}",
                    config.local_id.as_ref().unwrap(),
                    config.local_passcode.as_ref().unwrap()
                ))
            ),
        )
        .timeout(Duration::from_secs(120))
        .call()?;

    if !(res.status() >= 200 && res.status() < 300) {
        error!("unable to get auth url: {}", res.status());
        error!("body: {}", res.into_string()?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to await user authentication",
        )));
    }

    Ok(())
}

pub(crate) fn get_media_items(
    config: &Config,
    agent: &Agent,
    reload: bool,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error>> {
    let res = agent
        .get(&format!(
            "{}/download?reload={}&max_count=20",
            config.webserver_address, reload
        ))
        .set(
            "authorization",
            &format!(
                "basic {}",
                &base64::encode(format!(
                    "{}:{}",
                    config.local_id.as_ref().unwrap(),
                    config.local_passcode.as_ref().unwrap()
                ))
            ),
        )
        .call()?;

    if !(res.status() >= 200 && res.status() < 300) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get media items",
        )));
    }

    let body = res.into_json()?;
    Ok(body)
}

pub(crate) fn download_item(
    config: &Config,
    agent: &Agent,
    item: &MediaItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = &item.id;

    let param = match item.mimeType {
        Some(ref mime_type) if mime_type.contains("video") => "dv",
        _ => "d",
    };

    let url = format!("{}={}", &item.baseUrl, param);

    let res = agent.get(&url).call()?;

    if !(res.status() >= 200 && res.status() < 300) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to download item",
        )));
    }

    let tmp_dir = tempfile::Builder::new().prefix("syncabull-").tempdir()?;
    let mut dest = {
        let fname = tmp_dir.path().join(&file_name);
        info!("will be located under: '{:?}'", fname);
        File::create(fname)?
    };

    let mut reader = res.into_reader();

    std::io::copy(&mut reader, &mut dest)?;

    // if dest does not exist, create it
    if !config.store_path.exists() {
        std::fs::create_dir_all(&config.store_path)?;
    }

    // Attempt to move the file, fallback to copying if it fails
    if let Err(e) = std::fs::rename(
        tmp_dir.path().join(&file_name),
        config.store_path.join(&file_name),
    ) {
        error!("unable to rename file: {}", e);
        std::fs::copy(
            tmp_dir.path().join(&file_name),
            config.store_path.join(&file_name),
        )?;
    }

    Ok(())
}
