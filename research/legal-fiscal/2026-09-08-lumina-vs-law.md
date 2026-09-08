# Lumina versus the law, line by line (2026-09-08)

Task R7. One row for every legal mention and every calculation in
`research/competitors/2026-09-07-lumina-and-market/lumina-pos-teardown.md`.
Each row says what the law requires with its article, what Lumina does, and
what dz-pos does per `docs/features.md`.

Nothing in the teardown becomes source code. It is a competitor's proprietary
build, unpacked from an installer, and the repo rule in `research/README.md`
stands. This table is here so that a field we adopt is adopted because a text
requires it, not because Lumina printed it.

The headline: Lumina's field list is close to complete against décret 05-468,
and both of its calculations are wrong. The stamp formula is the pre-2025 one,
and a single global TVA rate cannot produce a correct total for a basket that
mixes 19 % and 9 % goods, which is every supérette basket that contains
vegetables.

## Calculations

| Item | Law | Lumina | dz-pos per `docs/features.md` |
|---|---|---|---|
| Droit de timbre, base | Code du timbre 2026 art. 100-I: tranches of 100 DA or fraction thereof, on the sum paid | `amount * 0.01`, a straight percentage of the net | Row "Droit de timbre": `tranches = ceil(amount / 100 DA)`, fixture `stamp_progressive_tranches` |
| Droit de timbre, rate | art. 100-I: 1 DA per tranche to 30 000 DA, 1,5 DA to 100 000 DA, 2 DA above, the whole amount at its band's rate. DGI circular 14/MF/DGI/LF.2025: "ce droit de timbre n'est pas soumis à une progressivité par tranche" | flat 1 % | the three bands, no progressivity, same fixture |
| Droit de timbre, floor | art. 100-I: "le montant du droit dû ne peut être inférieur à 5 DA" | 5 DZD floor | 5 DA floor, same |
| Droit de timbre, ceiling | no ceiling in art. 100-I | 2 500 DZD ceiling | none |
| Droit de timbre, exemption below 300 DA | art. 100-I opens its first band at "sommes dont le montant est supérieur à 300 DA", so the exemption is there by omission; the sentence "les sommes n'excédant pas 300 DA, ne donnent lieu à aucun [droit] à payer" is the circular's, 14/MF/DGI/LF.2025 p. 2, not the code's | none; a 100 DA cash sale still pays the 5 DA floor | nothing at or below 300 DA |
| Droit de timbre, payment mode | art. 258 quinquies: quittances settled by electronic means are exempt | `payment_mode === 'cash'` only | cash only, and the exemption is stated as the law's, not a choice |
| Droit de timbre, per-store toggle | no text lets a seller opt out of a duty | toggleable per store in settings | the payment mode decides the stamp, with no setting in between |
| TVA rate assignment | CTCA 2026 art. 21 (19 %) and art. 23 (9 %, listed by customs tariff line) | one global percentage for the whole store, set in Settings → store info as نسبة الضريبة العالمية (TVA %); the teardown shows no per-product rate | rate per product, defaulted from the category. Open decision 3, settled 2026-09-08. Row "TVA rates", fixture `tva_rates_table` |
| TVA arithmetic | no text prescribes how a facture rounds. CTCA art. 80bis sends the tax return to CIDTA art. 324, which is about the G50, not the document | not visible in the teardown, which records the global rate setting and the `TVA` and `TAX_TOTAL` template placeholders and says nothing about the arithmetic or the rounding | integer centimes, one rounding per rate group on the group's HT subtotal, half away from zero. Row "Rounding", fixture `tva_rounding_once_per_rate` |
| TVA base after discount | décret 05-468 art. 5: the TTC total includes every remise, rabais and ristourne agreed at the sale | applied to the discounted subtotal, which matches art. 5 in intent | `subtotal_ht = total_ht − discount`, then TVA per rate group |
| Global discount split across rate groups | no text | not applicable, one rate, so Lumina never has to split | pinned by the spec. Row "Global discount spread": the global discount is allocated to the rate groups in proportion to each group's HT subtotal, each share floored to the centime, the leftover centimes to the group with the largest HT subtotal, the lower rate winning a tie, fixture `discount_spread_largest_remainder`. A design choice, put to the comptable as question 7 of the comptable page |
| Amount in words | décret 05-468 art. 3: "prix total toutes taxes comprises, libellé en chiffres et en lettres" | the teardown shows only the `TOTAL_IN_WORDS` placeholder; `2026-09-08-fiscal-sources-and-findings.md` records a hand-written `numberToWords.js`, fr / ar / en, dinars and centimes, float input split with `Math.floor` and `Math.round` | three generators over integer centimes, fixture `words_{fr,ar,en}_golden`. The float split is exactly the bug class the money contract forbids |
| IFU regime | CTCA art. 2-12 puts IFU sellers outside the TVA field; art. 64 forbids them to mention TVA on a facture, on pain of art. 114 | no regime concept. An IFU shop that installs Lumina prints a TVA line and becomes personally liable for the tax | shop-level dated setting `ifu` or `réel`, fixture `regime_ifu_prints_no_tva` |

