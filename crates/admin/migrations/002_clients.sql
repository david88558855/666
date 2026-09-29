-- gostc-rs: client entities (gostc-style UX)
-- A client is a credential entity running on an intranet machine; one client
-- command serves ALL tunnels configured for it in the panel. Version 2.

CREATE TABLE IF NOT EXISTS clients (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id      INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    name         TEXT    NOT NULL,
    token        TEXT    NOT NULL UNIQUE,
    status       TEXT    NOT NULL DEFAULT 'active'
                     CHECK (status IN ('active', 'paused')),
    last_online  TIMESTAMP,
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_clients_user_id ON clients(user_id);
CREATE INDEX IF NOT EXISTS idx_clients_node_id ON clients(node_id);

-- Link tunnels to the client that serves them. SQLite cannot add an FK via
-- ALTER TABLE, so cascading on client delete is handled in the API layer.
ALTER TABLE tunnels ADD COLUMN client_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_tunnels_client_id ON tunnels(client_id);
