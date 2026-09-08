# Questions pour le comptable (2026-09-08)

Une page, sept questions. Chacune donne la règle telle que nous l'avons lue,
la source, et la raison de la question. Les textes cités sont dans
`research/legal-fiscal/sources/`. Nous cherchons la pratique constatée sur les
factures réelles, pas une confirmation du texte.

Contexte en une phrase : nous écrivons un logiciel de caisse pour une supérette
algérienne, tous les montants sont en centimes entiers, et chaque règle fiscale
doit être fixée par un test avant d'être programmée.

## 1. Le demi-dinar sur la tranche à 1,50 DA

Règle lue : le droit de timbre se calcule par tranche de 100 DA ou fraction de
tranche, à 1 DA jusqu'à 30 000 DA, 1,50 DA jusqu'à 100 000 DA, 2 DA au-delà.
Source : code du timbre 2026, art. 100-I, et circulaire n° 14/MF/DGI/LF.2025 du
5 mars 2025, dont les trois exemples tombent tous sur un nombre pair de tranches.
Pourquoi : 301 tranches à 1,50 DA donnent 451,50 DA. Le droit doit-il être arrondi
au dinar, et dans quel sens, ou bien s'inscrit-il en centimes sur la facture ?

## 2. L'arrondi de la TVA sur la facture

Règle lue : aucun texte ne fixe l'arrondi de la TVA sur une facture. L'art. 324
du CIDTA arrondit la base à l'unité ou à la dizaine de dinars inférieure et les
cotisations à la dizaine de centimes la plus proche, mais il vise la déclaration.
Source : CTCA 2026 art. 80bis, qui renvoie au CIDTA 2026 art. 324.
Pourquoi : nous calculons la TVA une seule fois par taux, sur le sous-total HT du
groupe, au centime. Les factures que vous voyez arrondissent-elles au centime ou
au dinar, et par groupe de taux ou ligne par ligne ?

## 3. La part des commerces à l'IFU

Règle lue : relèvent de l'impôt forfaitaire unique les personnes physiques dont
le chiffre d'affaires annuel ne dépasse pas 8 000 000 DA, et un assujetti à l'IFU
ne peut pas faire figurer la TVA sur ses factures.
Source : CIDTA 2026 art. 282 ter, CTCA 2026 art. 2-12 et art. 64.
Pourquoi : 8 000 000 DA par an font environ 22 000 DA de ventes par jour. Les
supérettes que vous suivez sont-elles majoritairement à l'IFU ou au réel, et
change-t-on de régime en cours d'année ou seulement au 1er janvier ?

## 4. La série des numéros et le changement d'année

Règle lue : la facture est régulière lorsqu'elle provient d'un facturier ou d'un
procédé informatique comportant « une série ininterrompue et chronologique de
factures », et un nouveau carnet ne peut être entamé qu'après épuisement du
précédent. Une facture annulée garde son numéro et porte la mention « facture
annulée » en diagonale.
Source : décret exécutif 05-468 du 10 décembre 2005, art. 10.
Pourquoi : le texte ne dit pas si la série repart à 1 au 1er janvier. En pratique,
la numérotation est-elle continue d'une année sur l'autre, ou remise à zéro avec
l'année dans le numéro ?

## 5. Le timbre sur une facture à crédit réglée plus tard en espèces

Règle lue : le droit de timbre frappe les titres qui constatent des paiements ou
des versements de sommes, et les règlements par moyen de paiement électronique en
sont dispensés. Il n'est donc pas dû à l'émission d'une facture non réglée.
Source : code du timbre 2026, art. 100-I et art. 258 quinquies.
Pourquoi : lorsqu'un client règle en espèces une facture émise à crédit,
plusieurs semaines après, le timbre est-il dû, et sur quelle base : le total TTC
de la facture, ou seulement la somme effectivement versée ce jour-là ?

## 6. La quittance du règlement différé

Règle lue : le droit de timbre s'applique aux quittances, factures et tickets de
caisse qui constatent un paiement, ce qui suppose qu'un document soit émis au
moment du règlement.
Source : code du timbre 2026, art. 100-I, et circulaire n° 14/MF/DGI/LF.2025, qui
cite expressément « quittance, facture, ticket de caisse ».
Pourquoi : le commerçant remet-il un reçu ou une quittance au client qui vient
solder sa dette en espèces, et ce document porte-t-il lui-même le timbre, ou bien
le timbre reste-t-il porté sur la facture d'origine ?

## 7. La remise globale répartie entre les taux de TVA

Règle lue : le prix total toutes taxes comprises comprend tous les rabais, remises
et ristournes accordés lors de la vente, et la TVA se calcule par taux.
Source : décret exécutif 05-468, art. 5 et 6, et CTCA 2026 art. 21 et 23.
Pourquoi : un panier qui mêle du 19 % et du 9 % avec une remise globale oblige à
répartir cette remise entre les deux groupes. Nous prévoyons une répartition
proportionnelle au HT de chaque groupe, le reliquat de centimes allant au groupe
le plus important. Est-ce la pratique, ou la remise est-elle ventilée ligne par
ligne avant tout calcul de taxe ?

## Une remarque utile pour la question 1

Le code de l'enregistrement, art. 11, tel que modifié par l'art. 31 de la loi de
finances pour 2025, tranche le même genre de cas pour ses propres droits : « les
fractions inférieures à 0,5 DA étant négligées et les fractions égales ou
supérieures à 0,5 DA étant comptées pour 1 DA ». C'est un autre code et il ne
s'impose pas au timbre, mais c'est le précédent le plus proche que nous ayons
trouvé. Si la pratique applique la même logique au droit de timbre, dites-le,
nous la fixerons par un test.

## Ce que nous ferons de vos réponses

Chaque réponse est reportée dans la colonne Source du tableau des règles fiscales
de `docs/features.md`, avec votre nom et la date, et fixée par un cas de test
nommé. Une règle sans source ni test n'existe pas chez nous.