## Seller mentions

Décret 05-468 art. 3-1 fixes the list. Lumina's `invoice_template_1.html`
placeholders are in the second column.

| Mention required by art. 3-1 | Lumina | dz-pos |
|---|---|---|
| nom et prénom(s) or raison sociale | `COMPANY_NAME` | store settings, §3 seller block |
| adresse | `COMPANY_ADDRESS` | yes |
| numéros de téléphone et de fax, email | `COMPANY_PHONE`, `COMPANY_FAX`, `COMPANY_EMAIL` | yes |
| forme juridique et nature de l'activité | **absent** | **absent from §3 seller block** |
| capital social, le cas échéant | **absent** | **absent from §3 seller block** |
| numéro du registre du commerce | `COMPANY_RC` | RC |
| numéro d'identification statistique | `COMPANY_NIS` | NIS |
| NIF, required by loi 04-02 art. 34 | `COMPANY_NIF` | NIF |
| mode de paiement et date de règlement | `PAYMENT_MODE`, no settlement date | `payment_mode` in the totals table, no settlement date field |
| date d'établissement et numéro d'ordre | in the template header | numbering per kind per year, §3 |
| dénomination et quantité des biens | line items | lines carry product snapshot and quantity |
| prix unitaire hors taxes | line items | `unit price HT` on the line |
| prix total hors taxes | `TOTAL_HT` | `total_ht` |
| nature et taux des taxes | `TVA`, `TAX_TOTAL` | TVA per rate group |
| prix total TTC en chiffres et en lettres | `NET_TO_PAY`, `TOTAL_IN_WORDS` | `total_ttc`, `amount_in_words` computed on `net_to_pay` |

Two gaps are shared. Neither product prints the forme juridique, the nature de
l'activité or the capital social, and both omit the settlement date. Under loi
04-02 art. 34 those three fall in the 10 000 to 50 000 DA class rather than the
80 % class, so they are cheap to miss and still worth adding to the store
settings before the templates freeze.

One difference worth naming: Lumina's template lists `TOTAL_IN_WORDS` beside
`NET_TO_PAY`, which includes the stamp. Décret 05-468 art. 3 asks for the
"prix total toutes taxes comprises" in words. Whether the droit de timbre
belongs inside that figure is question 8 of
`2026-09-08-questions-comptable.md`, and `docs/features.md` currently takes
the same position as Lumina by defining `amount_in_words` on `net_to_pay`.

## Buyer mentions

| Mention required by art. 3-2 | Lumina | dz-pos |
|---|---|---|
| nom et prénom(s) or raison sociale | `PARTY_NAME` | buyer block, snapshotted at issue |
| forme juridique et nature de l'activité | **absent** | **absent** |
| adresse, téléphone, fax, email | `PARTY_ADDRESS` only | address only |
| numéro du registre du commerce | `PARTY_RC` | RC |
| numéro d'identification statistique | `PARTY_NIS` | NIS |
| NIF | `PARTY_NIF` | NIF |
| article d'imposition | `PARTY_AI` | AI |
| consumer buyer: name and address only, art. 3 last paragraph and the Arabic "اسم المشتري ولقبه وعنوانه إذا كان مستهلكا" | one party form for everyone | row "Party identifiers": name and address when the buyer is a consumer |

The AI has no text making it a facture mention. CIDTA art. 183 ter asks a
wholesaler to hold each client's AI for the annual état-clients, which is why
both products carry the field. Keep it on the party record; printing it is
trade practice, not a legal duty.

## Documents and numbering

