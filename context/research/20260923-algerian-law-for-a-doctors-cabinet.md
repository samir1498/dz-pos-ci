---
title: 'Algerian law for a doctors cabinet: what the software must record, print, keep or refuse'
date: 2026-09-23
status: 'draft'
tldr: 'Every patient must have a dossier medical (Loi 18-11, Art. 26), and
  the professionnel de sante must keep it up to date (Art. 178). Medical
  secrecy (Art. 24) and the deontology codes secret professionnel
  (decret 92-276, art. 36 a 41) mean the software must protect the file
  against anyone without a legal reason to see it. A privately-issued
  ordonnance or certificat must be legible, identify the signer, and carry
  a date and the medecins own signature (decret 92-276, art. 56, found only
  in secondary sources, not confirmed against the primary text). Loi 18-11
  also creates a systeme national dinformation sanitaire that every health
  structure, public or private, must eventually integrate (art. 319 to
  323) - this is the biggest long-range constraint found, though no decree
  fixing the technical interface turned up. A remplacant needs his own
  authorisation and must sign under his own identity, never the titulaires
  (art. 166, 171, and an Ordre licence de remplacement). Psychotropes and
  stupefiants need a three-copy numbered ordonnancier (loi 04-18), which a
  plain software printout does not satisfy. Several concrete points stay
  not found: the retention period in years for a dossier medical, the
  patients right to a copy, what happens to the file when a cabinet
  closes, the CNAS arret-de-travail forms exact legal text, and whether
  practice-management software itself needs ANPP homologation.'
---

# Algerian law for a doctors cabinet: what the software must record, print, keep or refuse

Scope: only the rules that bear on what a doctors-cabinet module records,
prints, keeps or must refuse to do. A parallel piece of research covers who
inside the cabinet may see a note (secretary vs doctor); another covers
data protection (loi 18-07) and fiscal rules. Where a rule from those two
areas landed on a page read for this one, it is noted but not developed.

Method note on sourcing: most of the primary Algerian legal PDFs found
online are scanned images of the Journal Officiel with no text layer, so
they cannot be searched by machine, only read by a human. Two exceptions
extracted cleanly: the Journal Officiel N 46 issue carrying Loi 18-11
(a text-layer PDF hosted by snapo-dz.net), and a Direction de la Sante de
Saida compilation of ministry instructions and circulaires on private
practice (an old LZW-compressed PDF, also machine-readable once decoded).
Both were parsed programmatically for this page, article by article, and
the Journal Officiel PDF interleaves its two newspaper columns when read
this way, so a few passages are placed by content rather than by a
directly adjacent "Art. N." marker; those are flagged below. Everything
from the decret 92-276 (code de deontologie medicale) itself is a scanned
image with no text layer; its content here comes from secondary sources
(course PDFs, an Ordre-adjacent blog, search-engine synthesis of pages
that quote it) and is marked as such throughout.

## What the software must do, or must not do

Each line is marked **sourced** (an article or a named regulatory text
is quoted or cited, even if only via a secondary source) or **inferred**
(a reasonable conclusion from a sourced rule, not itself written down
anywhere found).

1. **Sourced.** Every patient must have a dossier medical. Loi 18-11,
   Art. 26: "Tout patient doit disposer d'un dossier medical." The clinic
   module's patient file satisfies this by existing; the law does not
   prescribe its format.
2. **Sourced.** Keeping that file current is a duty of the health
   professional, not just a courtesy. Loi 18-11, Art. 178, lists among the
   obligations of "professionnels de sante": "de tenir a jour le dossier
   medical du patient."
3. **Sourced, scope uncertain.** "Les structures et les etablissements
   publics et [...] prives de sante" must hold, for each patient, a
   "dossier medical unique informatise," and must ensure "la gestion et la
   conservation des archives medicales" (Loi 18-11, Art. 292). Whether a
   one-doctor cabinet counts as a "structure... privee de sante" under
   this article is not settled by the text found; Art. 308 does list
   "structures d'exercice individuel" as one category of private health
   activity, which points toward yes. If it applies, the law itself
   pushes toward a computerized record, not paper.
