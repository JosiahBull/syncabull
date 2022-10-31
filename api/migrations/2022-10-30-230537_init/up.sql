--- create a table for user data
CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    hashed_passcode TEXT NOT NULL,
    initial_scan_completed BOOLEAN NOT NULL,
    next_token TEXT,
    prev_token TEXT
);

--- create a table for user tokens
CREATE TABLE tokens (
    token TEXT NOT NULL PRIMARY KEY,
    user_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

--- create a table for google auth tokens
CREATE TABLE google_auth_tokens (
    associated_token TEXT,
    token TEXT NOT NULL PRIMARY KEY,
    refresh_token TEXT NOT NULL,
    token_expiry_sec_epoch TEXT NOT NULL,
    user_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);