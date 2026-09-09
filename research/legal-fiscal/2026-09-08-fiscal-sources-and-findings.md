# Fiscal sources and first findings (2026-09-08)

Status: first pass. Every row below says whether it is read from a primary
source or still an assumption. The research plan
`legal-fiscal-and-tooling-research-for-dz-pos` in `context/` tracks what
remains.

## Primary sources on hand (`sources/`)

| File | What | Origin |
|---|---|---|
| `CodedeTimbre2026fr.pdf` | Code du timbre, édition 2026 (121 p.) | DGI, mfdgi.gov.dz/files/803/2026/3724 |
| `CodedesTaxessurleChiffredAffaires2026fr.pdf` | Code des taxes sur le chiffre d'affaires (CTCA), édition 2026 (179 p.) | DGI, mfdgi.gov.dz/files/803/2026/3725 |
| `JO-2024-084-LF2025.pdf` | Journal Officiel n° 84/2024, loi de finances 2025 | joradp.dz/FTP/JO-FRANCAIS/2024/F2024084.pdf |
| `JO-2024-084-LF2025-ar.pdf` | The same JO issue, Arabic edition (official Arabic spelling of amounts, read visually) | joradp.dz/FTP/JO-ARABE/2024/A2024084.pdf |
| `BanqueAlgerie-Instruction-05-95-normalisation-du-cheque.pdf` | Cheque layout standard, amount in letters zone | bank-of-algeria.dz |
| `Decret-05-468-facture.pdf` | Décret exécutif 05-468, facture / bon de livraison / facture récapitulative (JO n° 80, 2005) | commerce.gov.dz/fr/telecharger/reglementation/283/article |
| `Loi-04-02-pratiques-commerciales.pdf` | Loi 04-02 on commercial practices (JO n° 41, 2004). Full text, arts. 1 to 67, not an excerpt as first noted; the two-column layout defeats naive text extraction | africa-laws.org copy; matches the official joradp copy word for word on arts. 10, 33, 34 |
| `Circulaire-14-MF-DGI-LF2025-timbre-de-quittance-2025-03-05.pdf` | DGI circular n° 14/MF/DGI/LF.2025 of 5 March 2025 on how to compute the droit de timbre de quittance (4 p., scanned; OCR text is rough) | copy hosted by aminahadji.com; the DGI lists it under legislation-fiscale/circulaires-et-instructions |
| `CodedesImpotsDirectsetTaxesAssimilees2026fr.pdf` | Code des impôts directs et taxes assimilées (CIDTA), édition 2026 (241 p.) | DGI, mfdgi.gov.dz/files/803/2026/3726 |
| `JO-2004-041-loi-04-02-pratiques-commerciales.pdf` | Official JO n° 41 of 27 June 2004, loi 04-02 at p. 3 | joradp.dz/FTP/JO-FRANCAIS/2004/F2004041.pdf, fetched 2026-09-08 |
| `JO-2010-046-loi-10-06-modifiant-04-02.pdf` | JO n° 46 of 18 August 2010, loi 10-06 at p. 10; its art. 3 rewrites loi 04-02 art. 10 | joradp.dz/FTP/JO-FRANCAIS/2010/F2010046.pdf, fetched 2026-09-08 |
| `JO-2005-080-decret-05-468-facture.pdf` | Official JO n° 80 of 11 December 2005, décret exécutif 05-468 at p. 16 | joradp.dz/FTP/JO-FRANCAIS/2005/F2005080.pdf, fetched 2026-09-08 |
| `JO-2005-080-decret-05-468-facture-ar.pdf` | The same JO issue, Arabic edition. Its font carries no ToUnicode map, so pages 19 and 20 are read as rendered images; they give the official Arabic field labels, the "بالأرقام والأحرف" phrase and the "فاتورة ملغاة" cancellation string | joradp.dz/FTP/JO-ARABE/2005/A2005080.pdf, fetched 2026-09-08 |
| `JO-2016-010-decret-16-66-bon-de-transaction-commerciale.pdf` | JO n° 10 of 22 February 2016, décret exécutif 16-66 at p. 3: the model of the document tenant lieu de facture, for agriculture, pêche, aquaculture, artisanat only | joradp.dz/FTP/JO-FRANCAIS/2016/F2016010.pdf, fetched 2026-09-08 |
| `JO-2014-030-arrete-2013-08-01-fausses-factures.pdf` | JO n° 30 of 21 May 2014, arrêté of 1 August 2013 at p. 7: false factures and factures de complaisance, 50 % fiscal fine | joradp.dz/FTP/JO-FRANCAIS/2014/F2014030.pdf, fetched 2026-09-08 |

