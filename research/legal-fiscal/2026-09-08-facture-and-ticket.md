# Facture and ticket: what the texts require (2026-09-08)

Read from primary sources. Every quote below comes from a Journal Officiel
PDF in `sources/`, downloaded from joradp.dz on 2026-09-08.

The short version: a consumer sale at a supérette till needs a *ticket de
caisse*, and no Algerian text says what that ticket must carry. A facture
is mandatory the moment the buyer is another economic agent, or when any
buyer asks for one, and its field list is fixed by décret 05-468 art. 3.
The NIF sits on a facture because loi 04-02 art. 34 treats its omission as
a défaut de facturation, not because any text lists it among the mentions.

## Sources used

| File in `sources/` | Text | Origin, fetched 2026-09-08 |
|---|---|---|
| `JO-2004-041-loi-04-02-pratiques-commerciales.pdf` | Loi 04-02 du 23 juin 2004, règles applicables aux pratiques commerciales (JO n° 41, p. 3) | joradp.dz/FTP/JO-FRANCAIS/2004/F2004041.pdf |
| `JO-2010-046-loi-10-06-modifiant-04-02.pdf` | Loi 10-06 du 15 août 2010 modifiant et complétant la loi 04-02 (JO n° 46, p. 10) | joradp.dz/FTP/JO-FRANCAIS/2010/F2010046.pdf |
| `JO-2005-080-decret-05-468-facture.pdf` | Décret exécutif 05-468 du 10 décembre 2005 (JO n° 80, p. 16) | joradp.dz/FTP/JO-FRANCAIS/2005/F2005080.pdf |
| `JO-2016-010-decret-16-66-bon-de-transaction-commerciale.pdf` | Décret exécutif 16-66 du 16 février 2016, modèle du document tenant lieu de facture (JO n° 10, p. 3) | joradp.dz/FTP/JO-FRANCAIS/2016/F2016010.pdf |
| `JO-2014-030-arrete-2013-08-01-fausses-factures.pdf` | Arrêté du 1er août 2013, fausses factures et factures de complaisance (JO n° 30 du 21 mai 2014, p. 7) | joradp.dz/FTP/JO-FRANCAIS/2014/F2014030.pdf |
| `CodedesTaxessurleChiffredAffaires2026fr.pdf` | CTCA 2026, art. 29 and the LF 2006 annex | already in `sources/` |
| `CodedesImpotsDirectsetTaxesAssimilees2026fr.pdf` | CIDTA 2026, art. 183 ter | already in `sources/` |

The ministry of commerce's website lists the texts in force per subject. Its
recueil "Règles applicables aux Pratiques Commerciales" lists two entries, loi
04-02 and loi 10-06, so 10-06 is the only amendment to date. Its recueil
"Conditions et modalités d'établissement de la facture" lists four entries:
décret 05-468, the arrêté of 1 August 2013, décret 16-66, and décret exécutif
20-389 of 19 December 2020 (JO n° 78 of 27 December 2020), which the listing
describes as fixing the form and the mentions of the procès-verbal an inspector
writes when he finds a commercial-practices offence. Not opened: the two
recueils themselves and décret 20-389 have no PDF in `sources/`; only the
listing was read. The joint arrêté that décret 05-468 art. 11 calls for, the
one that would set the modalities of the télématique facture, does not appear
in that listing, which is not the same as knowing it was never published.

Correction to the earlier findings note: `Loi-04-02-pratiques-commerciales.pdf`
is not an excerpt. It is the full eight-page JO n° 41 extract, arts. 1 to 67,
and it matches the official joradp copy word for word on arts. 10, 33 and 34.
Its two-column layout confuses naive text extraction, which is what made it
look truncated.

## Loi 04-02 art. 10, as amended by loi 10-06 art. 3

Loi 10-06 art. 3 rewrote art. 10 whole. The text in force reads:

> « Art. 10. — Toute vente de biens ou prestation de services effectuée
> entre les agents économiques exerçant les activités citées à l'article 2
> ci-dessus doit faire l'objet d'une facture ou d'un document en tenant lieu.
>
> Le vendeur ou le prestataire de services est tenu de délivrer la facture
> ou le document en tenant lieu et l'acheteur est tenu de réclamer, selon le
> cas, l'un ou l'autre document. Ils sont délivrés dès la réalisation de la
> vente ou de la prestation de services.
>
> Les ventes de biens ou les prestations de services faites au consommateur
> doivent faire l'objet d'un ticket de caisse ou d'un bon justifiant la
> transaction. Toutefois, la facture ou le document en tenant lieu doit être
> délivré si le client en fait la demande.
>
> Le modèle du document tenant lieu de facture ainsi que les catégories
> d'agents économiques tenus de l'utiliser sont définis par voie
> réglementaire ».

