-- What a credit note leaves behind, on both sides of it (features.md §3):
-- what a cancellation wrote on the document it annulled, and which line of a
-- facture each line of an avoir credits.
--
-- A cancelled document keeps its number and its row so the series never gaps
-- (décret 05-468 art. 10), which the `status` column has said since the first
-- migration. What it could not say is when it was annulled, by whom, why, and
-- which avoir carried the money back. All four are what the annulée face of
-- the paper prints and what a comptable is owed when a numbered document
-- stops asking for its amount.
--
-- Nullable, and null on every document that still stands. The four are
-- written together or not at all; the model reads a half-written block back
-- as an error the way it already does for the buyer block and the balance
-- triple, rather than the table hanging a CHECK across its own columns, which
-- in SQLite would mean rebuilding a table that holds every facture the shop
-- has issued.
--
-- Four added columns and no rebuild, so this migration runs inside the
-- transaction diesel opens for it and opens none of its own.

-- The moment on the shop's calendar, like `issued_at` beside it and never the
-- column default's UTC: one hour a day the two would disagree about which day
-- a facture stopped standing.
ALTER TABLE documents ADD COLUMN cancelled_at TEXT;

-- Who took the decision. RESTRICT by default like `user_id` above it: a user
-- row is not deleted out from under a document that names them.
ALTER TABLE documents ADD COLUMN cancelled_by INTEGER REFERENCES users(id);

-- Why, in the words the operator typed. Not a code: the reasons a shop
-- cancels a facture are its own, and a list would be wrong by the second
-- shop.
ALTER TABLE documents ADD COLUMN cancel_reason TEXT;

-- The avoir the cancellation issued, when it issued one. A cancelled cash
-- ticket carries none: the stock goes back and no money was ever owed, so
-- there is nothing to write a credit note for.
ALTER TABLE documents ADD COLUMN cancel_avoir_document_id INTEGER REFERENCES documents(id);

-- The facture line one avoir line credits. A partial avoir may be written
-- against a facture more than once, and what is left to credit on a line is
-- its quantity less what earlier avoirs took off that same line: matching the
-- two by product would go wrong the first time a facture carries one product
-- on two lines, which is ordinary as soon as a line discount or a second
-- price is involved.
--
-- Null on every line that credits nothing, which is every line of a ticket, a
-- facture and a proforma.
ALTER TABLE document_lines ADD COLUMN ref_line_id INTEGER REFERENCES document_lines(id);