The DGI site's TLS chain is incomplete and the file server answers 500 or
refuses connections at times; fetch with certificate verification off, retry,
and check the `%PDF` header.

## Findings

### Droit de timbre: the current rule is progressive (read, primary)

Code du timbre 2026, art. 100-I (p. 18):

> Les titres de quelque nature qu'ils soient [...] qui constatent des
> paiements ou des versements de sommes, sont assujettis à un droit de
> timbre dont la quotité est fixée par tranche de 100 DA ou fraction de
> tranche de 100 DA, comme suit :
> − Sommes dont le montant est supérieur à 300 DA et n'excédant pas les
>   30 000 DA : 1 DA ;
> − Sommes dont le montant est supérieur à 30 000 DA et n'excédant pas les
>   100 000 DA : 1,5 DA ;
> − Au-delà de la somme de 100 000 DA : 2 DA.
> Toutefois, le montant du droit dû ne peut être inférieur à 5 DA.

Art. 258 quinquies (p. 49):

> Sont également dispensées du droit de timbre, prévu à l'article 100-I du
> présent code, les quittances de sommes réglées par des moyens de paiement
> électronique.

DGI circular n° 14/MF/DGI/LF.2025 (5 March 2025) settles how to apply it:

- **Not progressive by tranche.** The whole amount takes the rate of its
  band: "ce droit de timbre n'est pas soumis à une progressivité par
  tranche". Example 2 in the circular: 35 000 DA cash → 350 tranches ×
  1,5 DA = 525 DA. Example 3: 350 000 DA → 3 500 × 2 DA = 7 000 DA.
  Example 1: 25 000 DA → 250 × 1 DA = 250 DA.
- **300 DA or less: nothing to pay.** "Les sommes n'excédant pas 300 DA ne
  donnent lieu à aucun droit."
- Tranches: divide by 100 and round up (a fraction counts as a tranche).
- Minimum 5 DA on anything above 300 DA.
- Applies to any document recording a payment: "quittance, facture,
  ticket de caisse".
- Electronic payment of any form (cards, transfer, cheque, mobile) is
  exempt, art. 258 quinquies.

So the rule for the fixture `stamp_progressive_tranches`, in centimes:

```
if mode is electronic or amount <= 300 00      -> 0
tranches = ceil(amount / 100 00)
rate     = 100 if amount <= 30 000 00
           150 if amount <= 100 000 00
           200 otherwise                       (centimes per tranche)
stamp    = max(tranches * rate, 5 00)
```

Cases to pin: 300 00 → 0; 300 01 → 5 00 (4 tranches × 1 DA = 4 DA, floor
applies); 1 000 00 → 10 00; 1 000 01 → 11 00; 25 000 00 → 250 00;
30 000 00 → 300 00; 30 000 01 → 451 00 (301 tranches × 1,5 = 451,5,
rounded how? the circular's examples are all whole tranches; pin 451 50
and ask); 35 000 00 → 525 00; 100 000 00 → 1 500 00; 100 000 01 → 2 002 00;
350 000 00 → 7 000 00.

What this changes against the Lumina-derived assumption in `docs/features.md`
(`clamp(net × 1 %, 5, 2 500)`, cash only):

- No cap, and the rate rises with the amount.
- The unit is a tranche of 100 DA rounded up, not a percentage of the
  exact amount.
- Card, transfer and cheque are exempt by law, not by our choice.
- One question left for the comptable: a 1,5 DA rate on an odd number of
  tranches gives half a dinar; the circular's examples avoid the case.

### TVA rates (read, primary)

CTCA 2026, art. 21 (p. 17): taux normal 19 %. Art. 23: taux réduit 9 %,
with the list of goods by customs tariff line (potatoes, tomatoes, onions,
cabbages, lettuce ... continues over several pages). Art. 22 abrogé.

