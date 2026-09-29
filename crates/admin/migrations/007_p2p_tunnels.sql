-- gostc-rs: dedicated P2P tunnel type (gostc-style, self-built). Version 7.
--
-- Adds 'p2p' to the tunnel type CHECK. P2P tunnels are the gostc-aligned
-- "both sides run a client" direct-connection tunnels (frp xtcp semantics):
-- the service-side client registers node + intranet target + vKey (stored
-- hashed in sk_hash), the visitor side opens a local listener and connects
-- through NAT traversal (STUN + UDP hole punching, EasyTier-principle
-- reference) with node relay fallback. Unlike stcp/sudp (relay-first),
-- p2p prefers direct connections and never occupies a public port.

CREATE TABLE tunnels_new (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id     INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    client_id   INTEGER,
    name        TEXT    NOT NULL,
    type        TEXT    NOT NULL CHECK (type IN ('tcp', 'udp', 'http', 'https',
                                                'stcp', 'sudp', 'p2p')),
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
                         remote_port, domain, token, sk_hash, status, created_at)
SELECT id, user_id, node_id, client_id, name, type, local_addr,
       remote_port, domain, token, sk_hash, status, created_at
FROM tunnels;

DROP TABLE tunnels;
ALTER TABLE tunnels_new RENAME TO tunnels;

CREATE INDEX IF NOT EXISTS idx_tunnels_user_id ON tunnels(user_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_node_id ON tunnels(node_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_client_id ON tunnels(client_id);
