mod database;
mod json_templates;
mod photoscanner;
mod webserver;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::RwLock;
use webserver::WebServer;

pub type AuthToken = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleAuth {
    /// A bearer token used to access the google api
    pub token: String,
    /// Time when the above bearer token expires, in seconds since unix epoch
    pub token_expiry_sec_epoch: u64,
    /// Token used to refresh the bearer token with the google api
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserState {
    /// The id of this user
    pub user_id: String,
    /// users authentication token they should use to connect to the api
    pub auth_token: AuthToken,
    /// The users google bearer token
    pub google_token: GoogleAuth,
    /// If the api has not completed a scan of the google api, this is the token of the next page to be scanned
    pub next_token: Option<String>,
    /// Whether the api has completed the initial scan of this users photos
    pub initial_scan_completed: bool,
    /// The last time the api checked the users photos in second since epoch
    pub last_checked: u64,
    /// The number of photos scanned by the ai so far
    pub photos_scanned: u64,
    /// seconds since epoch when user profile was last fetched
    pub profile_fetch_epoch: u64,
    /// User email address
    pub email: String,
    /// User profile picture
    pub profile_picture: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OneTimeCode {
    /// The email this code is assigned to
    pub email: String,
    /// The time the code expires in seconds since unix epoch
    pub expiry_sec_epoch: u64,
}

#[derive(Default, Debug)]
pub struct AppState {
    /// registered users
    users: RwLock<HashMap<String, UserState>>,
    /// one time auth codes
    otcs: RwLock<HashMap<String, OneTimeCode>>,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::default());
    //TODO: database integration

    let webserver_state = state.clone();
    let webserver_handle = tokio::task::spawn(async move {
        WebServer::builder()
            .google_client_id(env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID is set"))
            .google_client_secret(
                env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET is set"),
            )
            .domain(env::var("BROWSER_BASE_URL").expect("BROWSER_BASE_URL is set"))
            .token_url("https://www.googleapis.com/oauth2/v3/token")
            .auth_url("https://accounts.google.com/o/oauth2/v2/auth")
            .state(webserver_state)
            .build()
            .run()
            .await;
    });

    join_all([webserver_handle]).await;
}
