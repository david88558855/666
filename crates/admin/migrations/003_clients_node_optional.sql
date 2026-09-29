-- gostc-rs: make clients.node_id optional.
-- Clients now connect to the central console directly (gostc-style), so a
-- client does not need to be bound to a node at creation time. The binding
-- happens per-tunnel (tunnel selects client + node). Version 3.
-- SQLite cannot ALTER a NOT NULL column, so rebuild the table.

CREATE TABLE clients_new (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id      INTEGER REFERENCES nodes(id) ON DELETE CASCADE,
    name         TEXT    NOT NULL,
    token        TEXT    NOT NULL UNIQUE,
    status       TEXT    NOT NULL DEFAULT 'active'
                     CHECK (status IN ('active', 'paused')),
    last_online  TIMESTAMP,
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO clients_new (id, user_id, node_id, name, token, status, last_online, created_at)
    SELECT id, user_id, node_id, name, token, status, last_online, created_at FROM clients;

DROP TABLE clients;
ALTER TABLE clients_new RENAME TO clients;

CREATE INDEX IF NOT EXISTS idx_clients_user_id ON clients(user_id);
CREATE INDEX IF NOT EXISTS idx_clients_node_id ON clients(node_id);