4. **Sourced.** Medical secrecy covers everything that reaches a health
   professional's knowledge and can only be lifted by the competent
   court (or, for a minor or an incapable person, at the request of the
   spouse or a parent). Loi 18-11, Art. 24: "le secret medical couvre
   l'ensemble des informations parvenues a la connaissance des
   professionnels de sante... [il] peut etre leve par la juridiction
   competente." The deontology code adds the same duty at the
   profession's own level (decret 92-276, secret professionnel, art. 36
   a 41, per two independent secondary summaries): it covers what the
   doctor "a vu, entendu, compris" or been told, survives the patient's
   death except to assert rights, and requires protecting "les fiches
   cliniques et documents" against "toute indiscretion." The software
   must not let anyone without a legal basis view, print or export a
   patient file.
5. **Sourced.** A liberal professional can be replaced (remplacement) for
   training, leave, or up to a year (renewable once) for health reasons,
   under conditions set by Art. 166; the remplacant "exerce sa profession
   sous son identite legale et demeure responsable de ses activites"
   (Loi 18-11, Art. 171). A ministry-instruction-level rule found in the
   Saida DSP compilation adds detail: the titulaire sends a written
   request to the Direction de la Sante de wilaya, the Section Ordinale
   gives its opinion first, and it is the Section Ordinale that "delivre
   une licence de remplacement." **Inferred:** any ordonnance or
   certificat produced while a remplacant is on duty must show the
   remplacant's own name and Ordre number, never the titulaire's, since
   the remplacant "demeure responsable."
6. **Sourced, secondary only.** Any ordonnance, certificat, attestation or
   document a doctor or dentist writes must be legible, allow
   identification of the signer, and carry a date and the doctor's
   signature. This is reported consistently as decret 92-276, Article 56,
   by multiple secondary sources reached through search, but the primary
   JORADP text could not be machine-read to confirm the article number
   directly, so it is flagged as secondary-sourced rather than
   quoted-from-original. **Inferred:** a software-typeset document already
   satisfies "legible" and "identifies the signer" better than
   handwriting; it still needs a date field and a place for the doctor's
   own signature (see point 12 on whether a printed signature suffices).
7. **Sourced.** Opening a private cabinet needs an "autorisation
   d'ouverture" or "autorisation d'exploitation" delivered by the wali.
   Loi 18-11, Art. 314, refers to "l'autorisation d'exploitation prevue a
   l'article [307]"; separately, the 1987 ministry Instruction
   n 00112/MSP/SG (found in the Saida DSP compilation) states plainly:
   "La decision portant autorisation d'ouverture de cabinet... prives est
   delivree par le wali." **Inferred:** the cabinet's settings screen is a
   reasonable place to record this authorization's reference and date,
   even if it is not printed on every document.
8. **Sourced.** Practicing privately requires "inscription au tableau" of
   the Ordre, with a personal registration number. This shows up twice:
   the association-contract templates in the Saida DSP compilation each
   require every associated doctor's "Numero d'inscription au tableau,"
   and a secondary academic summary of the Ordre's role calls that
   inscription "une condition generale indispensable pour l'exercice
   professionnel." **Inferred:** the practitioner's profile in the
   software should store this Ordre number, since printed documents
   (ordonnance, certificat) commonly carry it in practice even though no
   article was found making that mandatory in so many words.
9. **Sourced.** A solo cabinet is legally a "structure d'exercice
   individuel," one of several defined categories of private health
   activity alongside "structures d'exercice de groupe," private
   hospitals, private care/diagnostic establishments, medical-analysis
   laboratories, and licensed medical-transport structures (Loi 18-11,
   Art. 308). Group cabinets are additionally governed by a chain of
   ministry circulaires (977/1989, 638/1995, 02/1998, found only in the
   Saida DSP compilation, not confirmed at JORADP) requiring a filed
   association contract reviewed by the Section Ordinale, and, per the
   two model contracts included there, doctors in a group cabinet may or
   may not pool fees ("sans" / "avec mise en commun des honoraires").
   **Inferred:** if Dinar ever supports more than one doctor on the same
   installation, each doctor's identity, Ordre number and (per the
   contract) fee accounting are likely to need to stay separable, not
   merged by default.
