#![allow(non_snake_case)]

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct QueryData {
    pub code: String,
}

#[derive(Deserialize)]
pub struct RequestParameters {
    pub reload: bool,
    pub max_count: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MediaMetadata {
    pub creationTime: String,
    pub width: String,
    pub height: String,
    pub photo: Option<Photo>,
    pub video: Option<Video>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Photo {
    pub cameraMake: Option<String>,
    pub cameraModel: Option<String>,
    pub focalLength: Option<f64>,
    pub apertureFNumber: Option<f64>,
    pub isoEquivalent: Option<u64>,
    pub exposureTime: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Video {
    pub cameraMake: Option<String>,
    pub cameraModel: Option<String>,
    pub fps: Option<f64>,
    pub status: Option<VideoProcessingStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VideoProcessingStatus {
    UNSPECIFIED,
    PROCESSING,
    READY,
    FAILED,
}

impl Display for VideoProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoProcessingStatus::UNSPECIFIED => write!(f, "UNSPECIFIED"),
            VideoProcessingStatus::PROCESSING => write!(f, "PROCESSING"),
            VideoProcessingStatus::READY => write!(f, "READY"),
            VideoProcessingStatus::FAILED => write!(f, "FAILED"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContributorInfo {
    pub profilePictureBaseUrl: String,
    pub displayName: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MediaItem {
    pub id: String,
    pub description: Option<String>,
    pub productUrl: String,
    pub baseUrl: String,
    pub mimeType: Option<String>,
    pub mediaMetadata: Option<MediaMetadata>,
    pub contributorInfo: Option<ContributorInfo>,
    pub filename: String,

    #[serde(default)]
    pub download_attempts: u32,

    #[serde(default)]
    pub download_success: bool,
}

#[derive(Deserialize)]
pub struct GetMediaItems {
    #[serde(default)]
    pub mediaItems: Vec<MediaItem>,
    pub nextPageToken: Option<String>,
}

#[derive(Deserialize)]
pub struct GoogleProfile {
    /// Google ID for user
    pub sub: String,
    /// Url to profile picture of user
    pub picture: String,
    /// Email address of user
    pub email: String,
    /// Whether the email of this user has been verified
    pub email_verified: bool,
}
