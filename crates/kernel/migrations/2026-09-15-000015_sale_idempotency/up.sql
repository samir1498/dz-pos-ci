-- M7 T4: idempotent sale ring. A client key per sale (the phone's queue
-- item id), mapped to the document it rang, so a retry after a lost answer
-- returns the original instead of ringing twice. UNIQUE on (shop, key) is
-- the race guard: two same-key rings serialize and the loser re-reads.
-- request_hash fingerprints the ring the key was first seen with; the same
-- key on a different ring is a client bug answered 422, never a neighbor's
-- sale handed back in silence.

CREATE TABLE sale_idempotency_keys (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id      INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    key          TEXT NOT NULL,
    sale_id      INTEGER NOT NULL REFERENCES documents(id),
    request_hash TEXT NOT NULL,
    created_at   TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX sale_idempotency_keys_shop_key ON sale_idempotency_keys(shop_id, key);
