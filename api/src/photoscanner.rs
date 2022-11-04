#![allow(dead_code)]

use reqwest::Method;
use shared_libs::json_templates::GetMediaItems;
use std::time::Duration;

use crate::GoogleAuth;

#[derive(Debug)]
pub enum ScanningError {
    NoConnection,
    InvalidGoogleAuth,
    NetworkFailure(reqwest::Error),
    InternalFailure(String),
}

impl std::fmt::Display for ScanningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ScanningError::NoConnection => write!(f, "No connection to Google Photos"),
            ScanningError::InvalidGoogleAuth => write!(f, "Invalid Google Auth"),
            ScanningError::NetworkFailure(ref err) => write!(f, "Network failure: {}", err),
            ScanningError::InternalFailure(ref msg) => write!(f, "Internal failure: {}", msg),
        }
    }
}

impl std::error::Error for ScanningError {}

impl From<reqwest::Error> for ScanningError {
    fn from(err: reqwest::Error) -> Self {
        ScanningError::NetworkFailure(err)
    }
}

#[derive(Debug)]
pub struct PhotoScanner {
    timeout_ms: u64,
}

impl PhotoScanner {
    pub fn new() -> Self {
        Self { timeout_ms: 20_000 }
    }

    pub async fn scan(
        &self,
        auth: &GoogleAuth,
        max_photos: u8,
        token: Option<String>,
    ) -> Result<GetMediaItems, ScanningError> {
        if auth.is_expired() {
            return Err(ScanningError::InvalidGoogleAuth);
        }

        let mut query = Vec::with_capacity(2);
        query.push(("pageSize", max_photos.to_string()));
        if let Some(page_token) = token {
            query.push(("pageToken", page_token));
        }

        let response = reqwest::Client::new()
            .request(
                Method::GET,
                "https://photoslibrary.googleapis.com/v1/mediaItems",
            )
            .query(&query)
            .header("Content-type", "application/json")
            .header("Authorization", format!("Bearer {}", auth.token))
            .timeout(Duration::from_millis(self.timeout_ms))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScanningError::InternalFailure(format!(
                "{}",
                response.status()
            )));
        }

        let body_str = response.text().await?;

        let body: GetMediaItems = match serde_json::from_str(&body_str) {
            Ok(body) => body,
            Err(e) => {
                return Err(ScanningError::InternalFailure(format!(
                    "{}\n{}",
                    body_str, e
                )))
            }
        };

        Ok(body)
    }
}
