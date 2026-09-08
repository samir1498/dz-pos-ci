# Arabic amount in words: what the official texts do (2026-09-08)

Task R6. No code here, only the corpus, the rules it supports, and a
convention to hand to a native reviewer.

The Arabic edition of the Journal Officiel writes every amount in letters
next to the figures, and the LF 2025 issue gives eighteen worked examples
covering hundreds, thousands, millions and milliards. It never writes a
centime, so the dinar side of the convention rests on a primary source and
the centime side does not.

## Sources

| Source | What it gives | Extraction |
|---|---|---|
| `sources/JO-2024-084-LF2025-ar.pdf`, JO n° 84 of 26 December 2024, Arabic edition | 18 amounts in letters with the figures beside them, from 100 DA to 8 523 063 673 111 DA | Text extracts once normalised NFKC. Correction to the earlier note in the findings doc: the file is not glyph-locked. Its fonts encode Arabic Presentation Forms-B, so a raw search for `دينار` misses and `ﺩﻳﻨﺎﺭ` hits. NFKC folds them |
| `sources/JO-2005-080-decret-05-468-facture-ar.pdf`, JO n° 80 of 11 December 2005, Arabic edition | The official Arabic wording of décret 05-468, including the phrase for "in figures and in letters" and the cancelled-facture mark | Font has no ToUnicode map, so the text layer is unreadable. Pages 19 and 20 were rendered to PNG and read visually |
| `sources/BanqueAlgerie-Instruction-05-95-normalisation-du-cheque.pdf` | Where the amount in letters goes on a cheque, zone B, after "Payez contre ce chèque", on two ruled lines | French only. No Arabic, no spelling rule, no centime |

Searched and not found tonight: an official Algerian text that writes a
centime amount in Arabic letters. The LF 2025 Arabic issue has zero
occurrences of `سنتيم`. The Arabic JO of 2005 cannot be searched as text.
The DGI file server at `mfdgi.gov.dz/files/803/2026/` answered HTTP 500 on
every id from 3727 to 3732, so the Arabic editions of the fiscal codes could
not be pulled to check art. 324 of the CIDTA, which is where a centime
wording would appear.

## The corpus: every amount in letters in the LF 2025 Arabic edition

| Figure | Arabic, as printed | What it shows |
|---|---|---|
| 100 | مائة دينار | 100 takes the singular in the genitive, no tanween |
| 200 | مائتا دينار | dual of مائة, nūn dropped before the counted noun |
| 500 | خمسمائة دينار | hundreds are written joined |
| 1 000 | ألف دينار | |
| 1 500 | ألف وخمسمائة دينار | و joins every element |
| 2 000 | ألفا دينار | dual of ألف, nūn dropped in the construct |
| 2 500 | ألفان وخمسمائة دينار | dual keeps its nūn when another element follows |
| 4 000 | أربعة آلاف دينار | 3 to 10 take the broken plural of the unit word |
| 8 000 | ثمانية آلاف دينار | |
| 10 000 | عشرة آلاف دينار | |
| 12 000 | اثنا عشر ألف دينار | 11 to 99 take the singular of the unit word |
| 50 000 | خمسين ألف دينار | the tens decline with the sentence case |
| 100 000 | مائة ألف دينار | |
| 8 000 000 | ثمانية ملايين دينار | |
| 10 000 000 | عشرة ملايين دينار | |
| 200 000 000 | مائتا مليون دينار, and مائتي مليون دينار after a preposition | |
| 15 816 812 151 000 | خمسة عشر ألفا وثمانمائة وستة عشر مليارا وثمانمائة واثنا عشر مليونا ومائة وواحد وخمسون ألف دينار | order runs milliards, millions, thousands; the last unit word before دينار is ألف, so دينار stays singular with no tanween |
| 8 523 063 673 111 | بثمانية آلاف وخمسمائة وثلاثة وعشرين مليارًا وثلاثة وستين مليونًا وستمائة وثلاثة وسبعين ألفًا ومائة وأحد عشر دينارًا | the only amount in the issue that ends on a unit count between 11 and 99, and the only one where the dinar word carries tanween |

Hundreds seen in print: مائة, مائتا, مائتي, خمسمائة, ستمائة, سبعمائة,
ثمانمائة. The issue never needs 300, 400 or 900, so ثلاثمائة, أربعمائة and
تسعمائة are inferred from the same joined pattern and are a question for the
reviewer. The spelling is مائة throughout, never مئة.

## Count forms

Arabic changes the counted noun by the value of the last count word. دينار is
masculine, so 3 to 10 take the feminine-marked numeral (ثلاثة, ثمانية, عشرة).
سنتيم is masculine too and takes the same shapes.

