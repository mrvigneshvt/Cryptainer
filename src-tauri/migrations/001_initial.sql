-- Cryptainer database schema
-- Containers table stores ONLY metadata.
-- Encrypted blobs are stored as flat files; blob_path points to them.
-- This separation means the DB can be backed up without exposing any secrets.

CREATE TABLE IF NOT EXISTS containers (
    id           TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    algo         TEXT NOT NULL DEFAULT 'AES-GCM-256',
    kdf          TEXT NOT NULL DEFAULT 'argon2id',
    kdf_params   TEXT NOT NULL,          -- JSON: KdfParams struct
    hint         TEXT,                   -- nullable password hint (plaintext)
    tags         TEXT,                   -- nullable comma-separated tags
    file_count   INTEGER NOT NULL DEFAULT 0,
    total_size   INTEGER NOT NULL DEFAULT 0,
    blob_path    TEXT NOT NULL UNIQUE,   -- absolute path to .enc blob on disk
    blob_sha256  TEXT NOT NULL,          -- hex SHA-256 for integrity verification
    created_at   TEXT NOT NULL,
    modified_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_containers_name ON containers(name);
CREATE INDEX IF NOT EXISTS idx_containers_created_at ON containers(created_at);
