#![allow(non_snake_case)]

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Serialize, Deserialize)]
pub struct MediaMetadata {
    pub creationTime: String,
    pub width: String,
    pub height: String,
    pub photo: Option<Photo>,
    pub video: Option<Video>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub id: Id,
    pub token: String,
    pub expiry: SystemTime,
}

#[derive(Serialize, Deserialize)]
pub struct Photo {
    pub cameraMake: Option<String>,
    pub cameraModel: Option<String>,
    pub focalLength: Option<f64>,
    pub apertureFNumber: Option<f64>,
    pub isoEquivalent: Option<u64>,
    pub exposureTime: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Video {
    pub cameraMake: Option<String>,
    pub cameraModel: Option<String>,
    pub fps: Option<f64>,
    pub status: Option<VideoProcessingStatus>,
}

#[derive(Serialize, Deserialize)]
pub enum VideoProcessingStatus {
    UNSPECIFIED,
    PROCESSING,
    READY,
    FAILED,
}

#[derive(Serialize, Deserialize)]
pub struct ContributorInfo {
    pub profilePictureBaseUrl: String,
    pub displayName: String,
}

#[derive(Serialize, Deserialize)]
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
    pub download_count: u32,
}
