// @generated automatically by Diesel CLI.

diesel::table! {
    google_auth_tokens (token) {
        associated_token -> Nullable<Text>,
        token -> Text,
        refresh_token -> Text,
        token_expiry_sec_epoch -> Text,
        user_id -> Nullable<Text>,
    }
}

diesel::table! {
    tokens (token) {
        token -> Text,
        user_id -> Nullable<Text>,
    }
}

diesel::table! {
    users (id) {
        id -> Text,
        hashed_passcode -> Text,
        initial_scan_completed -> Bool,
        next_token -> Nullable<Text>,
        prev_token -> Nullable<Text>,
    }
}

diesel::joinable!(google_auth_tokens -> users (user_id));
diesel::joinable!(tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    google_auth_tokens,
    tokens,
    users,
);
