use std::time::{Duration, SystemTime};

use diesel::Queryable;
use rand::{distributions::Alphanumeric, Rng};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Id = String;
pub type Passcode = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub id: Id,
    pub passcode: Passcode,
}

impl Credentials {
    pub fn new() -> (Self, Passcode) {
        let id: Id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let passcode_insecure: Passcode = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(&passcode_insecure);
        let hashed_passcode = format!("{:x}", hasher.finalize());

        (
            Self {
                id,
                passcode: hashed_passcode,
            },
            passcode_insecure,
        )
    }

    pub fn verify_passcode(passcode: &Passcode, hashed_passcode: &Passcode) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(passcode);
        let hashed_passcode_test = format!("{:x}", hasher.finalize());

        hashed_passcode == &hashed_passcode_test
    }
}
