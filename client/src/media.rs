use std::time::Duration;

use crate::{config::Config, Id, Passcode};
use futures_util::TryStreamExt;
use log::{error, trace};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared_libs::json_templates::MediaItem;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    time::Instant,
};
use tokio_util::io::StreamReader;

#[derive(Debug, Serialize, Deserialize)]
struct Register {
    id: Id,
    passcode: Passcode,
}

/// connect to the webserver and register an account, this will return an id and passcode
/// that we will need to peform further actions
pub(crate) async fn register(
    config: &Config,
    agent: &Client,
) -> Result<(Id, Passcode), Box<dyn std::error::Error>> {
    let url = format!("{}/register", config.webserver_address);
    trace!("registering with server at address: {}", &url);
    let res = agent
        .get(&url)
        .header("x-psk", &config.preshared_key)
        .send()
        .await?;

    trace!("got registration response");

    if !res.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to register with api",
        )));
    }

    trace!("parsing registration response");

    let body: Register = res.json().await?;

    trace!("registration response parsed");

    Ok((body.id, body.passcode))
}

/// connect to the server and request a url to authenticate to, for the user to connect their google account
pub(crate) async fn get_auth_url(
    config: &Config,
    agent: &Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("{}/auth_url", config.webserver_address);

    trace!("getting auth url from {}", &url);

    let res = agent
        .get(&url)
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
        .send()
        .await?;

    trace!("got auth url from server");

    if !res.status().is_success() {
        error!("unable to get auth url from api: {}", res.status());
        error!("body: {}", res.text().await?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get auth url",
        )));
    }

    trace!("parsing auth url response");

    Ok(res.text().await?)
}

/// connect to the api and await the user completing authentication
pub(crate) async fn await_user_authentication(
    config: &Config,
    agent: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/is_logged_in", config.webserver_address);

    trace!("awaiting user authentication, from url {}", &url);

    let res = agent
        .get(&url)
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
        .send()
        .await?;

    trace!("got response from server");

    if !res.status().is_success() {
        error!("unable to get auth url: {}", res.status());
        error!("body: {}", res.text().await?);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to await user authentication",
        )));
    }

    trace!("user authenticated");

    Ok(())
}

pub(crate) async fn get_media_items(
    config: &Config,
    agent: &Client,
    reload: bool,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = format!(
        "{}/download?reload={}&max_count=25",
        config.webserver_address, reload
    );

    trace!("getting media items");
    trace!("url: {}", url);

    let res = agent
        .get(&url)
        .basic_auth(
            config.local_id.as_ref().unwrap(),
            config.local_passcode.as_ref(),
        )
        .send()
        .await?;

    trace!("got media items");

    if !res.status().is_success() {
        //print response body
        error!("unable to download media item: {}", res.status());
        error!("body: {}", res.text().await?);

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get media items",
        )));
    }

    trace!("parsing media items");

    let body = res.json().await?;
    Ok(body)
}

async fn download<R>(
    config: &Config,
    mut reader: R,
    mut dest: File,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    R: AsyncReadExt + Unpin,
{
    // copy in chunks, respecting a rate limit if present
    // we limit in 100ms timeframes
    let mut total_bytes = 0;
    let mut time = Instant::now();
    let mut buf = vec![0; 1024.min(config.max_download_speed as usize / 10)];
    loop {
        let bytes = reader.read(&mut buf).await?;
        if bytes == 0 {
            break Ok(());
        }
        dest.write_all(&buf[..bytes]).await?;
        total_bytes += bytes;

        if config.max_download_speed > 0
            && total_bytes / 100 > config.max_download_speed as usize / 100
        {
            // if we are under a second, sleep for the remaining time
            if time.elapsed().as_millis() < 100 {
                let remaining = 100_000 - time.elapsed().as_micros();
                tokio::time::sleep(Duration::from_micros(
                    remaining.try_into().unwrap_or(u64::MAX),
                ))
                .await;
            }
            time = Instant::now();
            total_bytes = 0;
        }
    }
}

pub(crate) async fn download_item(
    config: &Config,
    agent: &Client,
    item: &MediaItem,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    trace!("downloading item: {:?}", item);
    let file_name = &item.id;

    let param = match item.mimeType {
        Some(ref mime_type) if mime_type.contains("video") => "dv",
        _ => "d",
    };

    let url = format!("{}={}", &item.baseUrl, param);

    trace!("downloading item: {} with param: {}", item.id, param);
    trace!("url: {}", &url);

    let res = agent.get(&url).send().await?;

    if !res.status().is_success() {
        //print response body
        error!("unable to download media item: {}", res.status());
        error!("body: {}", res.text().await?);

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to download item",
        )));
    }

    // if config.temp_path doesn't exist - create it
    if !config.temp_path.exists() {
        trace!("creating temp path: {:?}", config.temp_path);
        tokio::fs::create_dir_all(&config.temp_path).await?;
    }

    let tmp_dir = tempfile::Builder::new()
        .prefix("google_photos")
        .tempdir_in(&config.temp_path)?;
    let dest = {
        let fname = tmp_dir.path().join(&file_name);
        trace!("will be located under: '{:?}'", fname);
        File::create(fname).await?
    };

    trace!(
        "writing to temp dir {:?} with final dest {:?}",
        &tmp_dir,
        &dest
    );

    let length = res.content_length();

    let timeout = {
        if let Some(len) = length {
            // for every 1000000 bytes (or max download rate), add 2 seconds
            (len / (1000000.max(config.max_download_speed)) * 2) + 5
        } else {
            // give it 10 minutes to download
            60 * 10
        }
    };

    let reader = res
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let reader = StreamReader::new(reader);

    tokio::time::timeout(Duration::from_secs(timeout), download(config, reader, dest)).await??;

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
