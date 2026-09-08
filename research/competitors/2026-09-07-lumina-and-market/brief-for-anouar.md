# Algerian POS project — where we are after day one

Samir, 2026-09-07. Everything below comes from pulling apart the Lumina files
you shared and from checking the market while looking for a name. Detailed
write-ups sit next to this file in the research repo.

## Short version

- Lumina POS is a real, finished product, not a mockup. One developer in Aïn
  Beïda sells it for 12,000 DZD one-time over the phone.
- We have its complete Algerian invoice specification now, pulled from inside
  the installer. That was going to be the slowest part to figure out.
- The market is much bigger than Lumina. At least a dozen Algerian products do
  this, and one of them is sold through Algérie Télécom.
- MonStock gives us the stock and sales half. The invoicing, customers, debt
  and printing half has to be built.
- Tech decisions are made and written down. The build can start the day you
  answer the questions at the bottom.

## What Lumina is

A Windows desktop app plus an Android app, in Arabic on desktop and French on
the phone. It handles stock, sales, purchases, suppliers, customers with credit,
bundles, promotions, proforma invoices, delivery orders and reports. It runs
offline by default; a shop can also link several tills on one Wi-Fi network,
or sync to the cloud.

What it gets right, and what customers are paying for:

- Every printed document carries the legal fields (RC, NIF, NIS, AI) for both
  the shop and the customer, TVA, stamp duty, and the total written in words.
  It ships more than fifty print layouts in Arabic, French and English.
- Debt tracking. Who owes what, with credit limits and warnings.
- It works without internet, and it prints to cheap thermal printers.
- The phone app can photograph a supplier's invoice and pull the products out
  of it automatically. Nothing else in the product is this modern.

What it gets wrong:

- TVA is one global rate, not per product.
- Buying it means filling a form and waiting for a phone call.
- Desktop is Arabic only, phone is French only. That's an accident, not a plan.

The recording you made was of an empty install, so it never showed the till
screen or a printed invoice. Unpacking the installer got us those anyway.

## The market

Searching for a product name turned up the competition, and there is a lot of
it. Almawarid, AlgoStock, Factury (free tier), FooRa, GestiumPRO, COMMSoft,
SmartCom, Motakamel, and Fatoura. **Fatoura is listed as a product on Algérie
Télécom's website.** If that channel is reachable for us, it matters more than
anything in the software.

Two kinds of competitor: modern web and mobile apps that do content marketing
(Almawarid, Fatoura, Factury), and older desktop vendors selling through
resellers. Lumina sits in between: modern stack, phone-call sales.

So the question is not "how do we beat Lumina", it is "why would a shop pick us
over a dozen options, one of which is free". I don't have that answer. You
might.

## What we already have

MonStock, the Rust inventory app I built earlier: products, suppliers,
purchase orders, sales, expenses, a profit/loss dashboard, barcodes, backups,
French and English. Clean code, about 8,000 lines.

What it does not have, in the order we'd build it: customers and a debt ledger,
invoices with the legal fields, a print engine for thermal and A4, users and
roles, Arabic with right-to-left layout, several devices on one network.

That is the full list. None of it is exotic, and Lumina's own templates tell
us exactly what the invoice needs to look like.

## Technical decisions, already made

Written up properly in `architecture-notes.md`; the short form:

- Desktop stays on Rust with a web-style interface (Tauri). Phone is React
  Native. The current Rust desktop interface gets dropped; it was too painful
  to style.
- The core logic sits behind one internal API from the start, so "runs on the
  shop's PC" and "runs on our server" is a deployment choice, not two products.
  Your SaaS-or-offline answer does not change the first weeks of work.
- Money is stored as whole centimes. Never decimals. This gets fixed before any
  invoice code is written.
- The phone is never a second source of truth. One machine owns the numbers;
  the phone talks to it. Lumina lets both sides edit while offline and
  reconcile later, and that is where their bugs live. We won't.
- Sonar, coverage gates and the testing setup you asked for are planned from
  the first commit, on both the Rust and the JavaScript sides. Same rules as
  ObserveOne.

## Names

Every name I liked is taken by a product in this exact space: Daftar, Hanout,
Mizan, Zimam, Qayd, Tijara, Hisab. Each one is a live app doing invoicing,
POS or accounting, several of them Algerian. The only survivor was "Sijil"
(register), which is available but not memorable. We need a name from you, or
a second round of ideas from both of us. Until then the working name is dz-pos.

## What I need from you

1. **SaaS with an account, offline licence, or both?** This decides pricing
   and whether we host anything. Lumina charges 12,000 DZD once and never
   again. Factury is free.
2. **A name**, or the go-ahead to keep hunting.
3. **New GitHub org or under an existing one?** Repo starts fresh either way;
   we copy in what we keep from MonStock.
4. **Who sells this and to whom.** MonStock had five releases and zero
   feedback because nobody was selling it. Lumina's whole advantage is a phone
   number in Oum El Bouaghi. A first shop willing to use it matters more than
   any feature.
5. **Is Algérie Télécom's product listing something we can reach?**
6. **One printed facture** from any shop that uses any of these products. I
   have the field list; I'd like to see a real one on paper.

Questions 1 and 2 are blocking. The rest can wait a week.
