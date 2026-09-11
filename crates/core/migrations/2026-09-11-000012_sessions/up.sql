-- Who is acting on a request (M4 T2). The launch token says the caller is on
-- this machine; a session says which person is at the keyboard, and until
-- this table existed every route wrote the seeded owner's id.
--
-- One row per sign-in, not one per user: the same person can be at the till
-- and at the office screen, and closing one must not close the other. A row
-- is never deleted while it can still be read back against a document, so a
-- sign-out fills `ended_at` and leaves the row where it is.

CREATE TABLE sessions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id      INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    user_id      INTEGER NOT NULL REFERENCES users(id),

    -- The token is never stored. What is stored is SHA-256 of it, hex, the
    -- way a password's hash is stored and for the same reason: a copy of the
    -- shop file, or a backup of it, must not hand somebody a live session.
    --
    -- SHA-256 and not the argon2id `services::users` hashes a PIN with, which
    -- is a deliberate difference and the one place this file departs from
    -- "stored the way a password is". A PIN is at most a million
    -- combinations, so it needs a slow hash to survive a stolen file; this
    -- token is 32 bytes off the OS random source, which no amount of guessing
    -- reaches, and a salted argon2 string cannot be looked up by, so every
    -- request would verify every live session in turn at 19 MiB a go. The
    -- service still compares the found row's hash in constant time.
    token_hash   TEXT NOT NULL,

    created_at   TEXT NOT NULL,
    -- The last request this session carried, UTC. The idle time is measured
    -- from here and this column is rewritten on every request, so a session
    -- dies a settings-held stretch after the last thing its user did rather
    -- than a fixed time after they signed in.
    --
    -- UTC and not the shop's calendar, for the reason the lockout columns on
    -- `users` give: this is a delay measured in minutes and never a date
    -- anybody reads off a document, and the two hands have to be on the same
    -- clock or a session expires an hour early.
    last_seen_at TEXT NOT NULL,
    -- When the user signed out, NULL while the session is live. A row rather
    -- than a delete so a session that was used can still be accounted for,
    -- and so "signed out" and "never existed" are two different answers.
    ended_at     TEXT
) STRICT;

-- Every request looks a session up by its shop and this hash and by nothing
-- else, so the pair is the unique key rather than an ordinary index: two rows
-- of one shop under one hash would be a collision in SHA-256 or a bug in the
-- service, and the file should refuse both. Scoped by shop rather than global
-- for the reason every query in `repos` is (rule 3): a token is only ever
-- looked up inside the shop this server answers for, so uniqueness across
-- shops would be a promise nothing reads.
CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions (shop_id, token_hash);

-- Signing a user out of every screen at once, and the T8 case of a fiche
-- being switched off while its owner is still holding a session.
CREATE INDEX idx_sessions_shop_user ON sessions (shop_id, user_id);