What 10-06 changed against the 2004 original: it added "ou d'un document en
tenant lieu" to the business-to-business obligation, added the fourth alinéa
sending that document's model to a regulation, and reworded the consumer
alinéa with "Toutefois". The consumer ticket obligation itself is older than
2010. The 2004 original already read "Les ventes faites au consommateur
doivent faire l'objet d'un ticket de caisse ou d'un bon justifiant la
transaction. La facture doit être délivrée si le client en fait la demande."

### What the ticket must carry

Nothing. No article of loi 04-02, of décret 05-468 or of décret 16-66 gives
a field list for a ticket de caisse. Art. 10 names the document and stops.
Décret 05-468 art. 3 governs the facture only, and décret 16-66 governs the
bon de transaction commerciale only.

That leaves the ticket's content to other rules that reach it from outside:
the droit de timbre when the customer pays cash, because Code du timbre
art. 100-I taxes « les Titres de quelle que nature qu'ils soient, signés ou
non signés, faits sous signatures privées, qui comportent libération ou qui
constatent des paiements ou des versements de sommes » and the DGI circular
14/MF/DGI/LF.2025 names the ticket de caisse explicitly; and CTCA art. 64,
which says that IFU sellers « ne peuvent pas mentionner la taxe sur la valeur
ajoutée sur leurs factures ». The article says factures. Extending it to
every document kind, ticket included, is the reading of `docs/features.md`
row "Régime fiscal", not the text's.

### Facture versus ticket, the trigger

- Buyer is an agent économique carrying on an activity listed in art. 2
  (production, distribution, services, artisanat, pêche, agriculture): a
  facture is mandatory, and the buyer is under a matching duty to ask for it.
- Buyer is a consumer as defined in art. 3-2 (acquires for non-professional
  ends): a ticket de caisse or a bon justifying the transaction is enough,
  and a facture becomes mandatory as soon as that customer asks for one.
- Décret 05-468 art. 2 repeats the consumer rule from the facture side: "Dans
  ses relations avec le consommateur, le vendeur doit obligatoirement délivrer
  la facture si celui-ci en fait la demande."

A till cannot tell the two apart on its own. The operator decides, so the
sale screen needs a one-tap switch from ticket to facture that pulls in the
buyer block, and it needs it at the moment of sale, because art. 10 requires
the document "dès la réalisation de la vente".

### The sanction

> Art. 33. — Sans préjudice des sanctions prévues par la législation fiscale,
> toute infraction aux dispositions des articles 10, 11 et 13 de la présente
> loi, est qualifiée de défaut de facturation et punie d'une amende égale à
> 80% du montant qui aurait dû être facturé quelle que soit sa valeur.

> Art. 34. — Est qualifiée de facture non conforme, toute infraction aux
> dispositions de l'article 12 de la présente loi et punie d'une amende de
> dix mille dinars (10.000 DA) à cinquante mille dinars (50.000 DA), à
> condition que la non conformité ne porte pas sur le nom ou la raison
> sociale du vendeur ou de l'acheteur, leur numéro d'identification fiscale,
> leur adresse, la quantité, la dénomination précise et le prix unitaire,
> hors taxes, des produits vendus ou des services rendus dont l'omission est
> qualifiée de défaut de facturation et punie conformément aux dispositions
> de l'article 33 ci-dessus.

Loi 10-06 did not touch arts. 33 or 34, so both stand as written in 2004.

Art. 34 splits the mentions of the facture into two classes, and the split
is the reason the print templates need a hard validation gate:

- Six mentions are load-bearing. Seller or buyer name or raison sociale,
  their NIF, their address, the quantity, the precise designation, and the
  unit price HT. Omitting one of these is a défaut de facturation under
  art. 33, punished at 80 % of the amount.
- Every other mention of décret 05-468 art. 3, omitted, costs 10 000 to
  50 000 DA.

## Where the NIF obligation actually comes from

Décret 05-468 art. 3 does not name the NIF. It requires the "numéro
d'identification statistique" for the seller and for the buyer. Loi n° 05-16
du 31 décembre 2005 portant loi de finances pour 2006, art. 42, replaced that
reference across the tax codes:

> Art. 42. — La référence au numéro d'identification statistique (NIS)
> contenue dans les divers codes fiscaux est remplacée par celle du numéro
> d'identification fiscale (NIF). Les codes des impôts sont annotés en
> conséquence.

That substitution reaches the fiscal codes, not décret 05-468, which is a
commerce text and still says NIS on its face. The direct obligation to print
the NIF on a facture is in loi 04-02 art. 34, quoted above, which names
"leur numéro d'identification fiscale" among the mentions whose omission is
a défaut de facturation. Loi 04-02 is from 2004 and already used the term,
a year before the LF 2006 substitution.

