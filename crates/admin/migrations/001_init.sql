-- gostc-rs admin initial schema
-- Version 1

CREATE TABLE IF NOT EXISTS users (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    username             TEXT    UNIQUE NOT NULL,
    password_hash        TEXT    NOT NULL,
    role                 TEXT    NOT NULL CHECK (role IN ('admin', 'user')),
    traffic_quota_bytes  INTEGER,
    bandwidth_limit_bps  INTEGER,
    must_change_password INTEGER NOT NULL DEFAULT 0,
    created_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS nodes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    UNIQUE NOT NULL,
    secret          TEXT    UNIQUE NOT NULL,
    api_endpoint    TEXT    NOT NULL,
    tunnel_endpoint TEXT    NOT NULL,
    status          TEXT    NOT NULL DEFAULT 'offline'
                       CHECK (status IN ('online', 'offline', 'disabled')),
    last_heartbeat  TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tunnels (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id     INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    type        TEXT    NOT NULL CHECK (type IN ('tcp', 'udp', 'http', 'https')),
    local_addr  TEXT    NOT NULL,
    remote_port INTEGER,
    domain      TEXT,
    token       TEXT    NOT NULL,
    status      TEXT    NOT NULL DEFAULT 'paused'
                   CHECK (status IN ('active', 'paused')),
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (node_id, name)
);

CREATE INDEX IF NOT EXISTS idx_tunnels_user_id ON tunnels(user_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_node_id ON tunnels(node_id);

CREATE TABLE IF NOT EXISTS traffic_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tunnel_id   INTEGER NOT NULL REFERENCES tunnels(id) ON DELETE CASCADE,
    bytes_in    INTEGER NOT NULL,
    bytes_out   INTEGER NOT NULL,
    recorded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_traffic_logs_tunnel_recorded
    ON traffic_logs(tunnel_id, recorded_at);

CREATE TABLE IF NOT EXISTS audit_logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER REFERENCES users(id) ON DELETE SET NULL,
    action     TEXT    NOT NULL,
    target     TEXT,
    details    TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);