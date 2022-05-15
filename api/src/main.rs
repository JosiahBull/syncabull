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
pub type UserId = [u8; 8]; //8-byte userid

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
    pub user_id: UserId,
    /// Email address of this user
    pub email: String,
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
}

#[derive(Default, Debug)]
pub struct AppState {
    users: RwLock<HashMap<UserId, UserState>>,
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
            .redirect_url("http://localhost:8080/api/1/auth")
            .token_url("https://www.googleapis.com/oauth2/v3/token")
            .auth_url("https://accounts.google.com/o/oauth2/v2/auth")
            .state(webserver_state)
            .build()
            .run()
            .await;
    });

    let refresh_service = tokio::task::spawn(async move {
        //TODO: this task will handle refreshing google api tokens for users when they are close to expiring
    });

    join_all([webserver_handle]).await;
}
