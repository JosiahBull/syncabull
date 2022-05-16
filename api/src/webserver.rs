use std::{sync::Arc, convert::Infallible};

use reqwest::StatusCode;
use warp::{Reply, Rejection, reject::Reject};

use crate::{AppState, auth::{Credentials, Token}, UserData};

#[derive(Debug)]
pub struct CustomError(String, StatusCode);

impl CustomError {
    pub fn new(msg: String, status: StatusCode) -> CustomError {
        CustomError(msg, status)
    }
}

impl Reject for CustomError {}

pub async fn handle_custom_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(CustomError(msg, status)) = err.find::<CustomError>() {
        Ok(warp::reply::with_status(msg.clone(), *status))
    } else {
        Err(err)
    }
}

pub async fn register(
    state: Arc<AppState>,
) -> Result<impl Reply, Infallible> {
    let mut auth: Credentials;
    let mut insecure: String;
    loop {
        (auth, insecure) = Credentials::new();
        if !state.users.read().await.contains_key(&auth.id) {
            break;
        }
    }

    state.users.write().await.insert(auth.id.clone(), UserData {
        hashed_passcode: auth.passcode,
        tokens: Vec::new(),
    });

    auth.passcode = insecure;
    Ok(warp::reply::with_status(
        warp::reply::json(&auth),
        warp::http::StatusCode::OK,
    ))
}

pub async fn login(
    state: Arc<AppState>,
    creds: Credentials,
) -> Result<impl Reply, Rejection> {
    let hashed_passcode = match state.users.read().await.get(&creds.id) {
        Some(s) => s.hashed_passcode.clone(), //clone requires alloc, but it allows us to drop the rwlock
        None => return Err(warp::reject::custom(CustomError::new(String::from("invalid login"), StatusCode::UNAUTHORIZED))),
    };

    if !Credentials::verify_passcode(&creds.passcode, &hashed_passcode) {
        return Err(warp::reject::custom(CustomError::new(String::from("invalid login"), StatusCode::UNAUTHORIZED)));
    }

    let token = Token::generate_token(&creds.id);

    let reply = warp::reply::with_status(
        warp::reply::json(&token),
        warp::http::StatusCode::OK,
    );
    state.tokens.write().await.insert(token.token.clone(), token);

    Ok(reply)
}

pub async fn download(
    state: Arc<AppState>,
    token: String
) -> Result<impl Reply, Rejection> {
    Ok(String::from("not implemented"))
}

pub async fn google_login(
    state: Arc<AppState>,
    token: String
) -> Result<impl Reply, Infallible> {
    Ok(String::from("not implemented"))

}

pub async fn login_check(
    state: Arc<AppState>,
    token: String
) -> Result<impl Reply, Infallible> {
    Ok(String::from("not implemented"))

}

pub async fn delete_data(
    state: Arc<AppState>,
    token: String
) -> Result<impl Reply, Rejection> {
    let user_id = match state.tokens.read().await.get(&token) {
        Some(t) => t.id.clone(),
        None => return Err(warp::reject::custom(CustomError::new(String::from("invalid login"), StatusCode::UNAUTHORIZED))),
    };

    let user = match state.users.write().await.remove(&user_id) {
        Some(u) => u,
        None => return Err(warp::reject::custom(CustomError::new(String::from("invalid user state, token not removed"), StatusCode::UNAUTHORIZED))),
    };

    // remove all tokens for this user
    if !user.tokens.is_empty() {
        let mut writer = state.tokens.write().await;
        for token in user.tokens {
            writer.remove(&token);
        }
    }

    Ok(warp::reply::with_status("", StatusCode::NO_CONTENT))
}
