use std::{collections::HashMap, error::Error, path::PathBuf, sync::Mutex};

use diesel::{sqlite::Sqlite, Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use shared_libs::json_templates::MediaItem;

use crate::config::Config;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type DbConnection = diesel::SqliteConnection;
pub type DB = Sqlite;

pub fn run_migrations(
    connection: &mut impl MigrationHarness<DB>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

pub fn establish_connection(
    database_url: &str,
) -> Result<DbConnection, Box<dyn Error + Send + Sync + 'static>> {
    Ok(DbConnection::establish(database_url)?)
}

// media (id) {
//     id -> Text,
//     description -> Nullable<Text>,
//     product_url -> Text,
//     base_url -> Text,
//     mime_type -> Nullable<Text>,
//     filename -> Text,
//     creation_time -> Nullable<Text>,
//     width -> Nullable<Integer>,
//     height -> Nullable<Integer>,
//     camera_make -> Nullable<Text>,
//     camera_model -> Nullable<Text>,
//     focal_length -> Nullable<Float>,
//     aperture -> Nullable<Float>,
//     iso_equivalent -> Nullable<Integer>,
//     exposure_time -> Nullable<Text>,
//     fps -> Nullable<Float>,
//     processing_status -> Nullable<Text>,
//     profile_picture_url -> Nullable<Text>,
//     display_name -> Nullable<Text>,
//     download_attempts -> Integer,
//     download_success -> Bool,
//     download_timestamp -> Text,
// }

pub fn save_media_item(
    connection: &mut DbConnection,
    media_item: &MediaItem,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    use crate::schema::media::dsl::*;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let records = (
        id.eq(&media_item.id),
        description.eq(&media_item.description),
        product_url.eq(&media_item.productUrl),
        base_url.eq(&media_item.baseUrl),
        mime_type.eq(&media_item.mimeType),
        filename.eq(&media_item.filename),
        download_attempts.eq(media_item.download_attempts as i32),
        download_success.eq(&media_item.download_success),
        download_timestamp.eq(&now),
        // mediaMetadata might be null
        creation_time.eq({
            media_item
                .mediaMetadata
                .as_ref()
                .map(|media_metadata| &media_metadata.creationTime)
        }),
        width.eq({
            media_item
                .mediaMetadata
                .as_ref()
                .map(|media_metadata| &media_metadata.width)
        }),
        height.eq({
            media_item
                .mediaMetadata
                .as_ref()
                .map(|media_metadata| &media_metadata.height)
        }),
        camera_make.eq({
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    // if photo is some -> get photo value, otherwise get video value
                    // ensure we only create an Option<String> and not an Option<Option<String>>
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.cameraMake.as_ref())
                        .or_else(|| {
                            media_metadata
                                .video
                                .as_ref()
                                .and_then(|video| video.cameraMake.as_ref())
                        })
                })
        }),
        camera_model.eq({
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    // if photo is some -> get photo value, otherwise get video value
                    // ensure we only create an Option<String> and not an Option<Option<String>>
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.cameraModel.as_ref())
                        .or_else(|| {
                            media_metadata
                                .video
                                .as_ref()
                                .and_then(|video| video.cameraModel.as_ref())
                        })
                })
        }),
        focal_length.eq({
            // value only from photo if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.focalLength.as_ref().map(|f| *f as f32))
                })
        }),
        aperture.eq({
            // value only from photo if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.apertureFNumber.as_ref().map(|f| *f as f32))
                })
        }),
        iso_equivalent.eq({
            // value only from photo if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.isoEquivalent.as_ref().map(|f| *f as i32))
                })
        }),
        exposure_time.eq({
            // value only from photo if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .photo
                        .as_ref()
                        .and_then(|photo| photo.exposureTime.as_ref())
                })
        }),
        fps.eq({
            // value only from video if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .video
                        .as_ref()
                        .and_then(|video| video.fps.as_ref().map(|f| *f as f32))
                })
        }),
        processing_status.eq({
            // value only from video if present
            media_item
                .mediaMetadata
                .as_ref()
                .and_then(|media_metadata| {
                    media_metadata
                        .video
                        .as_ref()
                        .and_then(|video| video.status.as_ref().map(|s| s.to_string()))
                })
        }),
        profile_picture_url.eq({
            //only present if contributor is present
            media_item
                .contributorInfo
                .as_ref()
                .map(|contributor_info| &contributor_info.profilePictureBaseUrl)
        }),
        display_name.eq({
            //only present if contributor is present
            media_item
                .contributorInfo
                .as_ref()
                .map(|contributor_info| &contributor_info.displayName)
        }),
    );

    // insert with each field specified manually
    let r: String = diesel::insert_into(media)
        .values(records.clone())
        // on conflict, replace all fields
        .on_conflict(id)
        .do_update()
        .set(records)
        // return the id of the inserted row
        .returning(id)
        .load_iter(connection)?
        .next()
        .unwrap()?;
    Ok(r)
}

