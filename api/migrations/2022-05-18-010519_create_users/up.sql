CREATE TABLE IF NOT EXISTS users (
    -- Unique ID for this user. Note that these are randomly generated, and are not concurrent
    id TEXT PRIMARY KEY,
    -- sha256 hash of the randomly generated passcode
    hashed_passcode TEXT NOT NULL,
    -- When account was created
    crt TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS google_auth (
    -- ID of this google authentication
    id SERIAL PRIMARY KEY,
    -- the id of the user who's authenticated with this google account
    user_id TEXT NOT NULL,
    -- the provided google token
    token TEXT NOT NULL,
    -- when the above token wille expire
    token_expiry_sec_epoch TIMESTAMP NOT NULL,
    -- the token used to refresh it
    refresh_token TEXT NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tokens (
    -- The id of the token
    id SERIAL PRIMARY KEY,
    -- The id of the user who owns this token
    user_id TEXT NOT NULL,
    -- the token value itself
    token TEXT NOT NULL,
    -- when this token will expire
    expiry TIMESTAMP NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);