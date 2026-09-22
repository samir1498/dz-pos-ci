-- The indexes go with the table they are on; naming them is what keeps the
-- reverse readable beside the up.
DROP INDEX idx_sessions_shop_user;
DROP INDEX idx_sessions_token_hash;
DROP TABLE sessions;
