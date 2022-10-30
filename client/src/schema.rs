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
        media_metadata -> Nullable<Text>,
        contributor_info -> Nullable<Text>,
        filename -> Text,
        download_attempts -> Integer,
        download_success -> Bool,
        download_timestamp -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    config,
    media,
);
