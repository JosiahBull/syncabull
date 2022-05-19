mod auth;
mod database;
mod db_types;
mod json_templates;
mod photoscanner;
mod schema;
mod webserver;

#[macro_use]
extern crate diesel;

use auth::Token;
use database::{Database, GoogleAuth, UserData};
use webserver::WebServer;

use futures::future::join_all;
use handlebars::Handlebars;
use photoscanner::PhotoScanner;
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct AppState {
    users: RwLock<HashMap<String, UserData>>,
    tokens: RwLock<HashMap<String, Token>>,
    auth_keys: RwLock<HashMap<String, Token>>,
    unclaimed_auth_tokens: RwLock<HashMap<String, GoogleAuth>>,
}

#[tokio::main]
async fn main() {
    println!("Loading api...");
    let state = Arc::new(AppState::default());

    // This task handles interactions with the database, and is entirely transparent to the rest of the api
    let database_state = state.clone();
    let database_handle = tokio::task::spawn(async move {});

    // This is a cleaner task which looks for expired tokens, logins, etc and removes them automatically
    let token_cleaner_state = state.clone();
    let token_cleaner_handle = tokio::task::spawn(async move {});

    // This task handles webserver requests
    let webserver_state = state.clone();
    let webserver_handle = tokio::task::spawn(async move {
        println!("creating scanner");
        let scanner = PhotoScanner::new();

        println!("registering templates");
        let mut bars = Handlebars::new();
        bars.register_template_file("cookie", "./www/dynamic/cookie.handlebars")
            .expect("valid cookie template");
        bars.register_template_file("success", "./www/dynamic/success.handlebars")
            .expect("valid success template");

        println!("creating webserver");
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

    join_all([webserver_handle, database_handle, token_cleaner_handle]).await;
}
