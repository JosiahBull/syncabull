--- create a table for media items
CREATE TABLE media (
    --- Core Data
    id TEXT PRIMARY KEY NOT NULL,
    description TEXT,
    product_url TEXT NOT NULL,
    base_url TEXT NOT NULL,
    mime_type TEXT,
    filename TEXT NOT NULL,

    --- Extra Fields
    download_attempts INTEGER NOT NULL,
    download_success BOOLEAN NOT NULL,
    download_timestamp TEXT NOT NULL,

    --- Extra (Optional) Metadata
    creation_time TEXT,
    width TEXT,
    height TEXT,
    camera_make TEXT,
    camera_model TEXT,

    --- Photo Metadata (if photo)
    focal_length REAL,
    aperture REAL,
    iso_equivalent INTEGER,
    exposure_time TEXT,

    --- Video Metadata (if video)
    fps REAL,
    processing_status TEXT,

    -- contributor_info
    profile_picture_url TEXT,
    display_name TEXT
);

--- create a table for configuration
CREATE TABLE config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
