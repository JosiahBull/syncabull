mod auth;
mod photoscanner;
mod webserver;

use auth::Token;
use photoscanner::PhotoScanner;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use webserver::WebServer;

use futures::future::join_all;
use handlebars::Handlebars;
use std::{
    collections::HashMap,
    env,
    path::{self, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

const STORE_PATH: &str = "store.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleAuth {
    /// A bearer token used to access the google api
    pub token: String,
    /// Time when the above bearer token expires, in seconds since unix epoch
    pub token_expiry_sec_epoch: SystemTime,
    /// Token used to refresh the bearer token with the google api
    pub refresh_token: String,
}

impl GoogleAuth {
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.token_expiry_sec_epoch
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserData {
    pub hashed_passcode: String,
    pub tokens: Vec<String>,
    pub google_auth: Option<GoogleAuth>,
    /// If the initial scan has been completed
    pub initial_scan_complete: bool,
    /// The token for the next page
    pub next_token: Option<String>,
    /// The previous token that was used, so the user can repeat a request if required
    pub prev_token: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    users: HashMap<String, UserData>,
    auth_keys: HashMap<String, Token>,
    unclaimed_auth_tokens: HashMap<String, GoogleAuth>,
}

impl AppState {
    pub async fn from_disk(path: PathBuf) -> Self {
        let data = tokio::fs::read(&path).await.unwrap();
        let res: Self = serde_json::from_slice(&data).unwrap();
        res
    }

    pub async fn to_disk(&self, path: PathBuf) {
        let data = serde_json::to_vec(&self).unwrap();
        tokio::fs::write(&path, data).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();


    println!("starting api");
    println!("loading state");
    let state = match tokio::fs::metadata(path::Path::new(STORE_PATH)).await {
        Ok(_) => AppState::from_disk(path::PathBuf::from(STORE_PATH)).await,
        Err(_) => AppState::default(),
    };

    let state = Arc::new(RwLock::new(state));

    println!("database loader setup");
    // Extremely dirty solution which looks to save database data to the disk every 20 seconds
    let database_state = state.clone();
    let database_handle = tokio::task::spawn(async move {
        loop {
            database_state
                .read()
                .await
                .to_disk(path::PathBuf::from(STORE_PATH))
                .await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    println!("loading webserver");
    // This task handles webserver requests
    let webserver_state = state.clone();
    let webserver_handle = tokio::task::spawn(async move {
        let scanner = PhotoScanner::new();

        let mut bars = Handlebars::new();
        bars.register_template_file("cookie", "./www/dynamic/cookie.handlebars")
            .expect("valid cookie template");
        bars.register_template_file("success", "./www/dynamic/success.handlebars")
            .expect("valid success template");

        WebServer::builder()
            .google_client_id(env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID is set"))
            .google_client_secret(
                env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET is set"),
            )
            .domain(env::var("BROWSER_BASE_URL").expect("BROWSER_BASE_URL is set"))
            .token_url("https://www.googleapis.com/oauth2/v3/token")
            .auth_url("https://accounts.google.com/o/oauth2/v2/auth")
            .handlebars(bars)
            .state(webserver_state)
            .scanner(scanner)
            .build()
            .run()
            .await;
    });

    println!("server started, waiting for new connections");
    join_all([webserver_handle, database_handle]).await;
}
