use crate::schema::*;
use std::time::SystemTime;

#[derive(Debug, QueryableByName, Insertable)]
#[table_name = "google_auth"]
pub struct GoogleAuthDB {
    pub user_id: String,
    pub token: String,
    pub token_expiry_sec_epoch: SystemTime,
    pub refresh_token: String,
}

#[derive(Debug, QueryableByName)]
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

#[derive(Debug, QueryableByName, Insertable)]
#[table_name = "users"]
pub struct UserDB {
    pub id: String,
    pub hashed_passcode: String,
    pub crt: SystemTime,
}