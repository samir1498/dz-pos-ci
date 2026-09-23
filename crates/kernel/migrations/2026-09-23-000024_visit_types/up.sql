-- Visit types, the third of the book tools (C5b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md):
-- a name and a length ("première consultation", 30 minutes). A booking that
-- names one takes its length; the grid every start sits on stays the
-- slot-length setting. The appointment copies the length into its own
-- `slot_minutes` and keeps no pointer here, so editing or removing a type
-- changes no booking already made and needs no rebuild of `appointments`.
--
-- In the one shared migration list, like the other clinic tables. Additive,
-- so the revert is a DROP.

CREATE TABLE visit_types (
    -- A UUID v7 as its 36-character text, made by the clinic service, for
    -- the reason `patients.id` gives.
    id          TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- The kernel's tenant. RESTRICT, with every other table.
    shop_id     INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- What the desk picks from, trimmed by the service.
    name        TEXT NOT NULL CHECK (length(trim(name)) > 0 AND length(name) <= 60),
    -- 5 to 240 in steps of 5: 240 is `appointments.slot_minutes`' own
    -- ceiling. That it is a multiple of the grid in force is the service's
    -- check, the grid being a setting and not a column.
    minutes     INTEGER NOT NULL CHECK (minutes BETWEEN 5 AND 240 AND minutes % 5 = 0),
    -- The shop's clock, no DEFAULT, for the reason `patients` gives.
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

-- One type per name per shop, whatever the case of its ASCII letters: the
-- desk picks by name, and two "Contrôle" would be one too many.
CREATE UNIQUE INDEX idx_visit_types_shop_name ON visit_types (shop_id, name COLLATE NOCASE);
