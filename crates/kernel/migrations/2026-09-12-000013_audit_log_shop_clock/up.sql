-- Every row in this table took its moment from the column's own default,
-- SQLite's CURRENT_TIMESTAMP, which is UTC. Most created_at columns in this
-- file carry that same default, and on the ones a day filter reads it never
-- fires: the service stamps from services::clock before the insert, and the
-- supplier ledger refuses a row that arrives without a moment. The audit log
-- was the one column read by a filter and left to the default, and the shop
-- runs on UTC+1 with no daylight saving, so a row written at 00:30 in
-- Algiers was stored as 23:30 the day before, printed that way on the
-- owner's screen, and fell outside the day he filtered for.
--
-- The service stamps the row from services::clock from now on, which is why
-- this shift is a one-off. It has to travel in the same version as that
-- change: run against a build that already stamps, it would move those rows
-- an hour into the future.
UPDATE audit_log SET created_at = datetime(created_at, '+1 hour');

-- The default stays on the column and can no longer fire: AuditRowWrite
-- carries created_at as a plain NaiveDateTime, so there is no insert that
-- omits it. Dropping a default in SQLite means rebuilding the table, which
-- is a bigger risk than the one it removes.