10. **Sourced.** A practitioner exercising "a titre liberal" must carry
    professional civil-liability insurance. Loi 18-11, Art. 296: such
    structures "sont tenus de souscrire une assurance couvrant leur
    responsabilite civile et professionnelle." Not a software requirement
    as such; possibly a field worth tracking if the cabinet-setup screen
    ever records compliance items.
11. **Sourced, definition; inferred application.** "Dispositif medical" is
    defined by Loi 18-11, Art. 212, to include "les accessoires et
    logiciels intervenant dans son fonctionnement, destine a etre utilise
    chez l'homme a des fins medicales," and Art. 230 requires such devices
    to be registered or homologated (by the Agence Nationale des Produits
    Pharmaceutiques, per later articles and secondary reporting on ANPP
    Note 40/2025) before industrial use; Art. 232 restricts doctors to
    prescribing and using registered medicines and homologated devices.
    On a plain reading, a patient-file, queue and appointment-book module
    that computes nothing medical is not itself a "dispositif medical."
    **Inferred, and one of the sharper build constraints found:** the
    moment the clinic module adds anything that looks like a medical
    determination (a dose calculator, a diagnostic suggestion, an
    interaction alert), it risks crossing into this definition and into
    ANPP homologation exposure. No text found rules on this either way
    for pure practice-management software, and no confirmation was found
    that ANPP has ever asserted jurisdiction over one; treat this as an
    open question to raise before adding such a feature, not as a settled
    "we are covered."
12. **Sourced, one of the biggest long-range items.** Loi 18-11 creates a
    "systeme national d'information sanitaire" (Art. 319), which "integre
    toutes les donnees sanitaires et assure l'interoperabilite avec les
    systemes d'information d'autres secteurs d'activite" (Art. 320), and
    Art. 321 states without qualification: "Les structures et les
    etablissements de sante, publics et prives, sont dans l'obligation
    d'integrer le systeme national d'information sanitaire." Art. 322-323
    put security, confidentiality and data integrity on the structure and
    its users. No decree fixing "les modalites d'application... notamment
    le fonctionnement et les conditions d'acces au systeme" (promised by
    Art. 323 to come "par voie reglementaire") was found, so there is no
    known technical interface to build against today, and no evidence
    this is enforced against solo private cabinets in practice. Still,
    it means an offline-forever design is not guaranteed to stay
    compliant by law, independent of whatever cloud-sync module Dinar
    builds for its own product reasons.
13. **Sourced, secondary only.** Prescribing a stupefiant or substance
    psychotrope requires a special "ordonnance a trois souches": a
    numbered, three-copy, colour-coded (pink/yellow/white) prescription
    pad, described in press coverage of loi 04-18 (as later amended) and
    its implementing decree on psychotropes. The primary text was not
    reached to confirm the article number. **This directly limits the
    software:** a plain single-sheet printed ordonnance, however well
    formatted, does not satisfy this specific numbered/tri-copy
    requirement for controlled substances; that class of prescription
    needs either the official pre-printed stock or a route the software
    does not currently have a basis to claim covers it.
14. **Not confirmed as mandatory; do not build DCI-only.** One search
    result describes Algerian prescription content as needing "le nom
    commercial ou DCI" (commercial name **or** INN), which reads as
    optional, not "DCI-only" the way French law has required since 2015.
    No Algerian arrete requiring DCI-only prescribing was found.
    **Inferred:** the software should accept and print either a brand
    name or an INN, and should not force INN-only entry on the strength
    of the French rule, which does not appear to apply here.
15. **Sourced, general only.** A 2026 law on trust services, Loi
    n 26-02 du 17 fevrier 2026, establishes that "la signature
    electronique ou le cachet electronique ne peut etre prive de son
    efficacite juridique... au seul motif qu'il se presente sous une
    forme electronique," under the supervision of an Autorite Nationale
    de Certification Electronique that publishes a "liste de confiance"
    of authorized providers. This was reached only through secondary
    reporting, and nothing found ties it specifically to prescriptions or
    medical certificates. **Inferred, combining points 6 and 15:** a
    software-generated ordonnance or certificat is a legal document, but
    what makes it a validly signed one is still the doctor's own
    signature under Art. 56; an image of a signature simply embedded in a
    PDF is unlikely to be the kind of "cachet electronique" the 2026 law
    recognizes, since that requires an accredited provider. The safe
    default for the first version stays print-then-hand-sign, not an
    embedded signature image presented as equivalent.