Two tax texts make the NIF and the RC useless to omit even where no sanction
bites directly, because the buyer cannot deduct without them:

> CTCA 2026, art. 29: Pour que cette taxe soit admise en déduction, le relevé
> du chiffre d'affaires visé à l'article 76 [...] doit être appuyé d'un état
> [...] comportant pour chaque fournisseur, les informations suivantes :
> Numéro d'identifiant fiscal ; Nom et prénom (s) ou raison sociale ; Adresse ;
> Numéro d'inscription au registre de commerce ; Date et référence de la
> facture ; Montant des achats effectués ou des prestations reçues ; Montant
> de la taxe sur la valeur ajoutée déduite. Le numéro d'identification fiscale
> et celui du registre de commerce doivent être authentifiés selon la
> procédure en vigueur.

> CIDTA 2026, art. 183 ter (créé par l'art. 12/LF 2024): [tout vendeur en gros
> dépose un] état [...] comportant pour chaque client, les informations
> suivantes : nom et prénom (s) ou raison sociale ; numéro d'identification
> fiscale ; numéro d'inscription au registre du commerce ; numéro de l'article
> d'imposition ; adresse précise du client ; montant hors taxes des opérations
> de vente effectuées au cours de l'année civile ; le montant de la taxe sur
> la valeur ajoutée facturée.

Art. 183 ter is the only text read so far that names the **article
d'imposition**, and it asks a wholesaler to hold it for each client, not to
print it on the facture. That is why the AI field is on every facture in
circulation and on Lumina's template: a wholesale customer needs it from its
supplier to file its own état-clients. No text found tonight requires the AI
as a facture mention. Treat it as a field the seller collects on the party
record, printed because trade practice expects it.

## Facture: the field list, décret 05-468 art. 3 and 4

Seller mentions:

- nom et prénom(s) for a natural person, dénomination or raison sociale for
  a legal person
- adresse, numéros de téléphone et de fax, and the email address if there is
  one
- forme juridique de l'agent économique et nature de l'activité
- capital social, when there is one
- numéro du registre du commerce
- numéro d'identification statistique, read as the NIF since LF 2006 art. 42,
  and the NIF is required by loi 04-02 art. 34 in any case
- mode de paiement et date de règlement de la facture
- date d'établissement et numéro d'ordre de la facture
- dénomination et quantité des biens vendus
- prix unitaire hors taxes
- prix total hors taxes
- nature et taux des taxes, droits et contributions dus, the TVA omitted only
  when the buyer is exempt from it
- prix total toutes taxes comprises, libellé en chiffres et en lettres

Buyer mentions: nom et prénom(s) or raison sociale, forme juridique et nature
de l'activité, adresse and phone, fax and email, numéro du registre du
commerce, numéro d'identification statistique. That list assumes a trader.
The consumer case is the last alinéa of art. 3-2: « Si l'acheteur est un
consommateur, la facture doit mentionner ses nom, prénom(s) et adresse. »
Name and address, nothing else, and the identifiers of a party who has none
are not asked for. Loi 04-02 art. 34 separately makes the buyer's name and
address load-bearing, so omitting them is a défaut de facturation.

Art. 4: cachet humide and the seller's signature, unless the facture is issued
"par voie télématique" under art. 11. Art. 11 sends the modalities of that
route to an « arrêté conjoint des ministres chargés du commerce, des finances
et des télécommunications »; whether that arrêté was published was not
searched tonight, so the route is open, not closed. Public-money payments
cannot use it at all. Print layouts keep a stamp block and a signature block
until that search is done.

Art. 10: "La facture doit être lisible et ne comprendre aucune tâche, rature
ou surcharge." A facture is regular when it comes from a carnet à souches with
« une série ininterrompue et chronologique de factures », or is produced by a
computer process; a new book cannot start before the previous one is
exhausted; and « La facture régulièrement annulée doit faire l'objet d'une
mention "facture annulée" inscrite clairement en diagonale. » The text stops
there. That a cancelled facture keeps its number is our inference from the
série ininterrompue of the same article, since reusing the number would break
the series; question 4 of the comptable page asks whether practice agrees.

## Décret 16-66: the bon de transaction commerciale is not our document

Décret 16-66 defines the "document tenant lieu de facture" promised by art. 10
alinéa 4 and names it *bon de transaction commerciale*. Its art. 3 fixes who
must use it:

> Art. 3. — Les catégories d'agents économiques prévues à l'article 1er
> ci-dessus, englobe les opérateurs intervenant dans les secteurs de
> l'agriculture, de la pêche et de l'aquaculture ainsi que celui de
> l'artisanat et des métiers.

