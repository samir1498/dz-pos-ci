-- The up added two columns and moved no row; the calls recorded go with
-- them. `call_at` first: its CHECK names `call_outcome`, and SQLite refuses
-- to drop a column another column's CHECK names.
ALTER TABLE appointments DROP COLUMN call_at;
ALTER TABLE appointments DROP COLUMN call_outcome;
