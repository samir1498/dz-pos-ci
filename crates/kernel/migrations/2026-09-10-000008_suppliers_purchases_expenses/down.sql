-- The ten tables the up.sql adds, and nothing else: it rebuilt none of the
-- tables that were already there, so there is nothing here to put back.
--
-- Children before parents. With the foreign keys on, a DROP does the delete
-- its rows would need, and a parent dropped first fails on the rows still
-- pointing at it. The indexes go with their tables.
DROP TABLE purchase_receipt_lines;
DROP TABLE supplier_allocations;
DROP TABLE purchase_receipts;
DROP TABLE purchase_lines;
DROP TABLE supplier_ledger;
DROP TABLE purchases;
DROP TABLE suppliers;
DROP TABLE expenses;
DROP TABLE expense_categories;
DROP TABLE jobs;
