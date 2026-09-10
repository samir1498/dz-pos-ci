# Roadmap: from the money core to a first shop

Seven milestones, in order. Each one names what Anouar can demo when it
closes, what goes in, the fiscal fixtures it leans on (rows in
`features.md`), and what blocks it. No dates: a milestone closes when its
demo runs on a real machine and Samir has reviewed every PR in it. Steps
inside the current milestone live in `context/progress/now.md`; this page
never copies them.

Ordering decision (Samir, 2026-09-08, logged in `context/progress/now.md`):
a ticket sale ships before customers and debt, because a cash sale with a
printed ticket is the smallest loop a shop can run. `features.md` "Build
order" follows this page.

## M0. Money core and the first real screen (closed 2026-09-09, PR #14)

Demo: the products list on the laptop, read from a real SQLite file through
`crates/api`.

In: `Money` newtype and `pct`; TVA grouped per rate and the global discount
spread, whose allocation rule and fixture get written into `features.md`
before the code (the spec defines `subtotal_ht_at_rate` only for the
undiscounted case today); stamp duty per the 2025 tranche rule; amount in
words fr/ar/en with golden files; the first migration with `shop_id`, a
product price model that serves both regimes (IFU single price, réel HT
plus rate), settings including the dated régime fiscal, and one owner user
seeded so every later row has an author; `crates/api` with GET/POST
/products; `packages/shared` with the ts-rs types; the products screen
reading the API, which is also where the Anthropic `webapp-testing` and
`frontend-design` skills get their trial (R9, first half).

Fixtures: `money_no_float`, `tva_rounding_once_per_rate`,
`stamp_progressive_tranches`, `words_{fr,ar,en}_golden`.

Blocks: nothing. The half-dinar case at the 1,5 DA band stays an open
fixture until the accountant answers (R8).

## M1. A sale and a ticket on one desktop (in flight)

Demo: Anouar sells three products at the till, cash or card on a payment
terminal (TPE), and an 80 mm ticket comes out of a real thermal printer.
Stock goes down.

In: products CRUD (list, drawer, barcode, category, unit, TVA rate,
low-stock threshold); the till (lines, line and global discount, amount
tendered, change); the document model designed once with every kind, only
`ticket` issued here, numbered in its own gapless series from the first
sale; the stock movements ledger (opening, sale, adjustment); the user on
every document and ledger row (the seeded owner until M4) and the
append-only audit log for price changes, discount overrides, deletions and
settings changes; the store block in settings with RC, NIF, NIS, AI; the
régime fiscal control in settings; `ticket_80mm` golden files in three
languages, each with and without the stamp line (cash versus card);
ESC/POS over USB from the desktop; three languages on every screen, RTL
from the first one; daily backup, keep 30, restore from settings. The
first native window on the laptop is where the Tauri MCP trial (R9) runs.

Tests: `two_tickets_take_the_number_after_the_last` and
`a_refused_line_burns_no_number` for the ticket series;
`an_ifu_ticket_names_no_tax_in_any_language`; `a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`
(seller identity only).

Blocks: R3. The rule that picks facture or ticket at the till exists only
in the mockup today and its legal source (loi 04-02 art. 10 as amended in
2010) is unread; the article gets read, then the rule is written into
`features.md` §3, before the till codes it.

## M2. Facture, customers and credit

Demo: a company customer buys on credit and gets a numbered A4 facture with
the amount in words; pays part of it a week later; the statement shows the
balance. The facture carries no stamp line at issue: the stamp is cash
only (`stamp_progressive_tranches`), and a credit sale pays nothing at
issue.

In: customers (fiche; identifiers required for a company at issue); the
debt ledger, append-only, oldest-first settlement, credit limit warn and
block, the override reserved to the owner until M4 brings permissions;
facture A4 and A5; avoir as its own kind with its own series; a cancelled
facture keeps its number and reprints with the mention "facture annulée",
as a golden file; proforma; numbering per kind, gapless; `statement_a4`.

Tests: `each_kind_counts_in_its_own_series` and
`the_proforma_and_the_facture_series_do_not_touch` for the series;
`a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number`; the three words golden files.

