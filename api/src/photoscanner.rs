use crate::{
    json_templates::{GetMediaItems, MediaItem},
    AppState, UserId,
};
use reqwest::Method;
use std::sync::Arc;

pub async fn get_initial(
    state: Arc<AppState>,
    user_id: UserId,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error>> {
    let user_lock = state.users.read().await;
    let user_state = user_lock.get(&user_id);
    let user_state = match user_state {
        Some(s) => s,
        None => panic!("user not found"),
    };

    if user_state.initial_scan_completed {
        panic!("this user has already completed their userscan");
    }
    let next_token = user_state.next_token.as_ref();

    let response = reqwest::Client::new()
        .request(
            Method::GET,
            "https://photoslibrary.googleapis.com/v1/mediaItems",
        )
        .query(&[
            ("pageSize", "50"),
            ("pageToken", next_token.unwrap_or(&"".to_string())),
        ])
        .header("Content-type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", user_state.google_token.as_ref().unwrap().token), //TODO; respect invalid tokens
        )
        .send()
        .await
        .unwrap(); //TODO: handle error

    if response.status().is_success() {
        let body: Result<GetMediaItems, reqwest::Error> = response.json().await;
        let media_items = body.unwrap();

        let mut media_items_lock = state.users.write().await;
        let mut user = media_items_lock.get_mut(&user_id).unwrap();

        match media_items.nextPageToken {
            Some(s) => user.next_token = Some(s),
            None => {
                user.next_token = None;
                user.initial_scan_completed = true;
            }
        }

        Ok(media_items.mediaItems)
    } else {
        panic!("request failed");
    }
}

pub async fn get_new_photos(
    state: Arc<AppState>,
    user_id: UserId,
) -> Result<Vec<MediaItem>, Box<dyn std::error::Error>> {
    todo!()
}
