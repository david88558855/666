-- gostc-rs: secret tunnels (frp-style STCP/SUDP, self-built — orbien has no
-- visitor model). Version 6.
--
-- 1) tunnels: rebuild to widen the type CHECK with 'stcp'/'sudp' and add
--    sk_hash (hashed secret key; the plaintext sk is never stored).
--    Secret tunnels are NOT exposed on a node public port — access goes
--    visitor client -> (P2P direct per ARCHITECTURE.md 7.8, or node relay
--    fallback) -> service client -> local target.
-- 2) tunnel_visitors: the visitor side of a secret tunnel. Each visitor is
--    bound to one secret tunnel and one visitor-side client, and listens on
--    a local port on the visitor machine.

CREATE TABLE tunnels_new (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id     INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    client_id   INTEGER,
    name        TEXT    NOT NULL,
    type        TEXT    NOT NULL CHECK (type IN ('tcp', 'udp', 'http', 'https',
                                                'stcp', 'sudp')),
    local_addr  TEXT    NOT NULL,
    remote_port INTEGER,
    domain      TEXT,
    token       TEXT    NOT NULL,
    sk_hash     TEXT,
    status      TEXT    NOT NULL DEFAULT 'paused'
                   CHECK (status IN ('active', 'paused')),
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (node_id, name)
);

INSERT INTO tunnels_new (id, user_id, node_id, client_id, name, type, local_addr,
                         remote_port, domain, token, status, created_at)
SELECT id, user_id, node_id, client_id, name, type, local_addr,
       remote_port, domain, token, status, created_at
FROM tunnels;

DROP TABLE tunnels;
ALTER TABLE tunnels_new RENAME TO tunnels;

CREATE INDEX IF NOT EXISTS idx_tunnels_user_id ON tunnels(user_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_node_id ON tunnels(node_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_client_id ON tunnels(client_id);

CREATE TABLE IF NOT EXISTS tunnel_visitors (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    tunnel_id    INTEGER NOT NULL REFERENCES tunnels(id) ON DELETE CASCADE,
    client_id    INTEGER NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    local_listen INTEGER NOT NULL,
    status       TEXT    NOT NULL DEFAULT 'paused'
                    CHECK (status IN ('active', 'paused')),
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tunnel_id, client_id)
);

CREATE INDEX IF NOT EXISTS idx_tunnel_visitors_tunnel ON tunnel_visitors(tunnel_id);
CREATE INDEX IF NOT EXISTS idx_tunnel_visitors_client ON tunnel_visitors(client_id);
