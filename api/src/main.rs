mod auth;
mod json_templates;
mod webserver;

use auth::Token;
use futures::future::join_all;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use webserver::WebServer;
use std::{collections::HashMap, hash::Hash, env, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleAuth {
    /// A bearer token used to access the google api
    pub token: String,
    /// Time when the above bearer token expires, in seconds since unix epoch
    pub token_expiry_sec_epoch: u64,
    /// Token used to refresh the bearer token with the google api
    pub refresh_token: String,
}

#[derive(Debug, Default)]
pub struct UserData {
    pub hashed_passcode: String,
    pub tokens: Vec<String>,
    pub google_auth: Option<GoogleAuth>,
}

#[derive(Debug, Default)]
pub struct AppState {
    users: RwLock<HashMap<String, UserData>>,
    tokens: RwLock<HashMap<String, Token>>,
    auth_keys: RwLock<HashMap<String, Token>>,
    unclaimed_auth_tokens: RwLock<HashMap<String, GoogleAuth>>,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::default());

    let webserver_state = state.clone();
    let webserver_handle = tokio::task::spawn(async move {

        let mut bars = Handlebars::new();
        bars.register_template_file("cookie", "./www/dynamic/cookie.handlebars").expect("valid cookie template");
        bars.register_template_file("success", "./www/dynamic/success.handlebars").expect("valid success template");

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
            .build()
            .run()
            .await;
    });


    //TODO: write a cleaner function
    // which will loop through and remove auth keys and expired tokens


    join_all([webserver_handle]).await;
}