Consequence: which of a shop's products are at 9 % is a lookup by tariff
line, not a guess. The product form needs the rate per product (open
decision 3 in features.md is settled by the law's shape) and a way to
default it from a category.

### Rounding (read, primary): the law rounds the tax return, not the facture

CTCA 2026 art. 80bis (p. 44) sends TVA rounding to CIDTA art. 324. CIDTA
2026 art. 324 (p. 120):

> 1) Sauf dispositions spéciales précisées au présent code, les sommes
> servant de base à l'assiette des impôts directs et taxes assimilées,
> sont arrondies au dinar inférieur, si elles n'atteignent pas dix (10)
> dinars, à la dizaine de dinars inférieure dans le cas contraire. [...]
> Les cotisations relatives aux impôts directs et taxes assimilées, sont
> arrondies à la dizaine de centimes la plus voisine, les fractions
> inférieures à cinq (5) centimes étant négligées et les fractions égales
> ou supérieures à cinq (5) centimes étant comptées pour dix (10) centimes.

That is a rule for the tax base and the tax due on the monthly return
(G50), where the base is rounded down to the dinar or the ten dinars and
the duty to the nearest ten centimes. Nothing in the CTCA or in décret
05-468 prescribes how a line or a TVA amount on a facture is rounded.

Consequence for the product: on the facture, our rule stands as a design
choice, not a legal one: integer centimes, TVA computed per rate group on
the group's HT subtotal, rounded once, half away from zero, to the centime.
The G50 rounding (art. 324) is a reporting feature for later, applied to
the period totals, never to a document. The comptable review confirms the
facture practice (many shops print TVA to the centime, some to the dinar).

Also noted while reading: the CIDTA "Codes fiscaux 2026" download is
`.../files/803/2026/3726/CodedesImpotsDirectsetTaxesAssimilees2026fr`
(saved in `sources/`, 241 p.).

### Facture and ticket: what the texts require (read, primary)

Sources in `sources/`: `Decret-05-468-facture.pdf` (JO n° 80 of 11 Dec
2005, from commerce.gov.dz) and `Loi-04-02-pratiques-commerciales.pdf`
(JO n° 41 of 27 June 2004; the copy on hand is an excerpt, the 2010
amendment by loi 10-06 is not in it).

Décret 05-468:

- **Art. 3, seller mentions:** name / raison sociale; address, phone, fax,
  email if any; legal form and activity; capital social if any; RC
  number; NIS; payment mode and settlement date; date and *numéro
  d'ordre*; goods and quantities; unit price HT; total HT; nature and
  rate of taxes and duties (TVA omitted if the buyer is exempt); **total
  TTC in figures and in words**.
- **Art. 3, buyer mentions:** name / raison sociale; legal form and
  activity; address, phone, fax, email; RC; NIS. **If the buyer is a
  consumer: name, first name and address only.**
- **Art. 4:** wet stamp (cachet humide) and seller's signature, except
  when issued "par voie télématique" (art. 11, needs a joint arrêté).
  For a printed facture from the till this means a stamp block and a
  signature block on the layout.
- **Art. 5 and 6:** the TTC total includes every remise, rabais and
  ristourne decided at the sale; the three words are defined.
- **Art. 7 to 9:** transport not in the unit price, price supplements
  (interest on deferred payment, commissions), returnable packaging
  deposits and costs advanced for a third party are shown as their own
  lines on the facture.
- **Art. 10, numbering and cancellation:** the facture is regular when it
  comes from a *facturier*, a stub book with "une série ininterrompue et
  chronologique de factures", or is produced "sous forme dématérialisée
  à travers le recours à un procédé informatique". A new book cannot
  start before the previous one is used up. **A cancelled facture keeps
  its number and carries the mention "facture annulée" written
  diagonally.** This is the text behind gapless numbering (plan R5):
  numbers are never reused, a void is a state on the numbered document,
  and the series has no holes.
- **Art. 14 to 17:** bon de livraison instead of a facture only for
  repeated regular sales to the same trader (three or more a week),
  with prior authorisation from the wilaya's commerce directorate, and
  a monthly facture récapitulative listing the bons. Not a v1 feature;
  the data model should not forbid it.

Loi 04-02:

- **Art. 10, as rewritten by loi 10-06 art. 3 (JO n° 46 of 18 August 2010):**
  a sale between economic agents needs "une facture ou un document en tenant
  lieu"; a sale to a consumer needs "un ticket de caisse ou un bon justifiant
  la transaction", and a facture when the customer asks. No text says what a
  ticket must carry. Full quote and the diff against the 2004 wording are in
  `2026-09-08-facture-and-ticket.md`.
