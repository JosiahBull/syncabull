use std::{time::{SystemTime, Duration}, collections::HashMap, path::PathBuf, error::Error, sync::Arc};

use diesel::{sqlite::Sqlite, Connection, RunQueryDsl, Queryable, associations::HasTable, Insertable};
use diesel_migrations::{EmbeddedMigrations, embed_migrations, MigrationHarness};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

use crate::auth::Id;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type DbConnection = diesel::SqliteConnection;
pub type DB = Sqlite;

pub fn run_migrations(
    connection: &mut impl MigrationHarness<DB>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

pub fn establish_connection(
    database_url: &str,
) -> Result<DbConnection, Box<dyn Error + Send + Sync + 'static>> {
    Ok(DbConnection::establish(database_url)?)
}

#[derive(Debug, Queryable, Insertable, PartialEq)]
#[diesel(treat_none_as_default_value = false)]
#[diesel(table_name = crate::schema::tokens)]
struct DatabaseToken {
    token: String,
    user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub id: Id,
    pub token: String,
    pub expiry: SystemTime,
}

impl Token {
    pub fn generate_token(id: &Id) -> Token {
        Token {
            id: id.clone(),
            token: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(32)
                .map(char::from)
                .collect(),
            expiry: SystemTime::now()
                .checked_add(Duration::from_secs(60*5)) // 5 minutes to complete auth
                .unwrap(),
        }
    }

    /// function to check if token has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expiry
    }
}

#[derive(Debug, Queryable, Insertable, PartialEq)]
#[diesel(treat_none_as_default_value = false)]
#[diesel(table_name = crate::schema::google_auth_tokens)]
struct DatabaseGoogleToken {
    /// If the token hasn't been claimed yet, this will provide the token required to acquire it
    associated_token: Option<String>,
    token: String,
    refresh_token: String,
    token_expiry_sec_epoch: String,
    user_id: Option<String>,
}


#[derive(Debug, Clone)]
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

#[derive(Debug, Queryable, Insertable, PartialEq)]
#[diesel(treat_none_as_default_value = false)]
#[diesel(table_name = crate::schema::users)]
struct DatabaseUser {
    id: String,
    hashed_passcode: String,
    initial_scan_completed: bool,
    next_token: Option<String>,
    prev_token: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct UserData {
    pub hashed_passcode: String,
    pub tokens: Vec<String>,
    pub google_auth: Option<GoogleAuth>,
    /// If the initial scan has been completed
    pub initial_scan_complete: bool,
    /// The token for the next page
    pub next_token: Option<String>,
    /// The previous token that was used, so the user can repeat a request if required
    pub prev_token: Option<String>,
}

pub struct AppState {
    users: HashMap<String, UserData>,
    auth_keys: HashMap<String, Token>,
    unclaimed_auth_tokens: HashMap<String, GoogleAuth>,
    connection: Arc<Mutex<DbConnection>>,
}

impl AppState {
    pub async fn load(
        connection: Arc<Mutex<DbConnection>>,
    ) -> Result<Self, Box<dyn Error + Sync + Send + 'static>> {
        tokio::task::spawn_blocking(move || {
            let mut state = Self {
                users: HashMap::new(),
                auth_keys: HashMap::new(),
                unclaimed_auth_tokens: HashMap::new(),
                connection,
            };
            let users = crate::schema::users::table
                .load::<DatabaseUser>(&mut *state.connection.blocking_lock())?;
            for user in users {
                state.users.insert(
                    user.id,
                    UserData {
                        hashed_passcode: user.hashed_passcode,
                        tokens: vec![],
                        google_auth: None,
                        initial_scan_complete: user.initial_scan_completed,
                        next_token: user.next_token,
                        prev_token: user.prev_token,
                    },
                );
            }

            let tokens = crate::schema::tokens::table
                .load::<DatabaseToken>(&mut *state.connection.blocking_lock())?;
            for token in tokens {
                // if token has a user_id - add to that user, otherwise add it generally
                if let Some(user_id) = token.user_id {
                    state.users.get_mut(&user_id).unwrap().tokens.push(token.token);
                } else {
                    state.auth_keys.insert(token.token, Token::generate_token(&Id::new()));
                }
            }

            let google_tokens = crate::schema::google_auth_tokens::table
                .load::<DatabaseGoogleToken>(&mut *state.connection.blocking_lock())?;

            for google_token in google_tokens {
                // if token has a user_id - add to that user, otherwise add it generally
                if let Some(user_id) = google_token.user_id {
                    state.users.get_mut(&user_id).unwrap().google_auth = Some(GoogleAuth {
                        token: google_token.token,
                        token_expiry_sec_epoch: SystemTime::UNIX_EPOCH + Duration::from_secs(google_token.token_expiry_sec_epoch.parse::<u64>().unwrap()),
                        refresh_token: google_token.refresh_token,
                    });
                } else {
                    state.unclaimed_auth_tokens.insert(google_token.associated_token.unwrap(), GoogleAuth {
                        token: google_token.token,
                        token_expiry_sec_epoch: SystemTime::UNIX_EPOCH + Duration::from_secs(google_token.token_expiry_sec_epoch.parse::<u64>().unwrap()),
                        refresh_token: google_token.refresh_token,
                    });
                }
            }

            Ok(state)
        }).await?
    }

    pub async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<&UserData>, Box<dyn Error + Sync + Send + 'static>> {
        Ok(self.users.get(username))
    }

    pub async fn user_exists(&self, username: &str) -> Result<bool, Box<dyn Error + Sync + Send + 'static>> {
        Ok(self.users.contains_key(username))
    }

    pub async fn add_user(
        &mut self,
        user_id: String,
        user_data: UserData,
    ) -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
        self.users.insert(user_id.clone(), user_data.clone());

        tokio::task::spawn_blocking(move || -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
            let user = DatabaseUser {
                id: user_id.to_string(),
                hashed_passcode: user_data.hashed_passcode,
                initial_scan_completed: user_data.initial_scan_complete,
                next_token: user_data.next_token,
                prev_token: user_data.prev_token,
            };
            diesel::insert_into(crate::schema::users::table)
                .values(&user)
                .execute(&mut *self.connection.blocking_lock())?;


            // insert tokens and google_auth if present
            if let Some(google_auth) = user_data.google_auth {
                let google_token = DatabaseGoogleToken {
                    token: google_auth.token,
                    refresh_token: google_auth.refresh_token,
                    token_expiry_sec_epoch: google_auth.token_expiry_sec_epoch.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string(),
                    user_id: Some(user_id.clone()),
                    associated_token: None,
                };
                diesel::insert_into(crate::schema::google_auth_tokens::table)
                    .values(&google_token)
                    .execute(&mut *self.connection.blocking_lock())?;
            }

            for token in user_data.tokens {
                let token = DatabaseToken {
                    token,
                    user_id: Some(user_id.clone()),
                };
                diesel::insert_into(crate::schema::tokens::table)
                    .values(&token)
                    .execute(&mut *self.connection.blocking_lock())?;
            }

            Ok(())
        }).await??;

        Ok(())
    }
}
