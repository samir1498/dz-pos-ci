---
title: 'What clinic software provides'
date: 2026-09-22
status: 'draft'
tldr: 'These products sell a patient dossier as the hub, an agenda, a
  prescription tied to a drug database, and a billing pipe wired into
  a reimbursement fund; the Algerian names (CabiSante, ClinixaPro,
  GCDOC, WinMed) are real but thinly documented, prices almost never
  published, while the French names (Weda, HelloDoc, Doctolib) document
  SESAM-Vitale in detail no Algerian site attempts for CNAS. The
  sharpest difference from a shop is that a cabinet fiche is a clinical
  history with no debt shape, while a shop fiche is a debt shape with
  no history, and Dinar has only the debt shape. A first doctor package
  needs the patient file, the agenda, a consultation note, a printed
  ordonnance and certificat, and a fee receipt, in that order, cutting
  the fund-as-payer case (tiers payant) first since nothing in Dinar
  can hold a receivable owed by a party who was never the buyer. The
  worked example says a module never ships alone: an appointment needs
  a patient row, so the first module is patients-and-appointments
  together, joined to the shared users, shop and audit tables by
  foreign keys, never by changing those tables.'
---

This is desk research written 2026-09-22 by the session; no clinic
product named below was installed, trialled, or demonstrated, and no
doctor was consulted. A claim about a product is what its own site
or documentation says, cited inline; a claim carried by a reseller,
comparison site, or partner blog is marked as such. Read first, in order:
`context/research/20260922-a-doctors-cabinet-day-on-paper.md`
(an Algerian cabinet's day, desk research) and
`context/research/20260922-the-paper-test-a-consultation-in-the-document-model.md`
(the nineteen places a consultation tears Dinar's document model, against
`main` at commit `134a788`). What follows treats those pages as settled and
does not re-derive them; a cabinet would correct all of this in ten minutes
at the reception desk.

## What these products sell

The Algerian market is not thin in names, it is thin in published
detail. GCDOC and WinMed each call themselves Algeria's top cabinet
software without naming a single feature (https://gcdoc.dzdoc.com/,
https://biginformatique.com/produits/gestion-de-cabinet-medical).
CabiSante is the best documented: a patient dossier holding the ordonnance,
documents, appointments and payment in one window, a caisse-and-comptabilite
module, a waiting-room screen for the assistant profile, an Algerian
drug database updated daily, and a standalone or networked mode
(https://cabisante.com/). ClinixaPro names CNAS, RGPD and ANPDP conformity and
over 500 features across 20-plus modules, including automatic CNAS billing,
PACS/RIS imaging, and a laboratory module (https://www.clinixapro.com/), a
clinic's shopping list, not a solo cabinet's. EasyClinic and Hakim-DZ claim
wide Algerian use (https://easyclinic.app/, https://hakim-dz.com/fr/cabinets)
without saying whether they run offline or only hosted. On price, only one
figure turned up, a classified listing on Ouedkniss quoting 60 000 DA for
"Wittisoft Medical Office", a reseller's asking price, unverified.

The French market documents far more, because its paperwork, SESAM-Vitale
and the feuille de soins electronique, is the closest analogue to
an Algerian médecin's feuille de soins today. Weda is fully online:
patient dossier, HAS-certified prescription aid, SESAM-Vitale billing,
an agenda, specialty templates such as a connected ECG for cardiology,
and paid monthly add-ons for teletransmission, a drug database and online
booking (https://weda.fr/logiciel-medecin-generaliste, https://weda.fr/).
HelloDoc does electronic feuille de soins teletransmission with a
financial dashboard, a prescription aid linked to Vidal Expert and
Claude Bernard, and a national identity check since its Segur referencing
(https://www.cgm.com/fra_fr/tous-nos-produits/logiciels-metiers/decouvrir/hellodoc.html).
MediStory (https://medistory.com/) and AlmaPro (https://www.almapro.org/) are
smaller record systems, the latter non-profit and DCI-prescribing. Doctolib's
practice side bundles an agenda with reminders, a scanned dossier, feuille de
soins and SESAM-Vitale billing (https://info.doctolib.fr/medecin-generaliste/);
a comparison site, not Doctolib's own page, reports a price near 135 euros
a month (https://www.lonasante.com/doctolib/, unverified). SMS reminders
across this market are a third-party add-on (SMSFactor, Julie, e-Agenda,
Mobikap), never a bundled feature.

Open source shows the module shape without a sales pitch. GNU Health,
built on Tryton and a UN-recognized Digital Public Good, covers
registration, epidemiology, a laboratory system and billing, and can
run as a single-node record system for a small clinic or scale to a
hospital, per its Wikipedia entry and an opensource.com interview,
not read directly here (https://en.wikipedia.org/wiki/GNU_Health,
https://opensource.com/health/13/3/interview-luis-falcon-gnu-health).
OpenEMR bundles health records, practice management, scheduling
and billing in one codebase with a multi-provider calendar
(https://www.open-emr.org/wiki/index.php/OpenEMR_Features). Odoo is the case
Anouar names directly, and a real clinic install is a stack, not one module:
the core Appointments app for booking (https://www.odoo.com/app/appointments),
Contacts holding the patient as a party record, and Invoicing turning a
consultation into an ordinary invoice, a breakdown from partner-implementation
blogs rather than Odoo's own documentation, marked accordingly
(https://erpixel.com/erp-for-clinics-and-medical-centers/). The clinical
layer on top comes from a separate module: the Odoo Community Association's
vertical-medical extends Contacts with patient data and a medical-center
entity (https://github.com/OCA/vertical-medical), and the app store lists
paid clinic modules bundling appointments, records and prescriptions in one
add-on (https://apps.odoo.com/apps/modules/19.0/medical_clinic). The shape
every source agrees on is core ERP plus one clinical module, the split Anouar
wants for Dinar.

## Feature inventory

The patient file is the hub every product builds around: identity, history,
documents, appointments and payment in one record (CabiSante, HelloDoc's DPI,
Doctolib, Weda, OpenEMR and GNU Health's EHR core, Odoo's Contacts extended
by the OCA medical module). The appointment book is a calendar with reminders
(Doctolib, Weda's paid add-on, OpenEMR's multi-provider calendar, Odoo's
Appointments app); CabiSante's separate waiting-room screen is a walk-in
queue, not a booked slot. The consultation and its acts show up as specialty
templates, since the act varies by specialty: Weda's connected-ECG, audiogram
and pregnancy-calendar modules.

Prescriptions are tied to a certified drug database checking interactions
and contraindications (Weda, HelloDoc and AlmaPro against Vidal Expert or
Claude Bernard under an HAS-certified prescription aid; CabiSante against
a comparable Algerian drug database updated daily).

Billing and the fee receipt, in France, means electronic feuille de soins
teletransmission through SESAM-Vitale with a financial dashboard that
doubles as reporting (HelloDoc, Doctolib, Weda's add-on, Odoo's pivot views);
CabiSante's caisse-and-comptabilite module tracks cash with no electronic
CNAS link this pass could confirm. Third-party reimbursement is that same
pipeline seen from the fund's side; ClinixaPro claims automatic CNAS billing
but not what it transmits or to whom, unverified beyond the vendor's phrase.

Medicine or supply stock and lab or imaging results both sit at clinic or
hospital scale, not solo cabinet: Odoo's Inventory tracks consumables by batch
and expiry, GNU Health has pharmacy and laboratory modules, ClinixaPro names
PACS, RIS and its own laboratory module, OpenEMR supports coded lab results;
nothing surveyed sells either as a solo cabinet's feature, consistent with
a généraliste who orders exams elsewhere.

Reminders by SMS are a third-party add-on across the French market
(SMSFactor, Julie, e-Agenda), claimed by none of the Algerian products
surveyed. Multi-practitioner support appears in CabiSante (several doctors
at once), Odoo (staff per service), OpenEMR (multi-provider calendar),
and AlmaPro and Weda, built for shared practices.

## The three-way sort

Dinar already has, in the part every trade shares: a sign-in and staff
list in `users` (`schema.rs:96`), role column, PIN hash, password hash,
failure counting and a lockout timestamp, the same authentication a
multi-practitioner cabinet needs; role gating on API routes rather than a
standalone permissions table, since this pass found none and the paper-test
page names permissions as untouched by the consultation test; an audit
trail in `audit_log` (`schema.rs:207`) recording shop, user, action,
entity and a before-and-after snapshot, the traceability the reporting
feature above wants; versioned `settings` and freeform `preferences`
(`schema.rs:46,74`); a backup export under `crates/api/src/routes/backups.rs`;
a money kernel of checked centime arithmetic under `crates/core/src/money/`;
a printing engine under `crates/core/src/print/`; and phone pairing in
`pairing_tokens`/`paired_devices` for a second device on the LAN.

Dinar has it in a shop shape a cabinet would need differently: the
`customers` fiche (`schema.rs:222`) could become the patient's identity
row, but `party_kind` is only `company` or `consumer` with no value for
a fund, and `credit_limit_centimes`/`warn_threshold_centimes` assume
the party buys on credit, the reverse of a fund paying after the fact
(tears 13-14). `documents`/`document_lines`, the ticket, could hold a
reçu's total, but its seven-value `kind` enum has no fee-note variant,
`product_id` never joins since a consultation names no product, `qty_milli`
has no natural value, and the stamp formula runs on the unresolved premise
that a médecin's reçu is a "facture" (tears 1, 2, 3, 7). `stock_movements`,
non-nullable to a product and written once per sale line inside the document's
own transaction (`crates/core/src/services/sales.rs:1-7, 228-1061`), assumes
every billed line moves a shelf item, which a consultation never does (tear
4). `debt_ledger`/`supplier_ledger` record what a customer owes the shop or
the shop owes a supplier; a fund owing the cabinet for a conventioned visit
is a third direction neither can name (tears 15-17). And Dinar's `shifts`
and `cash_refunds` already close out a cash drawer across a day, the shape
CabiSante sells separately as its caisse-and-comptabilite module.

Dinar has nothing like it, and the doctor module would bring: the clinical
history itself, since a shop fiche's balance and credit limit hold none of
it; the appointment book and its reminders; a waiting-room queue scoped to
an assistant role; a free-text consultation note and specialty templates;
a prescription checked against a drug database; certificats médicaux;
the feuille de soins as its own claim object, carried by the patient to a
fund rather than a fiscal document in Dinar's own series; the fund-as-payer
relationship itself, a receivable held against a party that was never the
buyer (tears 17-19); lab and imaging result intake; and SMS reminder delivery.

## The minimum first package

A first doctor package worth sending to a real cabinet would need, in this
order: a patient file holding identity and a short history, since every other
screen points at it; an appointment book covering both a walk-in queue and a
booked slot, since the cabinet-day draft found both in use with neither sourced
as more common; a consultation record, a free-text note against the visit, so a
visit is something the file can recall later; printed output for an ordonnance
and a certificat médical, plain documents with no total and no TVA line,
cheap and used on most visits; a fee receipt for the cash paid at the desk,
reusing Dinar's money kernel and printing engine once a comptable resolves
the TVA and stamp questions the paper-test page left open, or shipping as
a plain non-fiscal receipt otherwise; and a feuille de soins printed and
stamped for a CNAS-affiliated patient, since the cabinet-day draft calls
"paid now, claimed later" the ordinary shape for a médecin non conventionné.

The one thing to cut first is the fund-as-payer case, tiers payant, where
a conventioned médecin is settled directly by CNAS. The paper-test page
calls it the harder second case, needing three tables to change shape at
once, `customers`, `debt_ledger` and `supplier_ledger`, none with a home
for a receivable owed by a party who was never the buyer. A first package
can ship every ordinary visit, cash paid in full with a claim filed later,
without Dinar ever holding that receivable.

## A worked example: the appointment book

Take the appointment book, since it touches no centime and asks no fiscal
question, the cleanest first feature against the plan's split of a shared
core and a module per trade. It cannot ship alone: an appointment is a
booking against a person, so it needs a `patients` table, since the tear list
already ruled out bending `customers` into that role over its credit-limit
and party-kind columns. The first module is really patients-and-appointments
together, one package.

The module brings its own screens, a day view and, following CabiSante's
example, a walk-in queue scoped to an assistant role rather than the doctor's
own; its own permissions, verbs like booking and cancelling that a shop's till
never needed and that the shared role gate has to learn without changing how
it works for a shop; its own tables, `patients` and `appointments`, versioned
by migrations shipped inside this customer's package, never altering a shared
table; and its own upgrade path, running alongside core's without assuming
it runs first or last.

Where it joins the shared tables, it joins by ordinary foreign key:
`appointments.practitioner_user_id` points at `users.id`, the same staff list
a shop's till uses; `appointments.shop_id` carries the same per-shop scoping
every shared table already has; logging a booking into `audit_log` costs
nothing structurally, since `entity` is a free-text column and "appointment"
is just a new string. Settings would hold a cabinet's opening hours the
way it holds a shop's tax regime, and printing would render the day's list
through the same engine a ticket uses. Money arithmetic stays untouched
unless a booking carries a deposit, an edge case a first package can leave out.

Three questions this leaves open. Whether a patient's identity belongs
in the module's own `patients` table or a shared `customers` row typed
differently is unresolved: Anouar's claim that a customer and a patient are
the same module with different strings survives the identity columns and
fails at the money columns. How the shared role gate learns a module's
own permission names at build time, when each package is assembled
separately, is not answered here. And how a module's migrations sequence
against core's, so a package still passes the upgrade path Dinar tests in
`crates/api/tests/upgrade_from_a_previous_version.rs`, is a question the
split has to settle before a second module gets added.