- **Art. 12:** facture, bon de livraison, facture récapitulative and bon
  de transfert follow the decree.
- **Art. 33:** no facture where one was due: fine of 80 % of the amount.
  **Art. 34:** non-conforming facture: 10 000 to 50 000 DA, except when the
  omission hits the seller's or buyer's name, their NIF, their address, the
  quantity, the precise designation or the unit price HT, which fall back to
  art. 33 and the 80 % fine.

The NIF is required on a facture by loi 04-02 art. 34 itself, which names
"leur numéro d'identification fiscale" among those six load-bearing mentions.
Décret 05-468 art. 3 still says "numéro d'identification statistique"; loi
05-16 (LF 2006) art. 42 replaced NIS by NIF across the tax codes, not across
the commerce decree. The article d'imposition has no text making it a facture
mention. CIDTA art. 183 ter asks a wholesaler to hold each client's AI for the
état-clients, which is why it appears on factures in circulation.

Consequence for the product: two document kinds. A **ticket** for a
consumer at the till: shop identity, date, number, lines, totals, taxes,
stamp, payment mode; no buyer block. A **facture** when the buyer is a
trader or asks for one: everything in art. 3, buyer block with RC and
NIS (or name and address for a consumer), amount in words, stamp and
signature blocks, TTC including discounts, and the extra lines of arts 7
to 9 when they apply.

### IFU regime (read, primary): an IFU shop must not show TVA at all

CIDTA 2026 (`sources/CodedesImpotsDirectsetTaxesAssimilees2026fr.pdf`):

- **Art. 282 ter (p. 108):** natural persons with an industrial,
  commercial, non-commercial or artisanal activity whose annual turnover
  does not exceed **8 000 000 DA** are under the impôt forfaitaire unique,
  unless they opt for the régime réel. Excluded: property development,
  importers reselling as is, wholesale buy-resell (art. 183 ter), and a
  further list.
- **Art. 282 quinquies (p. 110):** several shops of one owner are taxed
  separately as long as the sum of their turnovers stays under 8 M DA;
  above it, the owner moves to the régime réel for all of them.
- **Art. 282 sexies (p. 110):** IFU rate 5 % for production and sale of
  goods, 12 % for other activities, 0,5 % under the auto-entrepreneur
  status.
- **Art. 282 quater (p. 109):** for products with a regulated price or
  margin (large-consumption goods), the IFU base is the margin, and the
  return must split turnover between regulated and other products.

CTCA 2026 (`sources/CodedesTaxessurleChiffredAffaires2026fr.pdf`):

- **Art. 2-12 (p. 6):** retail sales are in the TVA field "à l'exclusion
  des opérations réalisées par des contribuables relevant de l'impôt
  forfaitaire unique".
- **Art. 64 (p. 41):** "Les redevables placés sous le régime de l'impôt
  forfaitaire unique ne peuvent pas mentionner la taxe sur la valeur
  ajoutée sur leurs factures sous peine de se voir appliquer les
  sanctions prévues à l'article 114." Whoever prints TVA without paying
  it is personally liable for it.

Consequence for the product, and it is a big one:

- The shop has a **régime fiscal** setting: `ifu` or `réel`. It is not a
  per-product thing and not a toggle to hide a line; it changes what a
  price is.
- **IFU shop:** one price per product, no TVA rate, no HT/TTC split, no
  TVA line on ticket or facture (printing one is an offence). Totals are
  lines, discount, stamp (the timbre is a separate duty and still applies
  to cash), net to pay. The facture still carries every 05-468 mention
  except the TVA line. The margin split of art. 282 quater is a reporting
  concern for later (flag on product: regulated price yes/no).
- **Réel shop:** everything already specified: rate per product, HT
  lines, TVA per rate group, TTC.
- A shop can change regime at a year boundary (turnover crosses 8 M DA
  or an option is taken). Documents keep the regime they were issued
  under; the setting is dated, not overwritten.
- Which regime most target shops are under is a market question, not a
  legal one: 8 M DA a year is about 22 000 DA a day of sales, so a small
  supérette is IFU and a busy one is réel. The product must do both from
  v1; the mockup today only shows the réel case.

### Amount in words (decree read; libraries checked)