/// check if a media item is present in the database, searching by id
pub fn in_database(
    connection: &mut DbConnection,
    search_id: &str,
) -> Result<bool, Box<dyn Error + Send + Sync + 'static>> {
    use crate::schema::media::dsl::*;
    let r: Vec<String> = media.select(id).filter(id.eq(search_id)).load(connection)?;
    Ok(!r.is_empty())
}

pub fn load_config(
    connection: &mut DbConnection,
) -> Result<Config, Box<dyn Error + Send + Sync + 'static>> {
    // load every row from the config table into a hashmap of key-value pairs
    use crate::schema::config::dsl::*;
    let r: HashMap<String, String> = config
        .select((key, value))
        .load::<(String, String)>(connection)?
        .into_iter()
        .collect();

    // if a key exists in env, load that over trying to load from the database
    // otherwise pull it from the database and pass that into the config
    let store_path = match std::env::var("STORE_PATH") {
        Ok(s) => PathBuf::from(s),
        Err(_) => PathBuf::from(r.get("store_path").expect("store_path not found in config")),
    };

    let temp_path = match std::env::var("TEMP_PATH") {
        Ok(s) => PathBuf::from(s),
        Err(_) => PathBuf::from(r.get("temp_path").expect("temp_path not found in config")),
    };

    // if authenticated not present == false
    let authenticated = match std::env::var("AUTHENTICATED") {
        Ok(s) => s == "true",
        Err(_) => r.get("authenticated").unwrap_or(&String::from("false")) == "true",
    };

    let local_id = match std::env::var("LOCAL_ID") {
        Ok(s) => Some(s),
        Err(_) => r.get("local_id").map(|s| s.to_string()),
    };

    let local_passcode = match std::env::var("LOCAL_PASSCODE") {
        Ok(s) => Some(s),
        Err(_) => r.get("local_passcode").map(|s| s.to_string()),
    };

    let webserver_address = match std::env::var("WEBSERVER_ADDRESS") {
        Ok(s) => s,
        Err(_) => r.get("webserver_address").unwrap().to_string(),
    };

    let preshared_key = match std::env::var("PRESHARED_KEY") {
        Ok(s) => s,
        Err(_) => r.get("preshared_key").unwrap().to_string(),
    };

    let initial_scan_complete = match std::env::var("INITIAL_SCAN_COMPLETE") {
        Ok(s) => s == "true",
        Err(_) => {
            r.get("initial_scan_complete")
                .unwrap_or(&String::from("false"))
                == "true"
        }
    };
    let initial_scan_complete = Mutex::new(initial_scan_complete);

    let max_download_speed = match std::env::var("MAX_DOWNLOAD_SPEED") {
        Ok(s) => s.parse::<u64>().unwrap(),
        Err(_) => r
            .get("max_download_speed")
            .unwrap_or(&String::from("0"))
            .parse::<u64>()
            .unwrap(),
    };

    Ok(Config {
        store_path,
        authenticated,
        local_id,
        local_passcode,
        webserver_address,
        preshared_key,
        initial_scan_complete,
        temp_path,
        max_download_speed,
    })
}

pub fn save_config(
    connection: &mut DbConnection,
    save_config: &Config,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    use crate::schema::config::dsl::*;

    // convert the config struct into a hashmap of key-value pairs
    let authenticated = save_config.authenticated.to_string();
    let initial_scan_complete = save_config
        .initial_scan_complete
        .lock()
        .unwrap()
        .to_string();
    let mut r = vec![
        ("store_path", save_config.store_path.to_str().unwrap()),
        ("authenticated", &authenticated),
        ("webserver_address", &save_config.webserver_address),
        ("preshared_key", &save_config.preshared_key),
        ("initial_scan_complete", &initial_scan_complete),
    ];

    if let Some(local_id) = &save_config.local_id {
        r.push(("local_id", local_id));
    }

    if let Some(local_passcode) = &save_config.local_passcode {
        r.push(("local_passcode", local_passcode));
    }

    // insert with each field specified manually
    for (d_key, d_value) in r.into_iter() {
        diesel::insert_into(config)
            .values((key.eq(d_key), value.eq(d_value)))
            // on conflict, replace all fields
            .on_conflict(key)
            .do_update()
            .set(value.eq(d_value))
            .execute(connection)?;
    }

    Ok(())
}
