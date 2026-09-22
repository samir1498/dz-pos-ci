-- M6 T2: QR pairing (60s single-use) and paired devices (long-lived, revocable).
-- A pairing token is 32 random bytes, stored as SHA-256 hex, 60s expiry,
-- single-use; a phone trades it for a device token that lives until revoked.

CREATE TABLE pairing_tokens (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id      INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    used_at      TEXT,
    created_by   INTEGER NOT NULL REFERENCES users(id)
) STRICT;
CREATE INDEX idx_pairing_tokens_shop_hash ON pairing_tokens(shop_id, token_hash);
CREATE INDEX idx_pairing_tokens_expires ON pairing_tokens(expires_at);

CREATE TABLE paired_devices (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id      INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    last_seen_at TEXT,
    revoked_at   TEXT,
    created_by   INTEGER NOT NULL REFERENCES users(id)
) STRICT;
CREATE INDEX idx_paired_devices_shop ON paired_devices(shop_id);