Décret 05-468 requires the TTC total "en chiffres et en lettres". French
wording in practice: "Arrêtée la présente facture à la somme de ... dinars
algériens et ... centimes". Lumina prints it in the invoice language with
the prefix "Facture arrêtée à la somme de :" (fr) and an Arabic
equivalent; Arabic wording convention on printed factures still needs a
native review.

Lumina ships a hand-written `numberToWords.js` (7.5 KB, fr / ar / en,
dinars and centimes, float input split with `Math.floor` and
`Math.round`). It is their code; we do not copy it. It is evidence that
three languages and centimes are what the market expects, and that the
Arabic form uses the dual ("ألفان") and "دينار / سنتيم".

Open source checked 2026-09-08:

| Library | Languages we need | Notes |
|---|---|---|
| `n2words` 6.1.2 (npm, MIT, zero deps) | fr, en, ar (ar-SA) cardinals; currency form exists but has no DZD (ar/fr know MAD, TND) | good as an independent oracle in vitest for the fr / en / ar cardinal part |
| `num2words` 1.2.0 (crates.io, MIT/Apache) | en, fr only, no Arabic; float input via `num-bigfloat`; currency has a generic `DINAR` | would cover two of three languages and bring a float type into the money path |

Official spelling references found 2026-09-08:

- **French, the rule:** Académie française, "Questions de langue: Nombres
  (écriture, lecture, accord)", dictionnaire-academie.fr/article/QDL057.
  Traditional rule: hyphens between elements under one hundred, none
  around "et" (vingt et un). The 1990 rectifications (JO français du 6
  décembre 1990) allow hyphens everywhere (vingt-et-un, deux-cent-mille);
  both are accepted. Agreement: vingt and cent take an s when multiplied
  and not followed by another number word (quatre-vingts, deux cents, but
  quatre-vingt-un, deux cent trois); mille never varies; million and
  milliard are nouns (deux millions de dinars).
- **French, official Algerian usage:** the Journal Officiel writes amounts
  in words next to the figures, and it uses the 1990 hyphenation:
  "cent cinquante-et-un mille dinars", "mille cinq cents dinars (1.500
  DA)", "cinquante mille dinars (50.000 DA)", "dix millions de dinars",
  "cent cinquante milliards de dinars" (JO n° 84/2024). Our French golden
  file follows the JO style: hyphens everywhere, "de dinars" after
  million/milliard, "dinars" and "centimes" spelled out, no "DA". One JO
  line reads "deux cent millions de dinars"; the Académie rule gives
  "deux cents millions"; pin the Académie form and note the JO variant.
- **Cheques:** Banque d'Algérie instruction n° 05-95 (normalisation du
  chèque, PDF in `sources/`) fixes where the amount in letters goes
  ("Payez contre ce chèque", two lines) and that the marking band carries
  the amount in centimes; it gives no spelling rule.
- **Arabic:** no Algerian text prescribes the spelling, but the JO Arabic
  edition writes every amount in letters and gives 18 worked examples in the
  LF 2025 issue. `sources/JO-2024-084-LF2025-ar.pdf` does extract as text
  once normalised NFKC; its fonts encode Arabic Presentation Forms-B, which
  is what made it look glyph-locked. The corpus, the count-form table, the
  décret 05-468 Arabic wording "محرّرًا بالأرقام والأحرف", the cancellation
  string "فاتورة ملغاة" and the official Arabic field labels are in
  `2026-09-08-amount-in-words-arabic.md`. No official Algerian source found
  for the centime word in letters, so that line stays an assumption and the
  Arabic golden file waits on a native reviewer.

Recommendation: write the three converters in Rust in `crates/core`
(integer centimes in, string out; cardinals up to 10^9 are ~100 lines per
language), golden-tested, with the French agreement traps (`quatre-vingts`
/ `quatre-vingt-un`, `cent` / `cents`, `mille` invariable) and the Arabic
dual and plural forms as fixture rows. Use `n2words` only in the vitest
side as a second implementation to cross-check the golden files, never as
a runtime dependency. A native speaker reviews the Arabic golden file
before the first printed facture.

## Lumina versus the law

The full line-by-line diff is `2026-09-08-lumina-vs-law.md`: every legal
mention and every calculation in the teardown, with the article, what Lumina
does, and what `docs/features.md` says. The two calculations Lumina gets wrong
are the droit de timbre, where it still uses the pre-2025 flat 1 % capped at
2 500 DA, and the TVA, where one global rate cannot produce a correct total
for a basket mixing 19 % and 9 % goods. Four mentions of décret 05-468 art. 3
are missing from both products: forme juridique, nature de l'activité, capital
social, and the settlement date.

