--- create a table for media items
CREATE TABLE media (
    id TEXT PRIMARY KEY NOT NULL,
    description TEXT,
    product_url TEXT NOT NULL,
    base_url TEXT NOT NULL,
    mime_type TEXT,
    media_metadata TEXT,
    contributor_info TEXT,
    filename TEXT NOT NULL,
    download_attempts INTEGER NOT NULL,
    download_success BOOLEAN NOT NULL,
    download_timestamp TEXT NOT NULL
);

--- create a table for configuration
CREATE TABLE config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);