-- The four statements of `up.sql`, undone back to front.

-- The series strings, back to the key without a year.
UPDATE documents SET series = substr(series, 1, instr(series, ':') - 1)
WHERE instr(series, ':') > 0;

-- The counter names, back to the same key. A counter of a series no document
-- has used is not one this migration renamed, so it is not one to rename back;
-- the EXISTS is what tells the two apart, on the series strings the statement
-- above has just put back.
UPDATE counters
SET name = substr(name, 1, instr(name, ':') - 1)
WHERE instr(name, ':') > 0 AND name LIKE 'doc\_%' ESCAPE '\';

ALTER TABLE documents DROP COLUMN series_year;
