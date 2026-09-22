---
title: 'A doctor''s cabinet day on paper'
date: 2026-09-22
status: 'draft'
tldr: 'A médecin libéral in Algeria sets a cash consultation fee freely, with no décret fixing it and a result private doctors themselves call a "désordre tarifaire"; what CNAS or CASNOS pays back is a tarif de référence unrelated to that fee and, by the Ordre des médecins own president, not updated in years. The two fiscal facts that matter most: médecins are professions libérales réglementées, excluded from the IFU forfait since the 2022 finance law and so declared on the régime réel as bénéfices non commerciaux for IRG, and the droit de timbre is written for invoiced commercial transactions, so whether it falls on a médical fee note at all is unsourced rather than settled. A cabinet's day is a chain of acts, not a basket of goods, and most of what happens (the exam, the ordonnance, the appointment, the dossier) has no shop equivalent. What is the same transaction as a shop's is narrow: a cash or card payment received over a counter, a paper handed back for it, and a debt when the payer is a fund rather than the patient in front of you.'
---

# A doctor's cabinet day on paper

This is desk research written 2026-09-22 by the session, not Samir's own
research. Samir's own research, when he does it, confirms or replaces
everything below. No cabinet, no doctor and no patient has been consulted
for this page: dz-pos has at least been built against a real till and a
real Algerian invoice, but nobody involved has stood behind a cabinet's
reception desk. A cabinet would run its day in something like the order
below; a cabinet would also correct half of it in ten minutes of
conversation, which is why D2 exists before any design does.

## Before the patient is in the room

