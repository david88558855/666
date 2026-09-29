-- gostc-rs: per-node transport protocol selection.
-- Each node fixes the transport used by tunnel-clients to reach it,
-- mirroring orbien's [transport] protocol option (tcp / quic / websocket /
-- kcp). Defaults to tcp. Version 4.

ALTER TABLE nodes ADD COLUMN transport TEXT NOT NULL DEFAULT 'tcp'
    CHECK (transport IN ('tcp', 'quic', 'websocket', 'kcp'));
