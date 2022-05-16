use std::time::{UNIX_EPOCH, SystemTime};

use rand::{Rng, distributions::Alphanumeric};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

pub type Id = String;
pub type Passcode = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub id: Id,
    pub passcode: Passcode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub id: Id,
    pub token: String,
    pub expiry: u64,
}

impl Token {
    pub fn generate_token(id: &Id) -> Token {
        Token {
            id: id.clone(),
            token: rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect(),
            expiry: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
        }
    }

    /// function to check if token has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() > self.expiry
    }
}

impl Credentials {
    pub fn new() -> (Self, Passcode) {
        let id: Id = rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
        let passcode_insecure: Passcode =  rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();

        let mut hasher = Sha256::new();
        hasher.update(&passcode_insecure);
        let hashed_passcode = format!("{:x}", hasher.finalize());

        (Self {
            id,
            passcode: hashed_passcode,
        }, passcode_insecure)
    }

    pub fn verify_passcode(passcode: &Passcode, hashed_passcode: &Passcode) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(passcode);
        let hashed_passcode_test = format!("{:x}", hasher.finalize());

        hashed_passcode == &hashed_passcode_test
    }
}