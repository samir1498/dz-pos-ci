# Roadmap: from the money core to a first shop

Seven milestones, in order. Each one names what the owner can demo when it
closes, what goes in, the fiscal fixtures it leans on (rows in
`features.md`), and what blocks it. No dates: a milestone closes when its
demo runs on a real machine and every PR in it went through the review
lenses and merged green. Steps
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

## M1. A sale and a ticket on one desktop (closed 2026-09-09; live thermal print dropped 2026-09-13)

Demo: The owner sells three products at the till, cash or card on a payment
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
three languages on every screen, RTL from the first one; daily backup,
keep 30, restore from settings. Live ESC/POS over USB was dropped on
2026-09-13: there is no thermal printer. The first native window on the
laptop is where the Tauri MCP trial (R9) runs.

Tests: `two_tickets_take_the_number_after_the_last` and
`a_refused_line_burns_no_number` for the ticket series;
`an_ifu_ticket_names_no_tax_in_any_language`; `a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`
(seller identity only).

Blocks: R3. The rule that picks facture or ticket at the till exists only
in the mockup today and its legal source (loi 04-02 art. 10 as amended in
2010) is unread; the article gets read, then the rule is written into
`features.md` §3, before the till codes it.

## M2. Facture, customers and credit (closed 2026-09-10)

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

## M3. Stock in, expenses, reports (closed 2026-09-10)

Demo: a purchase from a supplier lands stock and supplier debt; the
dashboard shows today's sales, gross margin, cash position, low stock and
both debts.

In: suppliers; purchases with partial receipt, transport and extra costs,
the unpaid part to supplier debt; `bon_de_reception`, stored and listed,
not printed; expenses with the seeded categories; the nightly re-derive of
quantity on hand with a drift report; dashboard and reports; Excel export
and the product import with its template; `barcode_label`.

Blocks: nothing fiscal.

## M4. Team (closed 2026-09-11, PR #33)

Demo: a cashier signs in with a PIN, cannot see cost or override a credit
block; the owner reads the audit log of a price change.

In: owner, manager, cashier; PIN on the till, password elsewhere; the
permission list from `features.md` §5 applied to the document and ledger
rows that have carried a user since M1; the credit-block override moves
from owner-only to the permission.

Blocks: nothing.

## M5. First release, v1.0 (closed 2026-09-13)

Demo: The owner installs from a signed Windows installer, reads version, git
hash and build date in About, updates in place, and a first shop runs on it.

In: the bundle identifier is `com.dinar.app` (product name Dinar); the Tauri
updater and its signing key; the Windows code-signing certificate; the
previous-version migration test enforced from the first tag onward; an
Arabic and RTL polish pass; and the five release questions listed as open
in `docs/architecture.md` § Release (version in the log header and
support bundle, backup before each migration, key holder, tag-only
builds) answered and built as answered.

Not in, despite an earlier draft of this list saying so: the bon de
livraison. `docs/features.md` § Later parks it, and the reason is fiscal
rather than a matter of effort. Décret 05-468 articles 14 to 17 allow a
delivery note only together with a facture récapitulative and a wilaya
authorisation, so a shop that issued one on its own would be issuing a
document it may not. The `bon_de_livraison` kind stays in the model, with
no template and no screen, which is what the spec asks for and what the
code already does.

Release gates, all of them: every fiscal row in `features.md` confirmed by
a comptable (R8); a real printed facture seen (open decision 4); the Arabic
words file reviewed by a native speaker (R6); Sonar quoted as a gate only if
Rust support on the team server was verified first. The NIF citation (R3) is
closed: loi 04-02 art. 34 makes it a facture mention and no text makes the
article d'imposition one, which leaves the AI to the comptable under R8.

Blocks: the final name (Samir, decided 2026-09-13); the certificate's cost and lead time
(Samir); who holds the updater signing key (Samir, decided
before the key exists).

## M6. The phone in the shop (closed 2026-09-15, PR #83)

Demo: a phone pairs by QR (60s, `EditSettings`), sells from the shop floor
(`POST /sales` via `queue.ts` retry), and the ticket prints on the desktop
(`POST /sales/{id}/print` → `spool/ticket-*.bin` + optional `9100`).

In: LAN mode (`bind_lan` `0.0.0.0`, `mdns` `_dzpos._tcp`, `architecture.md`
decides HTTP on trusted Wi-Fi, TLS fingerprint stays candidate, Firewall
banner); QR pairing `60s` single-use (`pairing_tokens`/`paired_devices` STRICT,
`POST /pairing/qr` `EditSettings`, `POST /pairing/claim` no session, second
claim `401`); Expo thin client `dinar-mobile` (`App.tsx` → `Till.tsx`,
`pair•till•cart•pay•ticket•products•customers`, `queue.ts` `AsyncStorage`);
paired devices list+revoke `GET /pairing/devices` / `POST …/revoke`
(`EditSettings`, `Eighteen` reads, `device.revoked` audit, `PairedDeviceDto`);
print through desktop (`POST /sales/{id}/print` `Sell`, `spool/`); Maestro
`apps/mobile/maestro/pair-and-sell.yaml` over Tailscale; office password
reset `POST /users/{id}/password` (`ManageUsers`, same as PIN, `SetPasswordDto`);
Expo plugin + MCP still for later when mobile work begins (R9), but scaffold
already `pnpm -r build` as a real workspace package.

Blocks: none. TLS decision is `http` for v1, reversible via QR `fp`. Open unknowns: queue idempotency, device middleware, QR claim TOCTOU.

## M7. The paired phone, proven (closed 2026-09-15, PR #91 for T6)

Demo: pair a phone, sign in as cashier, kill the Wi-Fi mid-sale, retry
and the sale rings once (fingerprint dedupe, `200` replay); revoke the
phone mid-shift, its next call fails closed (`401` before session);
scan one QR twice at once, the second claim fails (conditional mark-used
in `BEGIN IMMEDIATE`). Phone carries launch+device+session on every call,
including retry; queue skips another signer's items.

In: atomic QR claim; device auth actually checked; a session story for
the phone so the desktop's permission gates apply unchanged; idempotent
sale ring with the print following a retried ring; Maestro proof on a
real phone over Tailscale. Full task list with the evidence behind each
in `context/plans/20260915-m7-paired-phone-proven.md`.

Blocks: none. The Tailscale proof is the demo — run it from the laptop
(`tmux dz`) with a phone on the same tailnet, as in M6 T7. No new money
rules.

## After M6

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
which the session opens and merges once CI is green (since 2026-09-09).
