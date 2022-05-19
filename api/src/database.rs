//! This module handles database interaction

use crate::auth::Token;
use crate::db_types::*;
use crate::schema::{google_auth, tokens, users};
use async_trait::async_trait;
use diesel::QueryDsl;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    ExpressionMethods, OptionalExtension, PgConnection, RunQueryDsl,
};
use serde::{Deserialize, Serialize};
use std::{net::Ipv4Addr, time::SystemTime};

type PooledPg = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleAuth {
    /// A bearer token used to access the google api
    pub token: String,
    /// Time when the above bearer token expires, in seconds since unix epoch
    pub token_expiry_sec_epoch: SystemTime,
    /// Token used to refresh the bearer token with the google api
    pub refresh_token: String,
}

impl GoogleAuth {
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.token_expiry_sec_epoch
    }
}

#[derive(Debug, Default)]
pub struct UserData {
    pub hashed_passcode: String,
    pub tokens: Vec<String>,
    pub google_auth: Option<GoogleAuth>,
}

pub struct DatabaseInformation {
    pub database_host: Ipv4Addr,
    pub database_port: u16,
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
}

#[async_trait]
pub trait DatabaseMethods<Id, Item, Error: std::error::Error> {
    /// create a new database
    async fn new(info: DatabaseInformation) -> Result<Self, Error>
    where
        Self: Sized;
    /// add an item to the database
    async fn insert(&mut self, id: Id, item: Item) -> Result<Option<Item>, Error>;
    /// get an item from the database
    async fn get(&mut self, id: &Id) -> Result<Option<Item>, Error>;
    /// delete an item from the database
    async fn delete(&mut self, id: &Id) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum DatabaseError {}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for DatabaseError {}

pub struct Database {
    pool: PooledPg,
}

#[async_trait]
impl DatabaseMethods<String, (UserData, Vec<Token>), DatabaseError> for Database {
    async fn new(data: DatabaseInformation) -> Result<Self, DatabaseError> {
        let manager = ConnectionManager::<PgConnection>::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            data.database_user,
            data.database_password,
            data.database_host,
            data.database_port,
            data.database_name,
        ));
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create pool");
        Ok(Self { pool })
    }

    async fn insert(
        &mut self,
        id: String,
        (user_data, tokens): (UserData, Vec<Token>),
    ) -> Result<Option<(UserData, Vec<Token>)>, DatabaseError> {
        //Split item into it's component parts

        let user = UserDB {
            id: id.clone(),
            hashed_passcode: user_data.hashed_passcode,
            crt: SystemTime::now(),
        };

        let tokens: Vec<NewToken> = tokens
            .into_iter()
            .map(|token| NewToken {
                user_id: token.id,
                token: token.token,
                expiry: token.expiry,
            })
            .collect();

        let google_auth = match user_data.google_auth {
            Some(auth) => Some(NewGoogleAuth {
                user_id: id,
                token: auth.token,
                token_expiry_sec_epoch: auth.token_expiry_sec_epoch,
                refresh_token: auth.refresh_token,
            }),
            None => None,
        };

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let con = pool.get().expect("Failed to get connection from pool");
            diesel::insert_into(users::table)
                .values(&user)
                .execute(&con)
                .expect("Failed to insert user");

            diesel::insert_into(tokens::table)
                .values(&tokens)
                .execute(&con)
                .expect("Failed to insert tokens");

            if let Some(auth) = google_auth {
                diesel::insert_into(google_auth::table)
                    .values(&auth)
                    .execute(&con)
                    .expect("Failed to insert google auth");
            }
        })
        .await
        .unwrap();

        //TODO: check for existing users...?
        Ok(None)
    }

    async fn get(&mut self, id: &String) -> Result<Option<(UserData, Vec<Token>)>, DatabaseError> {
        let id = id.clone();
        let pool = self.pool.clone();
        let result: Result<Option<(UserData, Vec<Token>)>, DatabaseError> =
            tokio::task::spawn_blocking(move || {
                let con = pool.get().expect("Failed to get connection from pool");

                let user: Option<UserDB> = users::table
                    .filter(users::id.eq(&id))
                    .first(&con)
                    .optional()
                    .expect("Failed to get user");

                let user = match user {
                    Some(user) => user,
                    None => return Ok(None),
                };

                let google_auth: Option<GoogleAuthDB> = google_auth::table
                    .filter(google_auth::user_id.eq(&id))
                    .first(&con)
                    .optional()
                    .expect("Failed to get google auth");

                let google_auth: Option<GoogleAuth> = match google_auth {
                    Some(auth) => Some(GoogleAuth {
                        token: auth.token,
                        token_expiry_sec_epoch: auth.token_expiry_sec_epoch,
                        refresh_token: auth.refresh_token,
                    }),
                    None => None,
                };

                let tokens: Option<Vec<TokenDB>> = tokens::table
                    .filter(tokens::user_id.eq(&id))
                    .load(&con)
                    .optional()
                    .expect("Failed to get tokens");

                let tokens = match tokens {
                    Some(tokens) => tokens
                        .into_iter()
                        .map(|token| Token {
                            id: token.user_id,
                            token: token.token,
                            expiry: token.expiry,
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let user: UserData = UserData {
                    hashed_passcode: user.hashed_passcode,
                    google_auth,
                    tokens: tokens.iter().map(|x| x.token.clone()).collect(),
                };

                Ok(Some((user, tokens)))
            })
            .await
            .unwrap();

        result
    }

    async fn delete(&mut self, id: &String) -> Result<(), DatabaseError> {
        let id = id.clone();
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let con = pool.get().expect("Failed to get connection from pool");
            diesel::delete(users::table.find(id))
                .execute(&con)
                .expect("Failed to delete user");
        })
        .await
        .unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, convert::Infallible};

    use async_trait::async_trait;

    use super::{DatabaseInformation, DatabaseMethods};

    pub struct MockDatabase {
        storage: HashMap<String, String>,
    }

    #[async_trait]
    impl DatabaseMethods<String, String, Infallible> for MockDatabase {
        async fn new(_: DatabaseInformation) -> Result<Self, Infallible> {
            Ok(MockDatabase {
                storage: Default::default(),
            })
        }

        async fn insert(&mut self, id: String, item: String) -> Result<Option<String>, Infallible> {
            self.storage.insert(id, item);
            Ok(None)
        }

        async fn get(&mut self, id: &String) -> Result<Option<String>, Infallible> {
            Ok(self.storage.get(id).cloned())
        }

        async fn delete(&mut self, id: &String) -> Result<(), Infallible> {
            self.storage.remove(id);
            Ok(())
        }
    }
}
