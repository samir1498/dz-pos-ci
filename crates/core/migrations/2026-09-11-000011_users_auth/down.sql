-- The index first: SQLite refuses to drop a column an index names.
DROP INDEX idx_users_shop_name;
-- Reverse order of the up, and every one of them a column this migration
-- added: `users` itself, its id, name, role and `pin_hash` belong to the first
-- migration and stay. A column-level CHECK on the column being dropped is not
-- in SQLite's way (3.35 and above; libsqlite3-sys is pinned bundled).
ALTER TABLE users DROP COLUMN updated_at;
ALTER TABLE users DROP COLUMN locked_until;
ALTER TABLE users DROP COLUMN pin_failures;
ALTER TABLE users DROP COLUMN active;
ALTER TABLE users DROP COLUMN password_hash;