A supérette is none of those, so the bon de transaction commerciale is out of
scope for dz-pos. The decree is still worth keeping because it repeats the
numbering discipline in plainer words than 05-468 does, and it fixes the exact
cancellation mark:

> Le carnet à souches comprend une numérotation de série ininterrompue et
> chronologique de bons de transaction commerciale et ne peut être entamé
> qu'après épuisement du précédent.
>
> Le bon de transaction commerciale régulièrement annulé doit être barré en
> diagonale et porter la mention « ANNULE » en lettres capitales, clairement
> inscrite.

Its mention list is short: désignation, prix unitaire, quantité, montant par
produit, montant total, plus the deposit on returnable packaging and costs
advanced for a third party. The annexed models carry more than the article
lists: annexes 1, 1 bis and 2 have a "Numéro d'identification fiscal (NIF)"
line for the seller, every model has a "montant total hors taxes" column, and
annexes 2 and 3 add a "TVA (DA)" column. It is a transparency instrument for
produce circuits, and it takes the seller's stamp and signature plus the
buyer's signature.

## Arrêté of 1 August 2013: found, and it is about fraud

The arrêté the plan asked about exists, and it is not about facture mentions.
It was signed 1 August 2013 and published nine months later in JO n° 30 of
21 May 2014, p. 7. It defines the false facture and the facture de complaisance
under art. 65 of loi 02-11 (LF 2003) and CIDTA art. 219 bis.

> Art. 2. — La fausse facture, est la facture établie sans avoir procédé à
> aucune livraison ou prestation [...]

> Art. 3. — Il est entendu par facture de complaisance, le fait de camoufler
> ou de dissimuler sur une facture, l'identité ou l'adresse de ses fournisseurs
> ou de ses clients, ou d'accepter sciemment l'utilisation d'une identité
> fictive ou d'un prête-nom [...] La facture de complaisance correspond à un
> achat, une vente ou une prestation de service réel.

> Art. 4. — L'établissement de fausses factures ou de factures de complaisance
> entraîne l'application d'une amende fiscale égale à 50% de leur valeur et ce,
> conformément aux dispositions de l'article 65 de la loi n° 02-11 [...]
>
> L'amende fiscale citée précédemment s'applique, pour les cas de fraudes ayant
> trait à l'émission des fausses factures, tant à l'encontre des personnes
> ayant procédé à l'établissement des factures qu'à l'encontre de celles ayant
> été destinataires desdites factures.

The 50 % fine applies to both kinds. The second alinéa extends it to the
recipient, and states that extension for the fausses factures case; it does
not say the same for the facture de complaisance, so whether the buyer of a
facture de complaisance is fined under this arrêté is not settled by its text.
A POS that lets an operator type a free-text buyer name onto a facture is
still one keystroke away from a facture de complaisance on the seller's side.
The buyer block should be a foreign key to a party record with its
identifiers, and a party created at the till should be flagged as unverified
until someone fills the RC and the NIF.

## Field list per document kind, for the product

| Kind | Legal basis | Buyer block | Amount in words | Stamp and signature block | TVA line |
|---|---|---|---|---|---|
| `ticket` | loi 04-02 art. 10 al. 3 | none | not required | not required | only if the shop is au réel |
| `facture` | loi 04-02 art. 10 al. 1 and 2, décret 05-468 art. 3 and 4 | full, or name and address for a consumer | required, TTC in figures and words | required, unless télématique under art. 11 | only if the shop is au réel |
| `facture` to a consumer on request | décret 05-468 art. 2 last alinéa (issued on request) and art. 3-2 last alinéa (« ses nom, prénom(s) et adresse ») | name, prénom(s) and address | required | required | as above |
| `bon de livraison` and `facture récapitulative` | loi 04-02 art. 11, décret 05-468 arts. 14 to 17 | full | on the récapitulative | required | on the récapitulative |
| `bon de transaction commerciale` | décret 16-66 | none beyond the signature | not required | seller stamp and signature, buyer signature | none |

The last row is out of scope for a supérette and is here so nobody adds it
later by reading only the decree's title.

## What is still open

- No text prescribes what a ticket de caisse contains. What shops actually
  print is a practice question for the comptable, not a legal one.
- The AI (article d'imposition) has no text making it a facture mention.
- Décret 05-468 art. 11 allows a facture "par voie télématique" subject to an
  « arrêté conjoint des ministres chargés du commerce, des finances et des
  télécommunications », three ministers. Whether that arrêté was ever
  published decides whether a purely electronic facture with no wet stamp is
  legal. Not searched tonight; it is absent from the ministry of commerce
  listing read above, which is not proof.