## Evidence by topic

### 1. Loi 18-11 du 2 juillet 2018 relative a la sante

Read from the Journal Officiel N 46 text, hosted at snapo-dz.net (see
Sources). Key articles, quoted where the extraction was clean:

- Art. 24 (secret medical and privacy): "le secret medical couvre
  l'ensemble des informations parvenues a la connaissance des
  professionnels de sante... [il] peut etre leve par la juridiction
  competente," lifting also possible for minors/incapables at a spouse's
  or parent's request. Confirmed independently by a legal-doctrine.com
  summary citing the same article number.
- Art. 26: "Tout patient doit disposer d'un dossier medical."
- Art. 178: among "professionnels de sante" duties, "de tenir a jour le
  dossier medical du patient."
- Art. 168-171 (Titre on professionnels de sante): personal, named
  practice ("exerce sa profession sous son identite legale"), secret
  medical et/ou professionnel (Art. 169), and remplacement rules
  (Art. 171, referencing conditions set at Art. 166, not itself pulled).
- Art. 291-296 (structures et etablissements de sante): the "dossier
  medical unique informatise" and archive-conservation duty (Art. 292),
  a duty to report births and deaths to the commune (Art. 294), order and
  discipline in the premises (Art. 295), and mandatory professional
  liability insurance for those "exercant a titre liberal" (Art. 296).
- Art. 308-315 (structures et etablissements prives de sante): the list
  of private-practice forms (Art. 308), state control and evaluation
  (Art. 310), a cahier des charges for a service-public mission
  (Art. 311), public-tarification-and-information rules (Art. 313), an
  exploitation authorization referenced at Art. 314, and temporary or
  permanent closure by the wali (Art. 315).
- Art. 212-232 (produits pharmaceutiques et dispositifs medicaux): the
  "dispositif medical" definition including software (Art. 212),
  registration/homologation duties (Art. 230), and the restriction to
  prescribing only registered/homologated products (Art. 232).
