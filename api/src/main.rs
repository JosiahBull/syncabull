mod auth;
mod photoscanner;
mod webserver;
mod database;
mod schema;

use photoscanner::PhotoScanner;
use tokio::sync::RwLock;
use webserver::WebServer;

use futures::future::join_all;
use handlebars::Handlebars;
use std::{
    env,
    path,
    sync::Arc,
    time::Duration,
};

use crate::database::AppState;

const STORE_PATH: &str = "store.json";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    println!("starting api");
    println!("loading state");
    // let state = match tokio::fs::metadata(path::Path::new(STORE_PATH)).await {
    //     Ok(_) => AppState::from_disk(path::PathBuf::from(STORE_PATH)).await,
    //     Err(_) => AppState::default(),
    // };

    let state = AppState::default();

    let state = Arc::new(RwLock::new(state));

    // Extremely dirty solution which looks to save database data to the disk every 60 seconds
    // println!("database loader setup");
    // let database_state = state.clone();
    // let database_handle = tokio::task::spawn(async move {
    //     loop {
    //         database_state
    //             .read()
    //             .await
    //             .to_disk(path::PathBuf::from(STORE_PATH))
    //             .await;
    //         tokio::time::sleep(Duration::from_secs(60)).await;
    //     }
    // });

    // Remove expired Tokens from the database, checking every 60 seconds
    // println!("token cleaner setup");
    // let token_cleaner_state = state.clone();
    // let token_cleaner_handle = tokio::task::spawn(async move {
    //     loop {
    //         {
    //             let mut state = token_cleaner_state.write().await;
    //             state.auth_keys.retain(|_, token| {
    //                 if token.is_expired() {
    //                     println!("token expired: {}", token.token);
    //                     false
    //                 } else {
    //                     true
    //                 }
    //             });
    //         }

    //         tokio::time::sleep(Duration::from_secs(60)).await;
    //     }
    // });

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
    join_all([webserver_handle]).await;
}
