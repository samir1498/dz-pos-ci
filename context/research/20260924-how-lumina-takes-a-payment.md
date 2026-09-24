---
title: 'How Lumina takes a payment, and its till shortcuts'
slug: 'how-lumina-takes-a-payment'
status: 'active'
category: 'research'
created: 20260924
tldr: 'Read out of Lumina POS source (~/lumina/src/renderer.js, deobfuscated, and src/index.html): F1 with no customer sells at the exact total with no dialog; with a customer a pay dialog opens pre-filled with the total; F1..F12 map; focus handling; no quick-cash notes.'
---
# How Lumina takes a payment

Read on 2026-09-24 from Lumina's own code, for the till redesign
(`context/plans/20260924-shop-manual-test-findings.md`, T11, T17 to T21).
Sources: `~/lumina/src/index.html` (the in-app shortcuts sheet at lines
10024-10142, the checkout modal from ~8408) and `~/lumina/src/renderer.js`
(obfuscator.io, strings decoded with the file's own decoder in a Node vm).
The screen-recording frames do not show the payment dialog, so this rests
on code only.

## Paying

- F1 with no customer on the cart sells at once: `processSaleDirectly()`
  posts the sale with paid = total, no dialog, no change. A "quick sale
  without a customer?" confirm exists but its setting ships ticked to skip
  it (`index.html:2317`, `settSkipQuickSaleConfirm ... checked`).
- F1 with a customer opens the checkout dialog with "amount received"
  pre-filled with the exact total, method cash, change shown live (green),
  or "debt" in red with the shortfall when paid is below the total.
- Inside the dialog, F1 or the confirm button takes the sale. Enter does
  nothing. Only F1 works while the dialog is open.
- One button "full debt" sets received to 0. A separate "confirm and
  print" button sits beside confirm; F5 toggles auto-print.
- One payment method per sale (cash, cheque, CCP, virement, card, other).
  No split payment, no quick-cash note buttons.
- Stamp duty shows only for cash, computed `max(5, min(2500, total*1%))`.

## Shortcuts

| Key | Action |
|---|---|
| F1 | pay (see above) |
| F2 | park the cart as a draft |
| F3 | open drafts |
| F4 | empty the cart (not while typing) |
| F5 | toggle auto-print |
| F6 | featured products |
| F7 | focus product search |
| F8 | free/custom product |
| F9 / F10 | step back / forward through recent sales to edit |
| F11 | customer search |
| F12 | quantity of the first cart line |
| + / - | quantity of the selected line (0.1 step for weighed units) |
| Esc | open the cash drawer (no dialog open, not typing) |
| Enter in the scan box | add the scanned or typed code |

The shortcuts sheet is in the app, reachable from the UI.

## Focus

The scan box gets focus when the sales screen opens (re-asserted at 50 ms
and 250 ms), after picking a search result (unless a quantity or variant
prompt opened), and after a fast scan. Nothing steals focus back
continuously, and nothing refocuses it after a sale completes.

## What to take for Dinar

A no-customer cash sale should be one key (exact amount, no typing), the
pay step pre-fills the total, every action has a key shown on screen, and
focus returns to the scan box after every action including a finished sale,
which Lumina misses.
