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
| `Circulaire-14-MF-DGI-LF2025-timbre-de-quittance-2025-03-05.pdf` | DGI circular n° 14/MF/DGI/LF.2025 of 5 March 2025 on how to compute the droit de timbre de quittance (4 p., scanned; OCR text is rough) | copy hosted by aminahadji.com; the DGI lists it under legislation-fiscale/circulaires-et-instructions |

| `CodedesImpotsDirectsetTaxesAssimilees2026fr.pdf` | Code des impôts directs et taxes assimilées (CIDTA), édition 2026 (241 p.) | DGI, mfdgi.gov.dz/files/803/2026/3726 |

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

### Facture: mandatory mentions (read, official FAQ, decree text pending)

commerce.gov.dz FAQ "Que doit comporter une facture ?", restating décret
exécutif 05-468 (JO n° 80, 11 déc. 2005). Seller mentions: name / raison
sociale, address + phone + fax + email, legal form and activity, capital
social, RC number, NIS, mode and date of payment, date and numéro d'ordre,
goods and quantities, unit price HT, total HT, nature and rate of taxes
(TVA not shown if the buyer is exempt), **total TTC in figures and in
words**. Buyer mentions: name / raison sociale, legal form and activity,
address + phone, RC, NIS (the FAQ list continues; the decree text on
commerce.gov.dz/fr/reglementation/decret-executif-n05-468 has the full
articles, to be read for numbering rules, bon de livraison, facture
récapitulative).

Related texts listed on the same page: arrêté du 1er août 2013 (fausses
factures, sanctions). Consumer sales: the FAQ says the seller must issue a
facture if the consumer asks; what a ticket de caisse must carry is in loi
04-02 (pratiques commerciales), not yet read.

### IFU regime (not yet read, product-shaping)

Search results agree that sole traders under the impôt forfaitaire unique
(threshold quoted as 8 M DA, 15 M DA for achat-revente in one source) do
not invoice TVA at all. If most target shops are IFU, the till's default
document shows no TVA line and the per-rate rounding design serves the
minority on the régime réel. Needs the CIDTA articles (282 ter ff.) and
the LF 2026 changes read, then a decision on a per-shop "régime" setting.

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
- **Arabic:** no Algerian text prescribes the spelling. The JO Arabic
  edition (`sources/JO-2024-084-LF2025-ar.pdf`, glyph-encoded, not
  extractable as text) is the official usage to copy from by reading the
  pages; the cheque formula in use is "فقط ... دينار جزائري و ... سنتيم لا
  غير". The Arabic golden file is hand-written from those pages and
  reviewed by a native speaker before the first printed facture.

Recommendation: write the three converters in Rust in `crates/core`
(integer centimes in, string out; cardinals up to 10^9 are ~100 lines per
language), golden-tested, with the French agreement traps (`quatre-vingts`
/ `quatre-vingt-un`, `cent` / `cents`, `mille` invariable) and the Arabic
dual and plural forms as fixture rows. Use `n2words` only in the vitest
side as a second implementation to cross-check the golden files, never as
a runtime dependency. A native speaker reviews the Arabic golden file
before the first printed facture.

## Lumina versus the law (to be completed as a table, task R7)

| Rule | Lumina implements | Law says | Status |
|---|---|---|---|
| Droit de timbre | 1 % of net, min 5, max 2 500, cash only | per 100 DA tranche rounded up, 1 / 1,5 / 2 DA by band on the whole amount, ≤ 300 DA free, min 5, no cap, electronic exempt (art. 100-I, circ. 14/2025) | Lumina outdated |
| TVA | one global rate for the whole cart (`currentTVAPercentage`), applied in float to the discounted subtotal, printed with `toFixed(2)` | art. 21 / 23: the rate is per product by tariff line; no facture rounding rule (CIDTA 324 is for the return) | Lumina simplifies; a mixed 19/9 basket is wrong there |
| Amount in words | own `numberToWords.js`, fr / ar / en, dinars and centimes | décret 05-468: TTC in figures and words | consistent in intent; we write our own |
| Facture mentions | full field list in teardown | décret 05-468 | to be diffed |