| Last count | Dinar | Centime | Evidence |
|---|---|---|---|
| 1 | دينار واحد | سنتيم واحد | no JO example, standard grammar |
| 2 | ديناران, دينارين after a preposition | سنتيمان, سنتيمين | no JO example; the dual pattern is confirmed by ألفا and ألفان |
| 3 to 10 | ثلاثة دنانير | ثلاثة سنتيمات | pattern confirmed by أربعة آلاف, عشرة ملايين; the dinar plural itself has no JO example |
| 11 to 99 | أحد عشر دينارًا | أحد عشر سنتيمًا | confirmed, ومائة وأحد عشر دينارًا |
| 100, 200, hundreds | مائة دينار | مائة سنتيم | confirmed |
| 1 000 and above ending on ألف, مليون, مليار | ألف دينار | ألف سنتيم | confirmed |

## Décret 05-468 in Arabic

Art. 3, last seller mention, JO n° 80 of 2005, Arabic edition, p. 19:

> السعر الإجمالي مع احتساب كل الرسوم، محرّرًا بالأرقام والأحرف

That is the Arabic of "prix total toutes taxes comprises, libellé en chiffres
et en lettres". The phrase to use on a printed Arabic facture is
"بالأرقام والأحرف".

The buyer rule in Arabic, same page: "يجب أن تحتوي الفاتورة على اسم المشتري
ولقبه وعنوانه إذا كان مستهلكا."

Art. 10, p. 20, the cancelled facture: "ويجب أن تتضمن الفاتورة الملغاة قانونا
عبارة ”فاتورة ملغاة“ تسجل بوضوح بطول خط زاوية الفاتورة." The exact string for
the cancellation overlay in the Arabic templates is فاتورة ملغاة.

Official Arabic field labels from the same article, worth copying verbatim
into the i18n file rather than translating afresh:

| Field | Official Arabic |
|---|---|
| registre de commerce | رقم السجل التجاري |
| identifiant statistique, read as NIF | رقم التعريف الإحصائي |
| mode de paiement et date de règlement | طريقة الدفع وتاريخ تسديد الفاتورة |
| date et numéro d'ordre | تاريخ تحرير الفاتورة ورقم تسلسلها |
| désignation et quantité | تسمية السلع المبيعة وكميتها |
| prix unitaire hors taxes | سعر الوحدة دون الرسوم |
| prix total hors taxes | السعر الإجمالي دون احتساب الرسوم |
| nature et taux des taxes | طبيعة الرسوم ونسبها المستحقة |
| prix total TTC | السعر الإجمالي مع احتساب كل الرسوم |
| cachet humide et signature | الختم الندي وتوقيع البائع |
| facture | الفاتورة |
| bon de livraison | وصل التسليم |
| facture récapitulative | الفاتورة الإجمالية |
| bon de transfert | سند التحويل |

## Proposed convention

Write the amount as one string, dinars then centimes, joined by و, with no
currency abbreviation:

```
فقط <dinars in letters> <dinar count form> و<centimes in letters> <centime count form> لا غير
```

Decisions inside it, each with the reason:

- **مائة, not مئة.** The JO spells مائة without exception.
- **Joined hundreds.** ثلاثمائة, أربعمائة, خمسمائة, ستمائة, سبعمائة,
  ثمانمائة, تسعمائة. The JO prints the four it needs in the joined form.
- **و between every element**, descending by magnitude, units before tens
  inside each pair (ثلاثة وعشرون, not عشرون وثلاثة). The JO does this
  everywhere.
- **Count form by the last count word**, per the table above.
- **Tanween on the counted noun for 11 to 99 only**, written as دينارًا. The
  JO writes it, and it is the one place where the JO's own vowelling is
  visible.
- **Nominative by default.** The JO amounts appear inside sentences and take
  the case the sentence requires, which is why the same 200 000 000 appears
  as مائتا and as مائتي. A facture line stands alone, so the nominative is
  the right default and the generator does not need case agreement.
- **فقط ... لا غير** brackets the amount. This is the cheque and facture
  formula in circulation in Algeria. No text prescribes it. Keep it behind an
  i18n key so it can be dropped without touching the number code.
- **سنتيم for the subunit**, plural سنتيمات. No Algerian official text found
  tonight uses the word in letters, so this line is an assumption.
- **Zero centimes.** Print the dinar part alone rather than "وصفر سنتيم".
  No source; a choice, and one to put to the reviewer.

## What the native reviewer decides

Send these five with the golden file, not before it exists:

1. ثلاثمائة, أربعمائة, تسعمائة in the joined form, since the JO never needs them.
2. سنتيم and its plural سنتيمات, against any form an Algerian accountant
   actually writes on a facture.
3. Whether فقط and لا غير belong on a facture or only on a cheque.
4. Whether zero centimes is dropped or written.
5. Whether the dual ديناران and سنتيمان are used in practice or replaced by
   the figure, which is what many printed factures do.
6. Whether the currency word carries جزائري on a facture. The JO writes bare
   دينار everywhere; the cheque formula in circulation says دينار جزائري. The
   convention above uses the bare form because that is the sourced one.

## For the fixtures, later

The golden file `words_ar_golden` should carry one row per line of the count
table plus the two long JO amounts verbatim, because those two are the only
rows backed by a primary source and they are the rows that will catch a
regression in the thousands and milliards joins. Hand-write every expectation
in centimes as the money contract requires. Nothing here becomes code until
the reviewer has answered the five questions above.
