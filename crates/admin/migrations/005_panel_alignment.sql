-- gostc-rs: panel alignment with gostc-open (version 5).
--
-- 1) nodes: rebuild to widen the transport CHECK with 'wss' (gostc offers
--    TCP/KCP/QUIC/WS/WSS) and add gostc-style presentation fields:
--    remark, feature switches (web/forward/p2p), domain-resolution config
--    (http_port/domain), forward port quota, and traffic counters.
-- 2) clients: traffic counters (gostc client cards show IN | OUT).
-- 3) notices: admin announcements (gostc "通知公告").
-- 4) settings: key/value site configuration (gostc "系统配置-基础配置").

-- ---------------------------------------------------------------- nodes ----
CREATE TABLE nodes_new (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    UNIQUE NOT NULL,
    secret          TEXT    UNIQUE NOT NULL,
    api_endpoint    TEXT    NOT NULL,
    tunnel_endpoint TEXT    NOT NULL,
    transport       TEXT    NOT NULL DEFAULT 'tcp'
                       CHECK (transport IN ('tcp', 'quic', 'websocket', 'kcp', 'wss')),
    remark          TEXT    NOT NULL DEFAULT '',
    web             INTEGER NOT NULL DEFAULT 1,
    forward         INTEGER NOT NULL DEFAULT 1,
    p2p             INTEGER NOT NULL DEFAULT 1,
    http_port       TEXT    NOT NULL DEFAULT '',
    domain          TEXT    NOT NULL DEFAULT '',
    forward_ports   TEXT    NOT NULL DEFAULT '',
    input_bytes     INTEGER NOT NULL DEFAULT 0,
    output_bytes    INTEGER NOT NULL DEFAULT 0,
    status          TEXT    NOT NULL DEFAULT 'offline'
                       CHECK (status IN ('online', 'offline', 'disabled')),
    last_heartbeat  TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO nodes_new (id, name, secret, api_endpoint, tunnel_endpoint, transport,
                       status, last_heartbeat, created_at)
SELECT id, name, secret, api_endpoint, tunnel_endpoint, transport,
       status, last_heartbeat, created_at
FROM nodes;

DROP TABLE nodes;
ALTER TABLE nodes_new RENAME TO nodes;

-- -------------------------------------------------------------- clients ----
ALTER TABLE clients ADD COLUMN input_bytes  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clients ADD COLUMN output_bytes INTEGER NOT NULL DEFAULT 0;

-- -------------------------------------------------------------- notices ----
CREATE TABLE IF NOT EXISTS notices (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL,
    content    TEXT    NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ------------------------------------------------------------- settings ----
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES ('site_name', 'gostc-rs');
