use crate::{config::Config, Id, Passcode};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use shared_libs::json_templates::MediaItem;
use std::{
    fs::File,
    io::Write,
    time::{Duration, Instant},
};
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
    let url = format!("{}/register", config.webserver_address);
    trace!("registering with server at address: {}", &url);
    let res = agent.get(&url).set("x-psk", &config.preshared_key).call();

    trace!("got registration response");

    let res = match res {
        Ok(r) => r,
        Err(e) => {
            if let Some(r) = e.into_response() {
                trace!("parsing error response");
                r
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to get media items",
                )));
            }
        }
    };

    if !(res.status() >= 200 && res.status() < 300) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to register with api",
        )));
    }

    trace!("parsing registration response");

    let body: Register = res.into_json()?;

    trace!("registration response parsed");

    Ok((body.id, body.passcode))
}

/// connect to the server and request a url to authenticate to, for the user to connect their google account
pub(crate) fn get_auth_url(
    config: &Config,
    agent: &Agent,
) -> Result<String, Box<dyn std::error::Error>> {
    trace!("getting auth url");

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
        .call();

    trace!("got auth url from server");

    let res = match res {
        Ok(r) => r,
        Err(e) => {
            if let Some(r) = e.into_response() {
                trace!("parsing error response");
                r
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to get media items",
                )));
            }
        }
    };

    if !(res.status() >= 200 && res.status() < 300) {
        error!("unable to get auth url from api: {}", res.status());
        error!("body: {}", res.into_string()?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get auth url",
        )));
    }

    trace!("parsing auth url response");

    Ok(res.into_string()?)
}

/// connect to the api and await the user completing authentication
pub(crate) fn await_user_authentication(
    config: &Config,
    agent: &Agent,
) -> Result<(), Box<dyn std::error::Error>> {
    trace!("awaiting user authentication");

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
        .call();

    trace!("got response from server");

    let res = match res {
        Ok(r) => r,
        Err(e) => {
            if let Some(r) = e.into_response() {
                trace!("parsing error response");
                r
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to get media items",
                )));
            }
        }
    };

    if !(res.status() >= 200 && res.status() < 300) {
        error!("unable to get auth url: {}", res.status());
        error!("body: {}", res.into_string()?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to await user authentication",
        )));
    }

    trace!("user authenticated");

    Ok(())
}

pub(crate) fn get_media_items(
    config: &Config,
    agent: &Agent,
    reload: bool,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/download?reload={}&max_count=25",
        config.webserver_address, reload
    );

    trace!("getting media items");
    trace!("url: {}", url);

    let res = agent
        .get(&url)
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
        .call();

    trace!("got media items");

    let res = match res {
        Ok(r) => r,
        Err(e) => {
            if let Some(r) = e.into_response() {
                trace!("parsing error response");
                r
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to get media items",
                )));
            }
        }
    };

    if !(res.status() >= 200 && res.status() < 300) {
        //print response body
        error!("unable to download media item: {}", res.status());
        error!("body: {}", res.into_string()?);

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get media items",
        )));
    }

    trace!("parsing media items");

    let body = res.into_json()?;

    trace!("parsed media items");

    Ok(body)
}

pub(crate) fn download_item(
    config: &Config,
    agent: &Agent,
    item: &MediaItem,
) -> Result<(), Box<dyn std::error::Error>> {
    trace!("downloading item: {:?}", item);
    let file_name = &item.id;

    let param = match item.mimeType {
        Some(ref mime_type) if mime_type.contains("video") => "dv",
        _ => "d",
    };

    let url = format!("{}={}", &item.baseUrl, param);

    trace!("downloading item: {} with param: {}", item.id, param);
    trace!("url: {}", &url);

    let res = agent.get(&url).call();

    let res = match res {
        Ok(r) => r,
        Err(e) => {
            if let Some(r) = e.into_response() {
                trace!("parsing error response");
                r
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to get media items",
                )));
            }
        }
    };

    if !(res.status() >= 200 && res.status() < 300) {
        //print response body
        error!("unable to download media item: {}", res.status());
        error!("body: {}", res.into_string()?);

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to download item",
        )));
    }

    // if config.temp_path doesn't exist - create it
    if !config.temp_path.exists() {
        trace!("creating temp path: {:?}", config.temp_path);
        std::fs::create_dir_all(&config.temp_path)?;
    }

    let tmp_dir = tempfile::Builder::new()
        .prefix("google_photos")
        .tempdir_in(&config.temp_path)?;
    let mut dest = {
        let fname = tmp_dir.path().join(&file_name);
        info!("will be located under: '{:?}'", fname);
        File::create(fname)?
    };

    trace!(
        "writing to temp dir {:?} with final dest {:?}",
        &tmp_dir,
        &dest
    );

    let mut reader = res.into_reader();

    // copy in chunks, respecting a rate limit if present
    // we limit in 100ms timeframes, with a max chunk size of 512 bytes
    let mut total_bytes = 0;
    let mut time = Instant::now();
    let mut buf = [0; 512];
    loop {
        let bytes = reader.read(&mut buf)?;
        if bytes == 0 {
            break;
        }
        dest.write_all(&buf[..bytes])?;
        total_bytes += bytes;

        if config.max_download_speed > 0
            && total_bytes / 100 > config.max_download_speed as usize / 100
        {
            // if we are under a second, sleep for the remaining time
            if time.elapsed().as_millis() < 100 {
                let remaining = 100_000 - time.elapsed().as_micros();
                std::thread::sleep(Duration::from_micros(
                    remaining.try_into().unwrap_or(u64::MAX),
                ));
            }
            time = Instant::now();
            total_bytes = 0;
        }
    }

    trace!("moving to final destination");

    // if dest does not exist, create it
    if !config.store_path.exists() {
        trace!("creating store path: {:?}", &config.store_path);
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
        std::fs::remove_file(tmp_dir.path().join(&file_name))?;
    }

    trace!("removing temp dir");
    tmp_dir.close()?;

    Ok(())
}