Blocks: a real printed facture from an Algerian shop before the golden
files freeze (open decision 4); a native speaker's review of the Arabic
words file (R6); the accountant's answers (R8) on rounding practice, the
half-dinar, whether the yearly reset of the series is practice or habit,
and two questions added on 2026-09-08 that no source read so far answers:
when a credit facture is settled in cash later, does that payment need a
stamped receipt, and on which amount is the stamp computed, the facture's
`total_ttc` or the sum paid. Depending on the answer, a receipt kind
(quittance) joins the document model here.

## M3. Stock in, expenses, reports

Demo: a purchase from a supplier lands stock and supplier debt; the
dashboard shows today's sales, gross margin, cash position, low stock and
both debts.

In: suppliers; purchases with partial receipt, transport and extra costs,
the unpaid part to supplier debt; `bon_de_reception`, stored and listed,
not printed; expenses with the seeded categories; the nightly re-derive of
quantity on hand with a drift report; dashboard and reports; Excel export
and the product import with its template; `barcode_label`.

Blocks: nothing fiscal.

## M4. Team

Demo: a cashier signs in with a PIN, cannot see cost or override a credit
block; the owner reads the audit log of a price change.

In: owner, manager, cashier; PIN on the till, password elsewhere; the
permission list from `features.md` §5 applied to the document and ledger
rows that have carried a user since M1; the credit-block override moves
from owner-only to the permission.

Blocks: nothing.

## M5. First release, v1.0

Demo: Anouar installs from a signed Windows installer, reads version, git
hash and build date in About, updates in place, and a first shop runs on it.

In: the bundle identifier changed once, with the final name; the Tauri
updater and its signing key; the Windows code-signing certificate; the
previous-version migration test enforced from the first tag onward; an
Arabic and RTL polish pass; and the five release questions listed as open
in `docs/architecture.md` § Release (version in the log header and
support bundle, backup before each migration, key holder, tag-only
builds) answered and built as answered.

Release gates, all of them: every fiscal row in `features.md` confirmed by
a comptable (R8); a real printed facture seen (open decision 4); the Arabic
words file reviewed by a native speaker (R6); the article that puts NIF and
AI on a facture cited (R3); Sonar quoted as a gate only if Rust support on
the team server was verified first.

Blocks: the final name (Anouar); the certificate's cost and lead time
(Anouar); who holds the updater signing key (Anouar or Samir, decided
before the key exists).

## M6. The phone in the shop

Demo: a phone pairs by QR, sells from the shop floor, and the ticket prints
on the desktop.

In: LAN mode, one desktop serves; mDNS discovery; pairing by QR with a
short-lived token (the mockup shows 60 seconds); the Windows Firewall
banner; the Expo thin client (pair, till, cart, pay, ticket, products,
customers, more) with its retry queue; printing through the desktop;
Maestro flows on a real phone over Tailscale. The Expo plugin and the
Expo MCP (R9) are installed here, not before. Three controls proposed
here and written into `features.md` §6 when the milestone starts: the
token is single-use, only an owner or manager shows the QR, and settings
list paired devices with a revoke.

Blocks: whether LAN traffic needs TLS or the Wi-Fi is trusted; decided in
`docs/architecture.md` before a second device writes to the ledger.

## After M6, when Anouar decides

Cloud mode waits on open decision 1 (SaaS with an account, offline licence,
or both). When it comes, a tested backup and restore path exists first,
and the sync design (one direction, the server's answer wins, as the LAN
mode already does) gets its own page. Not scheduled.

## Parked

The spec lists these as later; the data model must not forbid them.
Batches and lots with expiry, variants, bundles, promotions. AI invoice
scanning on the phone. Phone-only offline through the Rust core compiled
into the app (uniffi). Cheque and transfer as payment modes. Bon de
livraison together with the facture récapitulative, since décret 05-468
art. 14 to 17 allow the first only with the second and a wilaya
authorisation.

## Every milestone

ISO 27001 controls ship with each feature (Anouar, 2026-07-13). Three
languages on every screen and every printed document. A golden file per
template and language; a change is a reviewed diff. Migrations forward-only
once released, each with a previous-version test. Money, roles and anything
that deletes data get the real-thing run and the `dz-review` pass before
merge, in M1 as much as in M4. A review before every merge into the
milestone branch; checkpoint PRs to `main` at the tasks the plan marks,
which Samir merges.