## Recommended edits to `docs/features.md`

Written by the research track, to be applied by whoever owns the spec. Each
item names the row and gives the replacement text verbatim.

**Row "Amount in words", Source cell.** Replace

> décret 05-468: total TTC "en chiffres et en lettres"; Arabic wording not yet sourced

with

> décret 05-468 art. 3, "prix total toutes taxes comprises, libellé en chiffres et en lettres"; Arabic wording from the JO Arabic edition, JO n° 84/2024, 18 amounts in letters, count forms and convention in `research/legal-fiscal/2026-09-08-amount-in-words-arabic.md`; the centime word سنتيم has no Algerian source and waits on a native reviewer

**Row "Party identifiers", Source cell.** Replace

> décret 05-468 art. 3 and 4; NIF/AI from tax texts, article to cite (R3)

with

> décret 05-468 art. 3 (seller and buyer mentions, "numéro d'identification statistique") and art. 4 (cachet humide et signature); the NIF is required by loi 04-02 art. 34, which lists it among the mentions whose omission is a défaut de facturation; loi 05-16 (LF 2006) art. 42 replaced NIS by NIF in the tax codes; no text makes the article d'imposition a facture mention, CIDTA art. 183 ter only requires it in the wholesaler's état-clients

**Row "Numbering", Source cell.** Replace

> décret 05-468 art. 10

with

> décret 05-468 art. 10, "une série ininterrompue et chronologique de factures", a new facturier only after the previous one is exhausted, and « La facture régulièrement annulée doit faire l'objet d'une mention "facture annulée" inscrite clairement en diagonale » (Arabic "فاتورة ملغاة", JO n° 80/2005 Arabic edition). That a cancelled facture keeps its number is an inference from the série ininterrompue, not a sentence of the décret; décret 16-66 art. 5 says the same for the bon de transaction commerciale with the mention « ANNULE » in capitals; whether the inference holds in practice and whether the series restarts each year are question 4 of `research/legal-fiscal/2026-09-08-questions-comptable.md`

**Row "Droit de timbre", Source cell.** Append at the end, after the circular
reference:

> ; nearest precedent for the half-dinar case is the code de l'enregistrement art. 11, as modified by LF 2025 art. 31, "les fractions inférieures à 0,5 DA étant négligées et les fractions égales ou supérieures à 0,5 DA étant comptées pour 1 DA", a different code and not binding on the timbre

**New row, after "Party identifiers".** The ticket versus facture trigger is a
rule the spec relies on and never states with a source:

> | Ticket versus facture | a consumer sale takes a ticket de caisse or a bon justifying the transaction, and a facture the moment the customer asks; a sale to another economic agent takes a facture at the time of the sale. No text prescribes the ticket's content | `ticket_vs_facture_trigger` | loi 04-02 art. 10 as rewritten by loi 10-06 art. 3 (JO n° 46 of 18 August 2010, p. 10); décret 05-468 art. 2 last paragraph; sanction loi 04-02 art. 33, 80 % of the amount that should have been invoiced |

**§3, Totals table.** Two additions that are not Source-column edits:

- `old_balance`, `remaining_debt` and `total_debt` should be marked as practice
  rather than law. No text requires or forbids them on a document, and a reader
  of that table currently cannot tell which rows are law and which are habit.
- `amount_in_words` is defined on `net_to_pay`, which includes the stamp, while
  décret 05-468 art. 3 asks for the "prix total toutes taxes comprises". Which
  figure goes in words is question 8 of `2026-09-08-questions-comptable.md`
  and is unanswered. The global discount split needs no edit: the row "Global
  discount spread" already pins it, fixture `discount_spread_largest_remainder`,
  and question 7 of the comptable page checks it against practice.

**§3, document kinds.** There is no `quittance` kind. A receipt for a later cash
payment is itself a title constating a payment under Code du timbre art. 100-I,
and the DGI circular 14/2025 names the quittance first. Question 6 of the
comptable page decides whether the kind is needed.

**§4, Printing.** Add the cancellation overlay to the template requirements: a
cancelled facture prints "facture annulée" (ar "فاتورة ملغاة") written
diagonally across the document, per décret 05-468 art. 10. That it prints
under its own number is the spec's numbering rule, not the décret's words.
