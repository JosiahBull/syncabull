mod auth;
mod json_templates;
mod webserver;

use std::{net::Ipv4Addr, sync::Arc, convert::Infallible, collections::HashMap};
use auth::{Credentials, Token};
use serde::{Serialize, Deserialize};
use warp::Filter;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct UserData {
    pub hashed_passcode: String,
    pub tokens: Vec<String>,
}

#[derive(Debug, Default)]
pub struct AppState {
    users: RwLock<HashMap<String, UserData>>,
    tokens: RwLock<HashMap<String, Token>>
}

fn with<T: Send + Sync>(
    data: Arc<T>,
) -> impl Filter<Extract = (Arc<T>,), Error = Infallible> + Clone {
    warp::any().map(move || data.clone())
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::default());

    // register this agent with the api
    let register = warp::get()
        .and(warp::path("register"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and_then(webserver::register)
        .recover(webserver::handle_custom_error);

    // log this agent into the api
    let login = warp::post()
        .and(warp::path("login"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and(warp::body::json::<Credentials>())
        .and_then(webserver::login)
        .recover(webserver::handle_custom_error);

    // check for new images to download
    let download = warp::get()
        .and(warp::path("download"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and(warp::header::header::<String>("authorisation"))
        .and_then(webserver::download)
        .recover(webserver::handle_custom_error);

    // initalise google login
    let google_login = warp::get()
        .and(warp::path("google_login"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and(warp::header::header::<String>("authorisation"))
        .and_then(webserver::google_login)
        .recover(webserver::handle_custom_error);

    // long poll for user login succeeding
    let login_check = warp::get()
        .and(warp::path("is_logged_in"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and(warp::header::header::<String>("authorisation"))
        .and_then(webserver::login_check)
        .recover(webserver::handle_custom_error);

    // delete all data associated with this user
    let delete_data = warp::delete()
        .and(warp::path("delete"))
        .and(warp::path::end())
        .and(with(state.clone()))
        .and(warp::header::header::<String>("authorisation"))
        .and_then(webserver::delete_data)
        .recover(webserver::handle_custom_error);

    // General catch-all endpoint if a failure occurs
    let catcher = warp::any()
        .and(warp::path::full())
        .map(|path| format!("Path {:?} not found", path));

    let routes = warp::any()
        .and(warp::path("api"))
        .and(warp::path("1"))
        .and(register.or(login).or(download).or(google_login).or(login_check).or(delete_data).or(catcher))
        .or(catcher);

    warp::serve(routes)
        .run((
            std::env::var("HOST")
                .expect("HOST to be set")
                .parse::<Ipv4Addr>()
                .expect("valid port"),
            std::env::var("PORT")
                .expect("PORT to be set")
                .parse()
                .expect("valid port"),
        ))
        .await;
}