| Item | Law | Lumina | dz-pos |
|---|---|---|---|
| Series | décret 05-468 art. 10: "une série ininterrompue et chronologique de factures", a new book only after the previous one is exhausted | per-document-kind counters; whether a cancelled number is reused is not visible in the teardown | one uninterrupted series per kind per year, gapless, never reused. Row "Numbering", fixture `numbering_gapless` |
| Cancellation | art. 10: « La facture régulièrement annulée doit faire l'objet d'une mention "facture annulée" inscrite clairement en diagonale », Arabic "فاتورة ملغاة". That the cancelled facture keeps its number is not in the text; it is our inference from the « série ininterrompue et chronologique » of the same art. 10, put to the comptable as question 4 | `credit_note` template ships, no cancellation overlay seen | a cancelled facture gets an avoir and is not deleted. **The diagonal overlay is not in `docs/features.md` §4** |
| Electronic issue | art. 10 allows "sous forme dématérialisée à travers le recours à un procédé informatique"; art. 4 still requires the cachet humide unless the facture is issued "par voie télématique" under art. 11, whose modalities are set by an « arrêté conjoint des ministres chargés du commerce, des finances et des télécommunications ». Whether that arrêté exists was not searched; the route is open, not closed | prints, with stamp and signature blocks | print engine renders HTML in the core, stamp and signature blocks on the layout |
| Bon de livraison and facture récapitulative | loi 04-02 art. 11 and décret 05-468 arts. 14 to 17: only for repeated regular sales to the same client, on prior authorisation from the commerce administration, with a monthly récapitulative referencing the bons | `delivery_note` and `recap_invoice` templates ship in ar / en / fr | `bon_de_livraison` is a document kind in §3; its template `bon_de_livraison_a4` is parked, not in §4, and the "Later" list models the precondition: the bon de livraison and the facture récapitulative come together, with a wilaya authorisation (`docs/features.md` §4 and Later) |
| Bon de transfert | loi 04-02 art. 11 last paragraph: goods moving without a commercial transaction travel under a bon de transfert; décret 05-468 chapter 2 | `reception_note` is a different thing; no bon de transfert seen | absent from `docs/features.md`. Relevant the day a shop has two locations |
| Avoir / credit note | no Algerian text found that governs the avoir. It is accounting practice | `credit_note` template | `avoir` is a document kind, and is the way a cancelled facture is corrected |
| Proforma | no legal status; it is an offer, not a facture | `proforma_invoice` template | `proforma` kind and template |
| Payment receipt | Code du timbre art. 100-I taxes « les Titres de quelle que nature qu'ils soient [...] qui comportent libération ou qui constatent des paiements ou des versements de sommes », so a quittance for a later cash payment is itself stampable | `payment_receipt` template | **no `quittance` kind in §3.** Question 6 of the comptable page |
| Ticket de caisse | loi 04-02 art. 10 al. 3 requires it for a consumer sale and says nothing about its content | `invoice_thermal_80.html` and five receipt templates | `ticket` kind, `ticket_80mm` template, seller identity only |
| Print language | no text. Loi 91-05 generalises Arabic in public administration, and the JO publishes in both | `settings.print_language`, independent of the UI language | UI language and print language are independently selectable, §Scope |
| Barcode label | no legal content | `barcode_label_template_1.html` | `barcode_label` template |

## Ledger fields on the document

`OLD_BALANCE`, `REMAINING_DEBT` and `TOTAL_DEBT` are printed on Lumina's
facture. No article of décret 05-468, of loi 04-02 or of the fiscal codes read
so far requires or forbids them. They are a convenience for a credit customer
and they are what the market expects, which is the only argument for them.
`docs/features.md` carries the same three fields in the totals table with the
same lack of a legal basis. Worth a line in the spec saying so, because a
reader of that table cannot currently tell which rows are law and which are
habit.

## What this table changes

- Three of Lumina's calculation choices are not options for us: the stamp
  formula, the single TVA rate, and the absence of a regime setting. The first
  two are already corrected in `docs/features.md`; the third is the row
  "Régime fiscal" there, fixture `regime_ifu_prints_no_tva`.
- Two of Lumina's field choices are worth copying because a text backs them:
  the NIF on both parties, and the amount in words. Two are worth copying with
  the reason recorded as practice rather than law: the AI, and the debt block.
- Four mentions of décret 05-468 art. 3 are missing from both products: forme
  juridique, nature de l'activité, capital social, and the settlement date.
  They belong in store settings and on the party record.
- The diagonal "facture annulée" overlay is a print-engine requirement that
  `docs/features.md` §4 does not mention.
