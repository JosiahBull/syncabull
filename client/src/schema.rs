// @generated automatically by Diesel CLI.

diesel::table! {
    config (key) {
        key -> Text,
        value -> Text,
    }
}

diesel::table! {
    media (id) {
        id -> Text,
        description -> Nullable<Text>,
        product_url -> Text,
        base_url -> Text,
        mime_type -> Nullable<Text>,
        filename -> Text,

        download_attempts -> Integer,
        download_success -> Bool,
        download_timestamp -> Text,

        creation_time -> Nullable<Text>,
        width -> Nullable<Text>,
        height -> Nullable<Text>,

        camera_make -> Nullable<Text>,
        camera_model -> Nullable<Text>,

        focal_length -> Nullable<Float>,
        aperture -> Nullable<Float>,
        iso_equivalent -> Nullable<Integer>,
        exposure_time -> Nullable<Text>,
        fps -> Nullable<Float>,
        processing_status -> Nullable<Text>,
        profile_picture_url -> Nullable<Text>,
        display_name -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(config, media,);