A cabinet opens with a secretary or a nurse at the front desk, sometimes
the doctor alone with no staff at all. Two shapes are common, a walk-in
queue in arrival order and an appointment book, and nothing sources how
common each is in Algeria specifically; that split is a labelled French
analogue, not an Algerian observation. What is Algeria-specific and
sourced is a CNAS scheme letting a patient register with a médecin
traitant conventionné, which changes what the desk does before the exam
starts (https://cnas.dz/en/attending-physician-agreement/,
https://www.algerie-focus.com/cnas-avantages-du-nouveau-dispositif-de-conventionnement-du-medecin-traitant/).

**Reception.** The secretary, where one exists, asks who the patient is,
whether they have been seen before, and whether they are covered by
CNAS, CASNOS, or nothing at all. This is where a cabinet's day first
forks from a shop's: a shop asks who is buying only on credit or when a
facture is asked for; a cabinet asks who is paying and through what fund
on every visit, because the answer decides which paper the patient
leaves with, not only how the money is recorded.

**The dossier.** A returning patient's file, paper in most cabinets
still, holds past visits, conditions, allergies, current medication.
This has no shop equivalent: a Dinar customer fiche holds a name, a
balance and a credit limit, and a dossier's medical history is
something a shop counter never touches and dz-pos's schema should never
have a column for.

## The consultation

The doctor takes a history, examines, sometimes orders an analysis or an
imaging exam elsewhere by ordonnance, sometimes concludes in the room.
Nothing here is a line on a receipt the way a product on a shelf is: the
thing billed is an act of variable length decided in the room, never
priced per minute or per component the way a shop prices a basket. A
généraliste's consultation and a spécialiste's are different acts with
different fees, and no décret found in this pass names either fee; both
are the doctor's own decision, a point returned to below.

**What is printed.** Out of one visit can come, in any combination: an
ordonnance naming medications or further exams, a certificat médical
(fitness for work or school, a medico-legal certificate), and, when the
visit is billed against a fund, a feuille de soins the doctor fills and
stamps for the patient to claim reimbursement later
(https://www.demarchesdz.com/assurance-maladie-regime-securite-sociale/,
https://zoomalgerie.com/malade-vacances-algerie-remboursement-soins/).
None of the three is a fiscal document the way Dinar's `facture` or
`ticket` is: an ordonnance and a certificat carry no price at all, and a
feuille de soins is a claim form owned by the patient, not a numbered
series the cabinet keeps a copy of. What the cabinet hands back as proof
of payment, a reçu or a note d'honoraires, is the one paper actually
about money changing hands, and this pass could not source whether
Algerian law requires it to take any particular form for a médecin
specifically. Décret 05-468 and loi 04-02, the texts Dinar's own facture
rule is built on, speak of a "prestation de services" between
"opérateurs économiques" and consumers; a consultation is a prestation
de services in the plain sense, but médecins are classed a profession
libérale non commerciale in every fiscal source found here
(https://lentrepreneuralgerien.com/impots/item/139-tout-sur-les-professions-liberales-en-algerie),
and nothing found says whether that carries them out of loi 04-02's duty
to hand over a ticket or a facture. That is left open, exactly the kind
of gap docs/features.md sends to a comptable rather than guesses at.

## What is owed, and by whom

A shop's débiteur is almost always the person at the counter; a
cabinet's often is not, and that is the real fork.

**Paid now, in full, out of pocket.** The ordinary case for a médecin
non conventionné: the fee changes hands at the desk, in cash or
occasionally by card, and nothing is owed once the patient leaves. This
is the one shape that reads exactly like a shop's cash sale.

**Paid now, claimed later from a fund.** The patient still pays in full
but leaves with a feuille de soins, an ordonnance and, for medication or
analyses, further factures. They later bring the stack to their own
caisse, CNAS for a salarié or CASNOS for a non-salarié, and are
reimbursed at a tarif de référence unrelated to what they actually paid
(https://www.cleiss.fr/docs/regimes/regime_algerie_salaries.html,
https://www.algerie-focus.com/securite-sociale-vers-le-remboursement-des-consultations-medicales/).
The debt is real but sits between the patient and their caisse: the
cabinet is paid in full at the desk and is owed nothing further. What is
owed downstream, by a party never in the room, is the fund's obligation
to the patient, a claim the cabinet neither carries nor collects.

**Tiers payant.** Where a médecin is conventionné, the fund can settle
with the doctor directly and the patient pays nothing or a reduced part,
the shape a mutuelle's third-party payment takes in France, named here
as a French analogue since no Algerian source describes the doctor's
side of the settlement. What is sourced for Algeria is a
"conventionnement du médecin traitant" scheme paying 400 DA per
généraliste consultation and 600 DA per spécialiste, and tiers payant
already running on medication under a threshold or within a patient's
first two ordonnances in three months
(https://www.demarchesdz.com/assurance-maladie-regime-securite-sociale/).
Whether the cabinet is owed by the fund for any gap is not sourced here,
and would be a receivable a shop never carries: a shop is never owed by
a third party for a sale it has already handed the goods over on.

**A mutuelle on top, and the size of the gap.** Where CNAS or CASNOS
covers only a fraction, a complementary mutuelle, often tied to a
public-sector employer, tops up the difference as a second
reimbursement claim against the same feuille de soins, another debt
that never touches the cabinet; CNAS reimbursement for privately
conventioned care is reported at often 30 to 50 percent of the real
price (https://www.sakinadz.com/blog/guides/assurance-sante-expatries-algerie-2026).
Older figures show how wide that gap runs: Algerie360, on Oran doctors
raising prices, put CNAS reimbursement at 50 DA for a généraliste
consultation and 100 DA for a spécialiste, unmoved for years while real
fees rose up to 40 percent
(https://www.algerie360.com/les-medecins-prives-augmentent-les-prix-des-consultations/,
dated 2017, so the DA figures are old, though the shape is not). A 2023
piece quotes the Ordre national des médecins' president still calling
reimbursement outdated, a figure still cited as 100 DA
(https://37degres.dz/index.php/2023/09/12/augmentation-des-honoraires-des-medecins-prives-une-reglementation-est-necessaire/).
No décret was found fixing what a médecin non conventionné may charge;
the fee is the doctor's own decision, and "désordre tarifaire" is the
profession's own word for the result.

## Fiscal rules for a médecin libéral

**TVA.** Sources conflict and this pass could not resolve it, which is
itself worth carrying forward. One reading, citing CTCA art. 23, is that
actes médicaux (médecins et vétérinaires) are taxed at the reduced 9
percent rate (https://merbouhi.com/blog/tva-algerie-19-9-exonere.html).
A second claim from the same source says actes médicaux are exonérés
outright, contradicting its own longer passage; the CTCA itself
(https://www.douane.gov.dz/IMG/pdf/code_des_taxes_sur_le_chiffre_d_affaire.pdf)
was not read article by article here. Underneath both: professions
libérales under IFU are not subject to TVA at all, but médecins have
been excluded from IFU since 2022, so that shelter does not apply to
most of them. Reduced rate or exemption is unresolved here, a comptable
question exactly like every TVA row in docs/features.md's own table.

**Droit de timbre on a cash consultation.** Code du timbre art. 100-I,
the article Dinar's own stamp rule is built on, taxes cash settlement of
an invoiced transaction, and 2025 finance-law guidance on liberal
professions says that where there is facturation, the stamp is due at
the ordinary rates and borne by the client
(https://www.intellixgroup.com/blog/algerie-droit-de-timbre-2025-loi-de-finance-journal-officiel-probleme-et-solutions).
No source found here names médecins specifically, either way. Since a
médecin's reçu is not established above to even be a facture in the
décret 05-468 sense, whether the stamp code's "opération facturée"
language reaches it is unverified, not assumed true by analogy.

**IRG and the forfait.** Médecins are professions libérales
réglementées, excluded from IFU by the 2022 loi de finances; a mid-year
loi de finances complémentaire restored IFU for avocats and notaires
after their protests, but médecins stayed on the régime réel
(https://algerie-eco.com/2022/07/27/ifu-le-gouvernement-prevoit-de-reinstaurer-leligibilite-des-professions-non-commerciales-plfc-2022/,
https://www.reporters.dz/lfc-2022-limpot-forfaitaire-unique-reintroduit-pour-les-professions-liberales/,
https://lamacta.com/blog/regime-ifu-algerie-2026-conditions/, stating
regulated liberal professions, avocats, médecins, architectes, remain
excluded as of 2026). On the régime réel a médecin's income is a
bénéfice non commercial for IRG, declared through a déclaration
prévisionnelle in June and a déclaration définitive the following
January (professions libérales overview above). Médecins have publicly
called the combined weight of IRG and CASNOS discriminatory, the
profession's own framing rather than a verified comparison to a shop
(https://www.tsa-algerie.com/irg-et-casnos-discrimination-et-hyper-imposition-des-medecins-liberaux/).
Professions libérales are also reported excluded from the taxe sur
l'activité professionnelle a shop pays on turnover
(https://imagine-experts.com and the overview above), which if correct
makes a cabinet's fiscal shape a different regime rather than a shop's
with different numbers.

**CASNOS: the doctor's own cover, not the patient's.** A médecin
libéral is a non-salarié paying into CASNOS for their own retirement,
health and family cover, not CNAS: a flat 15 percent of declared income
against an assiette reported at 216 000 to 4 320 000 DA a year, due by
30 June with penalties from 11 percent after
(https://www.demarchesdz.com/affiliation-casnos/,
https://coffice.dz/blog/cnas-casnos-guide-complet-securite-sociale). This
is structurally identical to a shop owner's own CASNOS duty, and worth
keeping apart from what CNAS or CASNOS owes a patient above: one is the
cabinet paying into its own cover, the other a fund reimbursing someone
who was never the cabinet's debtor. Whether a receipt is legally
required at all is unresolved the same way as under "what is printed"
above: nothing found states whether a médecin libéral counts as an
"opérateur économique" under the general facturation law or is carved
out of it the way it is carved out of TAP and, under IFU, of TVA.

## Which parts of a shop's day and a cabinet's day are the same transaction

A payment received in cash or by card, with a paper handed back for it,
is the same transaction in both places: money changes hands over a
counter, now, for something already delivered. A debt owed by the
person in front of you is the same shape too: a shop's credit sale and a
cabinet's unpaid balance, where either exists, are both a ledger row
against one named party who pays later. An audit row, a timestamped
record of who did what, is infrastructure rather than domain and a
cabinet needs it for the same reasons a shop does, none of them specific
to goods or to medicine.

What is not the same transaction is most of the day. A patient record is
a clinical history with no analogue in a fiche's balance and credit
limit: a shop has no business holding allergy information and a
dossier has no business being modelled as a debt ledger. An appointment
reserves the doctor's time, not stock, and moves no inventory count or
document number. An act with no stock behind it breaks the assumption
built into a `sale`: Dinar's sale line names a product and moves a stock
movement, and a consultation has neither a product nor a unit that
leaves a shelf. And a reimbursement claim to CNAS, CASNOS or a mutuelle
is not a document the cabinet issues into its own series at all: it is
a claim the patient carries away and files elsewhere, against a fund
the cabinet has no ledger relationship with unless it is conventioné
and paid through tiers payant, in which case the fund becomes a payer
the cabinet is owed by, a third shape neither a cash sale nor a credit
sale has room for today.

## What D3 can take from this

One concrete billed consultation, sourced as far as this pass could go
and invented no further than it has to be: a patient walks into a
médecin généraliste's cabinet in Algiers with no appointment, is seen
the same afternoon, and pays 1 500 DA in cash at the desk when the
consultation ends. That figure is from memory, unverified, offered only
as a plausible order of magnitude for an unconventioned généraliste's
fee today, since no current, dated figure for an ordinary cabinet's real
fee turned up in this pass, only the CNAS conventioned tariff of 400 DA
and old reimbursement figures near 50 to 100 DA. The patient leaves with
three papers: an ordonnance naming two medications, a feuille de soins
filled and stamped because the patient is a salarié affiliated to CNAS
and intends to claim back what the tarif de référence allows, and a
reçu for the 1 500 DA whose fiscal status this page has not established
either way. The cabinet owes nobody anything from this visit: it was
paid in full on the spot, and any debt from here runs between the
patient and CNAS, a fund the cabinet never bills and is never owed by.
The one obligation running the other way, outward from the cabinet, is
CASNOS: a fixed share of the doctor's own declared annual income, due
once a year, unrelated to this single visit's 1 500 DA. D3's paper test
can try to express the payment and the reçu in `documents` and its
lines the way a `ticket` is expressed today, and should expect the
ordonnance and the feuille de soins to have no field to land in at all:
neither is a fiscal document with a total, a TVA line or a stamp, and
forcing either into `documents` is the first place the model should
visibly tear.