- Art. 319-323 (systeme national d'information sanitaire): creation
  (Art. 319), interoperability (Art. 320), mandatory integration by
  "les structures et les etablissements de sante, publics et prives"
  (Art. 321), and security/confidentiality duties (Art. 322-323), with
  the technical rules deferred "par voie reglementaire" (not found).

No article was found in this law fixing a retention period, in years, for
a dossier medical, nor one spelling out a patient's right to a copy of
their file, nor one addressing what happens to the dossier when a
cabinet closes (see "What stays unsourced").

### 2. Code de deontologie medicale (decret executif n 92-276 du 6 juillet 1992) and ministry practice rules

The primary decret text is a scanned Journal Officiel PDF with no text
layer (tried at sia-enna.dz and two other mirrors; none had a machine
readable layer). Structure and a few quoted lines come from secondary
sources that summarize or partially quote it:

- Definition, Art. 1 (quoted by an over-blog.com legal-education post):
  "La deontologie medicale est l'ensemble des principes, des regles et
  des usages que tout medecin, chirurgien-dentiste et pharmacien doit
  observer ou dont il s'inspire dans l'exercice de sa profession."
- Structure, per the same source: devoirs generaux (art. 6 a 35 or 41,
  sources differ slightly on the boundary), secret professionnel (art. 36
  a 41), devoirs envers le malade (art. 42 a 58, one source says art. 43
  a 44 or art. 44 a 51 for a slightly different grouping - the two
  academic summaries found do not fully agree on where each section
  starts and ends), confraternite, rapports entre medecins, and a
  section on "regles particulieres a certains modes d'exercice" with
  "A. Exercice prive (8 articles) Art 77 a 84" as the private-practice
  block.
- Art. 56 (mandatory content of an ordonnance, certificat, attestation or
  document): legible, identifies the signer, dated and signed by the
  doctor. Reported consistently across several search results but never
  reached in a directly quotable primary or secondary document during
  this pass; treat as probable, not certain.
- No article number could be attached to honoraires ("tact et mesure"),
  the note d'honoraires, cabinet secondaire, or fee-display rules;
  several secondary sources describe French equivalents (Art R.4127-53)
  but nothing found ports that to a specific Algerian article number.
- The Direction de la Sante de Saida compilation ("Reglementation de
  l'exercice a titre prive des Medecins, Chirurgiens-Dentistes,
  Generalistes et Specialistes," a scanned-but-machine-readable ministry
  document) adds detail the decret itself does not, at the level of
  instructions and circulaires rather than a loi or decret:
  - Installation: "La delivrance des autorisations d'installation demeure
    du ressort du wali territorialement competent," and "la decision
    portant autorisation d'ouverture de cabinet ou d'officine prives est
    delivree par le wali sur presentation du certificat de cessation de
    paiement... et apres verification et enregistrement des diplomes sur
    le registre ad-hoc" (Instruction n 00112/MSP/SG du 02 Mars 1987).
  - Documents required for a cabinet's opening file, per the same
    compilation: acte de location or acte de propriete (or an equivalent
    from the CNEP, the OPGI, or a wali's arrete), inscription au tableau
    de l'Ordre, an attestation of non-affiliation to CNAS for a
    practitioner without prior activity, a decision de demission or
    certificat de cessation de paiement for those coming from public
    practice, certificat de nationalite, casier judiciaire, two
    certificats medicaux, and two photos.
  - Remplacement: the titulaire "doit etre inscrit au tableau," sends a
    written request naming the reason and duration to the Direction de
    la Sante de Wilaya, the request needs the Section Ordinale's prior
    opinion, which "delivre une licence de remplacement," and a refusal
    "doit etre motive."
  - Group cabinets: a chain of circulaires (n 977/DSS/SDCPI of 10 July
    1989, n 638/MSP/DNOSS/SDEASPS of 15 August 1995, n 02/MSP/DSS/SDCC of
    1 March 1998) plus two model association contracts, one with and one
    without pooled honoraires, both requiring each associate's Ordre
    registration number on the contract itself.

### 3. Prescription rules

- DCI: not found as a mandatory, DCI-only requirement in Algeria; one
  source describes prescriptions as needing "le nom commercial ou DCI,"
  which reads as either-or, not DCI-only.
- Psychotropes and stupefiants: loi 04-18 du 25 decembre 2004 (as
  modified, most recently reported as amended by a 2025 law) and a
  reported 2021 decree require a three-copy, numbered, colour-coded
  "ordonnance a trois souches" for these prescriptions; the exact article
  establishing the format was not reached in a primary text during this
  pass (sherloc.unodc.org hosts the law's text but the specific
  ordonnancier article was not located within the time available).
- Validity period of an ordinary ordonnance: not found.
- Electronic or software-printed ordonnance legality and signature: Loi
  n 26-02 du 17 fevrier 2026 (services de confiance, identification
  electronique) gives electronic signatures and cachets legal effect
  under an accredited-provider framework overseen by an Autorite
  Nationale de Certification Electronique; nothing found ties this
  specifically to medical prescriptions. Combined with decret 92-276's
  Art. 56 (signature required, see above), the safer reading for a first
  version is: the software may typeset and print the ordonnance, but the
  doctor's own signature (handwritten on the printout, or an accredited
  electronic signature, not a plain embedded image) is what the rule
  actually asks for.

### 4. Certificats

- A description of an arret-de-travail certificat's required content
  (name and address of the prescriber, date, signature and cachet, the
  employee's identity, and the duration of the arret) surfaced only
  through a search engine's own synthesis of several non-authoritative
  pages (demarchesdz.com, almawarid.app and similar); no CNAS regulatory
  text or JORADP arrete was reached to confirm it. Treat as a plausible
  checklist, not a sourced requirement.
- Certificat de bonne sante, certificat de deces, certificat for sport:
  who may issue each, their mandatory content, and any official CNAS
  form: not found.

### 5. Software homologation, e-sante, and the Chifa card

- "Dispositif medical," as defined by Loi 18-11 Art. 212, expressly
  includes "logiciels intervenant dans son fonctionnement." Registration
  or homologation (Art. 230) runs through the Agence Nationale des
  Produits Pharmaceutiques (ANPP); a 2025 ANPP note (reported as Note
  40/MIP/ANPP/DG/NOTE/2025) required filing homologation dossiers for
  medical devices already on the market without one. Nothing found
  applies this specifically to practice-management or e-prescription
  software, as opposed to physical medical devices and diagnostic
  equipment; nothing found exempts it either. This is a genuine open
  question, not a settled "no."
- A national e-sante strategy, an "Agence Numerique de Sante" (ANDS), or
  a national patient-record programme that a private cabinet's software
  would need to connect to: not found as a named, current programme,
  beyond the "systeme national d'information sanitaire" created in
  principle by Loi 18-11 Art. 319-323 (see point 12 above), whose
  technical rules were never located.
- Carte Chifa (CNAS): confirmed as the insured patient's smart card,
  used to verify rights and to process "feuilles de soins" mainly
  through the pharmacy network (over 13,000 conventioned pharmacies).
  Nothing found describes a private doctor's cabinet software
  interfacing directly with Chifa or with a CNAS tiers-payant system for
  consultations; this looks, from what was found, like a
  pharmacy-and-CNAS matter more than a doctor's-cabinet one, but that
  absence of evidence is not the same as a confirmed "no interface
  exists or is required."

### 6. Opening a cabinet: registration numbers on printed documents

- An Ordre inscription number: required to install at all (Art. 308
  context and the Saida compilation's model contracts, which each print
  a blank for "Numero d'inscription au tableau"). No article was found
  making its appearance on every printed ordonnance or certificat itself
  mandatory, but it is the number that proves the right to practice, and
  the model contracts treat it as a doctor's basic identifying reference
  alongside the address.
- Agrement / autorisation d'ouverture: delivered by the wali (Loi 18-11,
  Art. 314 area, and Instruction n 00112/MSP/SG of 1987, both above).
- NIF, NIS, or a registre-de-commerce equivalent: not found in any
  health-sector text reached during this pass. A liberal medical
  profession is not a commercant under Algerian law generally, so a
  registre du commerce itself is not expected to apply, but no text
  confirming a NIF/NIS obligation (or its absence) for a doctor's cabinet
  specifically was located; that likely sits closer to the fiscal-rules
  research this page is told not to duplicate.

## What stays unsourced ("not found")

- The retention period, in years, for a dossier medical (a 10-year figure
  circulated in one search engine's own summary as "common practice,"
  with no article, decree or arrete behind it; treated here as not
  found, not as a fact).
- A patient's explicit right of access to, or a copy of, their own
  dossier medical.
- What happens to a cabinet's dossier medical when the cabinet closes,
  the doctor retires, or the doctor dies.
- The exact article of decret 92-276 covering honoraires, "tact et
  mesure," the note d'honoraires, and fee display.
- The exact article of decret 92-276 (or a companion arrete) covering
  cabinet secondaire.
- The precise article of loi 04-18 (or its implementing decree)
  establishing the ordonnance a trois souches format, beyond secondary
  press description.
- Any arrete requiring DCI-only prescribing in Algeria.
- Whether ANPP homologation has ever been asserted, in practice, against
  a practice-management or e-prescription software product as opposed to
  a physical medical device.
- Any named, currently active e-sante or national-patient-record
  programme beyond the "systeme national d'information sanitaire" that
  Loi 18-11 creates in principle; and the decree that Art. 323 promises
  to fix its "modalites d'application."
- Whether a private doctor's cabinet (as opposed to a pharmacy) is
  expected to interface software with the Chifa/CNAS system for
  consultations.
- The official CNAS arret-de-travail form's legally mandated content,
  and the rules for certificat de bonne sante, certificat de deces, and
  sport certificates: who may issue each and what they must contain.
- Whether a NIF or NIS must appear on a doctor's printed documents.

## Sources

- Loi n 18-11 du 18 Chaoual 1439 correspondant au 2 juillet 2018 relative
  a la sante, Journal Officiel de la Republique Algerienne N 46 (text
  edition): https://www.snapo-dz.net/site/wp-content/uploads/2020/06/loi-sante-2018-11.pdf
- Decret executif n 92-276 du 6 juillet 1992 portant code de deontologie
  medicale, scanned Journal Officiel (no extractable text layer):
  https://www.sia-enna.dz/PDF/regulation/orders/fr/JO_52-1992-FR.pdf
- "Etude comparative du code de deontologie medicale algerien et
  francais," Faculte de Medecine d'Oran, Service de Medecine Legale:
  https://medecinelegalechuoran.over-blog.com/etude-comparative-du-code-de-d%C3%A9ontologie-m%C3%A9dicale-alg%C3%A9rien-et-fran%C3%A7ais
- "Deontologie medicale," Pr Ali Taleb, HMRUC, Faculte de Medecine
  Constantine 3 (course PDF):
  https://facmed.univ-constantine3.dz/wp-content/uploads/2024/04/D%C3%A9ontologie-m%C3%A9dicale.pdf
- "Reglementation de l'exercice a titre prive des Medecins,
  Chirurgiens-Dentistes, Generalistes et Specialistes," compilation of
  ministry instructions and circulaires (1985-1999), Direction de la
  Sante de Saida:
  https://dsp20saida.weebly.com/uploads/3/7/6/6/37664781/_rglementation_de_lexercice_a_titre_priv_des_mdecins_chirurgiens-dentistes_gnralistes_et_spcialistes.pdf
- "Le nouveau texte de la loi relative a la sante dejuge le decret
  executif 92-276 portant code de deontologie medicale," Dr Mahmoud
  Boudarene, republished from Liberte, 29 November 2016:
  http://docteurboudarene.unblog.fr/2016/11/29/le-nouveau-texte-de-la-loi-relative-a-la-sante-dejuge-le-decret-executif-92-276-portant-code-de-deontologie-medicale/
- Legal-doctrine.com summary of Loi n 18-11 competences et droits
  (secondary, cites Art. 24-25 on secret medical): https://legal-doctrine.com/en/edition/Loi-n-18-11-du-2-juillet-2018-relative-%C3%A0-la-sant%C3%A9-comp%C3%A9tences-et-droits
- Legal-doctrine.com, "Dispositifs medicaux en Algerie: obligations
  d'homologation et levee des reserves" (secondary, on ANPP and Note
  40/2025): https://legal-doctrine.com/en/edition/dispositifs-medicaux-en-algerie-obligations-dhomologation-et-levee-des-reserves-e3e2b96bb6668ff000473b242f570796
