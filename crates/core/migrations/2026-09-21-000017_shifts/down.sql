-- Back to UTC, the other half of the expense clock shift. A build that stamps
-- from the shop clock writing into a file this has reversed would be an hour
-- out, so going back means going back to a build from before the change too.
UPDATE expenses SET created_at = datetime(created_at, '-1 hour');

-- The indexes go with the table they are on; naming them is what keeps the
-- reverse readable beside the up.
DROP INDEX idx_shifts_shop_opened_at;
DROP INDEX idx_shifts_one_open_per_user;
DROP TABLE shifts;
