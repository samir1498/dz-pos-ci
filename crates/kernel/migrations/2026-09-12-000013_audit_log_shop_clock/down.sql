-- Back to UTC, the other half of the shift. A build that stamps from the
-- shop clock writing into a file this has reversed would be an hour out, so
-- going back means going back to a build from before the change too.
UPDATE audit_log SET created_at = datetime(created_at, '-1 hour');
