use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryData {
    pub code: String,
}

#[derive(Deserialize)]
pub struct MediaMetadata {
    pub creationTime: String,
    pub width: String,
    pub height: String,
    pub photo: Option<Photo>,
    pub video: Option<Video>,
}

#[derive(Deserialize)]
pub struct Photo {
    pub cameraMake: String,
    pub cameraModel: String,
    pub focalLength: f64,
    pub apertureFNumber: f64,
    pub isoEquivalent: u64,
    pub exposureTime: String,
}

#[derive(Deserialize)]
pub struct Video {
    pub cameraMake: String,
    pub cameraModel: String,
    pub fps: f64,
    pub status: VideoProcessingStatus,
}

#[derive(Deserialize)]
pub enum VideoProcessingStatus {
    UNSPECIFIED,
    PROCESSING,
    READY,
    FAILED,
}

#[derive(Deserialize)]
pub struct ContributorInfo {
    pub profilePictureBaseUrl: String,
    pub displayName: String,
}

#[derive(Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub description: String,
    pub productUrl: String,
    pub baseUrl: String,
    pub mimeType: String,
    pub mediaMetadata: MediaMetadata,
    pub contributorInfo: ContributorInfo,
    pub filename: String,
}

#[derive(Deserialize)]
pub struct GetMediaItems {
    pub mediaItems: Vec<MediaItem>,
    pub nextPageToken: Option<String>,
}
