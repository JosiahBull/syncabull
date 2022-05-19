use crate::schema::*;
use std::time::SystemTime;

#[derive(Debug, Queryable, QueryableByName)]
#[table_name = "google_auth"]
pub struct GoogleAuthDB {
    pub id: i32,
    pub user_id: String,
    pub token: String,
    pub token_expiry_sec_epoch: SystemTime,
    pub refresh_token: String,
}

#[derive(Debug, Insertable)]
#[table_name = "google_auth"]
pub struct NewGoogleAuth {
    pub user_id: String,
    pub token: String,
    pub token_expiry_sec_epoch: SystemTime,
    pub refresh_token: String,
}

#[derive(Debug, Queryable, QueryableByName)]
#[table_name = "tokens"]
pub struct TokenDB {
    pub id: i32,
    pub user_id: String,
    pub token: String,
    pub expiry: SystemTime,
}

#[derive(Insertable)]
#[table_name = "tokens"]
pub struct NewToken {
    pub user_id: String,
    pub token: String,
    pub expiry: SystemTime,
}

#[derive(Debug, Queryable, QueryableByName, Insertable)]
#[table_name = "users"]
pub struct UserDB {
    pub id: String,
    pub hashed_passcode: String,
    pub crt: SystemTime,
}
