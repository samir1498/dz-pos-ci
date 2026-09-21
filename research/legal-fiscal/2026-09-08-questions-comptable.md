# Questions pour le comptable (2026-09-08)

Écrit le 2026-09-08 avec huit questions. Un ajout daté du 2026-09-21, en bas
de page, porte les sept lectures que le logiciel a fixées depuis sans qu'un
texte ne les tranche ; la liste à jour de chaque hypothèse est le tableau
« Fiscal rules: current assumptions » de `docs/features.md`.

Une page, huit questions. Chacune donne la règle telle que nous l'avons lue,
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
précédent. « La facture régulièrement annulée doit faire l'objet d'une mention
"facture annulée" inscrite clairement en diagonale. » Le texte s'arrête là.
Source : décret exécutif 05-468 du 10 décembre 2005, art. 10.
Notre lecture : une facture annulée garde son numéro, parce que le réattribuer
casserait la série ininterrompue. C'est une déduction, pas une phrase du décret.
Pourquoi : cette déduction tient-elle en pratique, ou voit-on des numéros
réutilisés après annulation ? Et le texte ne dit pas si la série repart à 1 au
1er janvier : la numérotation est-elle continue d'une année sur l'autre, ou
remise à zéro avec l'année dans le numéro ?

**Décidé le 2026-09-10 :** la série repart à 1 chaque 1er janvier (Samir,
2026-09-10 ; le comptable confirme que c'est la pratique courante, R8). Voir
`docs/features.md`, ligne « Numbering ». La réutilisation d'un numéro après
annulation reste ouverte : la ligne « Cancellation » de `docs/features.md`
la marque toujours R8.

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

## 8. Le montant en lettres : total TTC ou net à payer

Règle lue : la facture comporte le « prix total toutes taxes comprises, libellé
en chiffres et en lettres ».
Source : décret exécutif 05-468, art. 3.
Pourquoi : lorsqu'un droit de timbre s'ajoute à une facture réglée en espèces,
le net à payer dépasse le total TTC du montant du timbre. Nous écrivons
aujourd'hui en lettres le net à payer, timbre compris, comme les logiciels du
marché. Sur les factures que vous voyez, le montant en lettres reprend-il le
total TTC, ou le net à payer avec le timbre ?

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

## Ajout du 2026-09-21 : sept lectures fixées depuis, à confirmer

Le logiciel a grandi de la caisse simple à la facture, l'avoir, l'annulation, le
stock et la marge. Chaque fois qu'aucun texte ne tranchait, nous avons choisi
une lecture, l'avons fixée par un test nommé, et l'avons marquée « à confirmer »
dans `docs/features.md`. Ce sont ces sept-là. Même forme que plus haut : la
règle telle que nous l'appliquons, la source ou son absence, et la question.

## 9. La base du droit de timbre : le total TTC avant le timbre lui-même

Règle appliquée : le timbre se calcule sur le total TTC de la facture, c'est à
dire le montant avant que le timbre ne s'y ajoute. Le net à payer est le total
TTC plus le timbre.
Source : le code du timbre 2026, art. 100-I, dit « montant » et ne nomme pas
de base.
Pourquoi : la base est-elle bien le total TTC, ou le timbre se calcule-t-il sur
une base qui l'inclut déjà, ce qui changerait le résultat d'une tranche sur les
montants proches d'un seuil ?

## 10. Le timbre n'est jamais restitué : sur l'avoir, et sur l'annulation d'un ticket réglé en espèces

Règle appliquée : un avoir ne porte pas de droit de timbre et ne restitue pas
celui de la facture qu'il crédite ; le total des avoirs sur une facture ne
dépasse jamais son total TTC. Depuis le 2026-09-21, un ticket en espèces annulé
peut être remboursé au client en espèces, et ce remboursement porte sur le net à
payer moins le timbre : le client récupère le prix des marchandises, pas le
timbre versé.
Source : le code du timbre taxe le paiement et ne dit rien d'une contre-passation.
Pourquoi : c'est une lecture, pas une phrase du code. Lorsqu'une vente en
espèces est annulée le jour même ou la semaine suivante et que le client est
remboursé, la pratique restitue-t-elle le timbre au client, le garde-t-elle
pour le Trésor, ou l'affaire ne se pose-t-elle pas parce que le ticket annulé
n'est jamais déclaré ?

## 11. Annuler un ticket de caisse sans document numéroté

Règle appliquée : un ticket ou une facture s'annule, jamais ne s'efface : il
garde son numéro et sa ligne, et enregistre quand, par qui et pourquoi. Une
facture qui avait mis de l'argent sur le compte d'un client est défaite par un
avoir entier ; un ticket à crédit est défait par une seule écriture au grand
livre du client, sans numéro pris dans la série des avoirs ; un ticket ou une
facture en espèces ne devait rien à personne, seules les marchandises reviennent.
Source : le décret 05-468 régit la facture et ne dit rien de la contre-passation
d'un ticket de caisse.
Pourquoi : défaire un ticket sans émettre de document numéroté tient-il en
pratique, ou un contrôle attend-il un avoir même pour un ticket ?

## 12. L'arrondi d'une ligne vendue au poids ou au volume

Règle appliquée : une quantité est un nombre entier de millièmes d'unité
(1 500 pour 1,5 kg), le montant brut de la ligne est le prix unitaire multiplié
par la quantité en millièmes, divisé par mille, arrondi au centime une seule
fois, au plus proche et à demi loin de zéro, avant toute remise de ligne.
Source : aucun texte ne dit comment une ligne pesée s'arrondit.
Pourquoi : les balances et les logiciels que vous voyez arrondissent-ils la
ligne au centime, au dinar, ou le prix au kilo est-il déjà exprimé de façon à
ne jamais produire de fraction ?

## 13. Les remises sur un avoir partiel, et la somme des avoirs

Règle appliquée : un avoir partiel crédite la même part de la remise de ligne et
de la remise globale qu'il crédite de la ligne et du panier, chaque part
arrondie au centime inférieur, parce qu'une remise est ce que le client n'a pas
payé et qu'arrondir au supérieur créditerait un centime que personne n'a versé.
Les avoirs sur une même facture se somment à cette facture moins le timbre ;
l'avoir qui solde la facture est la facture moins les avoirs précédents, champ
par champ, de sorte que la somme tombe juste même si les tranches, prises une à
une, arrondissent d'un centime de part et d'autre.
Source : aucun texte ne dit comment une remise se répartit sur une
contre-passation partielle, ni comment une contre-passation en plusieurs fois
s'arrondit.
Pourquoi : c'est la somme que vous lisez qui tranche. Un contrôle vérifie-t-il
que les avoirs d'une facture se somment exactement à elle, ou tolère-t-il un
centime d'écart né des arrondis par tranche ?

## 14. Le coût des marchandises vendues et la base de la marge

Règle appliquée : ce qu'une unité a coûté est le coût au moment de la sortie,
écrit sur le mouvement de stock de la vente, jamais le prix de revient de la
fiche, qui est celui de la dernière livraison et bouge à chaque achat. Un avoir
ou une annulation remet les marchandises au coût de la vente qu'il défait. La
marge du tableau de bord se lit sur les documents encore debout, et son chiffre
d'affaires est le HT des lignes moins la remise globale, parce qu'une remise est
un revenu jamais encaissé.
Source : aucun texte ne prescrit une valorisation de stock ni une base de marge
à un commerce qui tient ses propres comptes.
Pourquoi : la valorisation au coût de sortie (et non au dernier coût, ni au
coût moyen pondéré) est-elle celle qu'un comptable attend d'une supérette au
réel, et la marge lue nette de remise est-elle celle que vous calculez ? Un
chiffre de gestion lu d'une façon à la caisse et d'une autre dans les livres est
l'erreur que nous voulons éviter.

## 15. L'article d'imposition sur la facture

Règle appliquée : la facture porte le RC et le NIS des deux parties (décret
05-468, art. 3), le NIF (loi 04-02, art. 34 ; LF 2006, art. 42) et l'article
d'imposition du vendeur, celui-ci parce que toutes les factures en circulation
le portent et non parce qu'un texte l'exige.
Source : aucun texte ne fait de l'AI une mention de la facture ; le CIDTA
art. 183 ter demande au grossiste de tenir l'AI de chaque client pour son état
clients.
Pourquoi : cette lecture tient-elle ? Faut-il l'AI de l'acheteur sur une facture
à une société, ou celui du vendeur suffit-il ?
