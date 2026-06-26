CREATE TABLE IF NOT EXISTS audit_log (
    id             TEXT PRIMARY KEY,
    ts             TEXT NOT NULL,
    action         TEXT NOT NULL,
    container_id   TEXT,
    container_name TEXT,
    details        TEXT
);

CREATE INDEX IF NOT EXISTS audit_log_ts_idx ON audit_log(ts DESC);
