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

Not yet downloaded (server returned 500 on the first try): Code des impôts
directs 2026 (`.../3726/CodedesImpotsDirectsetTaxesAssimilees2026fr`), needed
for art. 324 (rounding). The DGI site's TLS chain is incomplete; fetch with
certificate verification off and check the `%PDF` header.

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

What this changes against the Lumina-derived assumption in `docs/features.md`
(`clamp(net × 1 %, 5, 2 500)`, cash only):

- No 2 500 DA cap. The rate rises with the amount instead.
- The unit is a tranche of 100 DA or fraction thereof, so 250 DA counts as
  three tranches. A percentage on the exact amount is not the same number.
- Amounts of 300 DA or less: the text says "supérieur à 300 DA", so
  whether a 300 DA cash sale owes the 5 DA minimum or nothing needs a
  reading of the whole title (art. 100-II and the exemptions in 258 ff.).
  Open question for the comptable.
- Whether the tranches are marginal (1 DA on the first 30 000, 1,5 DA on
  the next 70 000, 2 DA above) or the whole amount takes the rate of its
  band. The text reads as bands on "sommes dont le montant", i.e. the whole
  amount; vendor calculators (Fatoura, IntelliX) apply it marginally.
  Open question; the fixture must pin one and the comptable confirms.
- Electronic payment is exempt. "Cash only" was right for the wrong
  reason; card, CIB, Edahabia and transfer are exempt by art. 258
  quinquies, not by absence of a rule.

### TVA rates (read, primary)

CTCA 2026, art. 21 (p. 17): taux normal 19 %. Art. 23: taux réduit 9 %,
with the list of goods by customs tariff line (potatoes, tomatoes, onions,
cabbages, lettuce ... continues over several pages). Art. 22 abrogé.

Consequence: which of a shop's products are at 9 % is a lookup by tariff
line, not a guess. The product form needs the rate per product (open
decision 3 in features.md is settled by the law's shape) and a way to
default it from a category.

### Rounding (pointer found, target not yet read)

CTCA art. 80bis (p. 44): rounding of TVA bases and assessed duties follows
art. 324 of the Code des impôts directs. Art. 324 not yet read (PDF fetch
failed). Until then "half away from zero, once per rate" stays an
assumption.

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

### Amount in words (partly read)

The decree requires the TTC total "en chiffres et en lettres". French
wording in practice: "Arrêtée la présente facture à la somme de ... dinars
algériens et ... centimes". Arabic convention on printed factures not yet
sourced.

## Lumina versus the law (to be completed as a table, task R7)

| Rule | Lumina implements | Law says | Status |
|---|---|---|---|
| Droit de timbre | 1 % of net, min 5, max 2 500, cash only | progressive per 100 DA tranche, min 5, no cap, electronic exempt | Lumina outdated |
| TVA | 19 / 9 / 0 per product | art. 21 / 23, per tariff line | consistent |
| Facture mentions | full field list in teardown | décret 05-468 | to be diffed |