- TSA-Algerie, "Lutte contre le trafic de psychotropes: l'ordonnance a
  trois souches divise" (secondary, on loi 04-18's prescription pad):
  https://www.tsa-algerie.dz/lutte-contre-le-trafic-de-psychotropes-lordonnance-a-trois-souches-divise/amp/
- Loi No. 04-18 du 13 Dhou El Kaada 1425 correspondant au 25 decembre
  2004 (hosted by UNODC SHERLOC; the specific ordonnancier article was
  not located within it during this pass):
  https://sherloc.unodc.org/cld/uploads/res/document/dza/loi-04-18_html/algeria-loi04-18.pdf
- Reporting on Loi n 26-02 du 17 fevrier 2026 relative aux services de
  confiance et a l'identification electronique (secondary, JORADP text
  not fetched): https://www.mpt.gov.dz/wp-content/uploads/2026/02/Loi-n%C2%B0-26-02.Services-de-Confiance.Identification-electronique.FR_.pdf
  and https://lavoiedalgerie.dz/numerisation-une-nouvelle-loi-pour-encadrer-la-signature-et-le-cachet-electroniques/2026/26/00/
- Carte Chifa, Wikipedia (French) and CNAS's own pages (secondary,
  general description only): https://fr.wikipedia.org/wiki/Carte_Chifa
  and https://cnas.dz/fr/presentation-of-the-chifa-card/
- Almawarid.app, "Loi 18-07 et cabinet de sante: proteger les donnees
  patient en Algerie" (secondary, flagged here only because it names
  loi 18-11 alongside loi 18-07; the data-protection substance belongs
  to the other agent's research):
  https://almawarid.app/blog/loi-18-07-cabinet-sante-protection-donnees-patient-algerie/
- Search-engine synthesis (not independently verified against a primary
  or named secondary text) used only where explicitly flagged above as
  "secondary" or "not found," on: decret 92-276 Art. 56's exact wording,
  and CNAS arret-de-travail certificat content.
