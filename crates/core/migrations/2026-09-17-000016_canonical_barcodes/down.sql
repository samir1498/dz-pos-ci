-- Nothing to undo: the thirteen-digit form is the same article's number and
-- a twelve-digit one cannot be recovered from it without knowing which rows
-- this migration touched. Dropping the leading zero from every EAN-13 that
-- has one would rewrite codes that were always thirteen digits.
SELECT 1;
