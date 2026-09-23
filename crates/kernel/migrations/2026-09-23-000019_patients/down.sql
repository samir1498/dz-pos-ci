-- The indexes go with the table they are on; naming them is what keeps the
-- reverse readable beside the up. Nothing else moves: the up added a table
-- and touched no row of any other, so this is the whole of the reverse.
DROP INDEX idx_patients_shop_phone;
DROP INDEX idx_patients_shop_name;
DROP TABLE patients;
