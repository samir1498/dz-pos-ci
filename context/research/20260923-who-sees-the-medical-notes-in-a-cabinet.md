---
title: 'Who sees the medical notes: the secretary split in a cabinet'
date: 2026-09-23
status: 'draft'
tldr: 'Recommendation is option (b). The secretary or receptionist sees
  identity, phone and the appointment book; free-text clinical notes and
  history stay doctor-only, with the doctor able to widen a given record if
  needed. Algerian software already ships this split (WinMed, sold by BIG
  Informatique, logs the secretary in under a separate password that blocks
  medical data), French law binds a secretary to secret medical but the
  CNIL states plainly that a secretary reaches appointment data, not the
  full dossier medical, and Doctolib caps a secretary account below full
  file access by default. Algeria''s Loi 18-07 treats health data as
  sensitive, which argues for the same least-privilege design rather than
  one shared view for every signed-in user.'
---

# Who sees the medical notes: the secretary split in a cabinet

The answer is (b), not (a). A secretary or receptionist account should see
patient identity, phone number and the appointment book, and should not see
the free-text clinical notes and history. Those stay doctor-only, with room
for the doctor to open a specific note if the practice needs it. No source
found argues for giving every signed-in user, doctor and secretary alike,
the same full view of the medical file; the CNIL states the split in so
many words, the software actually sold in both Algeria and France ships a
narrower secretary role, and the Algerian data protection law treats health
data as sensitive, which pushes toward restricting it rather than sharing
it by default.

## 1. Algerian law

### Loi n° 18-11 du 2 juillet 2018 relative à la santé

FAOLEX and UNEP's legal-database listings confirm the law exists in this
form (9 titles, 450 articles, in force since 2 July 2018) and that it deals
with prevention, protection and the right to health with respect for
dignity and privacy; this session read those listing pages' summaries but
could not open FAOLEX's own document view (it returned HTTP 403), so what
follows beyond structure is not confirmed by direct reading. A search
summary reports that the law defines secret médical as covering everything
a health professional learns about a patient, refers violations to article
301 of the penal code, and allows information to travel among the members
of a care team treating the same patient. Those three specifics come from
a secondary aggregation, not from a primary copy of 18-11 opened this
session, so treat the article numbers as reported rather than verified. Not
found either way: any article of 18-11 that names secretaries or lay
auxiliaries, or that says whether they count as part of the "team" the
sharing exception speaks of.

Sources: FAOLEX
(https://www.fao.org/faolex/results/details/en/c/LEX-FAOC181216) and UNEP
LEAP (https://leap.unep.org/en/countries/dz/national-legislation/loi-ndeg-18-11-du-18-chaoual-1439-correspondant-au-2-juillet-2018)
confirm the law's existence and structure only; legal-doctrine.com's summary
(https://legal-doctrine.com/edition/Loi-n-18-11-du-2-juillet-2018-relative-%C3%A0-la-sant%C3%A9-comp%C3%A9tences-et-droits/)
was read directly and covers the law's general principles without naming
secretaries or the secret-médical article numbers.

### Code de déontologie médicale, décret exécutif n° 92-276 du 6 juillet 1992

The decree exists, applies to physicians, dental surgeons and pharmacists
in Algeria, and is enforced through the Conseils de l'Ordre. This session
tried to open three candidate PDFs to quote its article on professional
secrecy directly: services.mesrs.dz's document (filed under an
"EthiqueDeontologie/DocumentsCharte" path, so its exact identity is
unconfirmed), a scanned copy at snapo-dz.net, and a copy at
univ.ency-education.com. All three returned raw, undecoded PDF stream data
to this session's fetch tool rather than readable text, on both the first
and a retried attempt, so none of them could be read or quoted. A search
summary and several Algerian medical-faculty course documents describe the
same duty as the French equivalent code, R.4127-72: a physician must see
that the people who assist him in his practice are instructed in their
professional-secrecy obligations and comply with them. That is a plausible
reading, given how closely the Algerian code otherwise tracks the French
model, but it is not found here as a direct quote from décret 92-276 with
an article number, only as a description in secondary course material. Not
found: a primary-source quote of the article in décret 92-276 naming
auxiliaries or secretaries.

Sources: Wikipedia's page on the Algerian Conseils de l'Ordre
(https://fr.wikipedia.org/wiki/Conseils_de_l'Ordre_des_professions_m%C3%A9dicales_(Alg%C3%A9rie));
Faculté de Médecine Constantine's déontologie course
(https://facmed.univ-constantine3.dz/wp-content/uploads/2024/04/D%C3%A9ontologie-m%C3%A9dicale.pdf);
Faculté de Médecine Oran's secret médical course
(https://facmed-univ-oran.dz/ressources/fichiers_produits/fichier_produit_3025.pdf);
docteurboudarene's post noting décret 92-276 predates the 2018 health law
(http://docteurboudarene.unblog.fr/2016/11/29/le-nouveau-texte-de-la-loi-relative-a-la-sante-dejuge-le-decret-executif-92-276-portant-code-de-deontologie-medicale/).
None of these were confirmed to contain the decree's own article text.

### Loi n° 18-07 du 10 juin 2018 sur la protection des données personnelles

The law is Algeria's rough equivalent of the GDPR. It creates the ANPDP
(Autorité Nationale de Protection des Données à Caractère Personnel) as an
independent authority, and it names health data, biometric data and
criminal-record data as sensitive categories that need prior ANPDP
authorization before they can be processed at all, on top of the ordinary
declaration duty for personal data. Compliance guides built on the law
describe an access-control obligation, limiting access to authorized
persons only, as one of the required security measures, alongside
encryption and an access log. That access-limitation point is drawn from a
compliance guide's summary rather than a quoted article of the law itself,
so it is evidence of how the law is being explained to businesses rather
than a verified article citation.

Sources: Conseil National des Assurances' notice on the law's entry into
force
(https://cna.dz/entree-en-vigueur-de-la-loi-sur-la-protection-des-donnees-a-caractere-personnel-en-algerie/14/08/2023/);
HALKORB's compliance guide, which names the access-limitation duty
(https://halkorb.com/blog/protection-donnees-pdp-18-07-algerie.html); GAAN's
professional-association page on the law
(https://members.gaan.dz/articles-divers/loi-n18-07--protection-des-donnees-personnelles-page-15279).

## 2. Algerian cabinet software

### WinMed (BIG Informatique, sold across Algeria and Africa)

Confirmed directly from the vendor's own help page: WinMed ships three
default passwords, each opening a different scope. "go" is the
administrator physician's password, the only one that can change the
others. "sec" is the secretary's password, and it explicitly blocks access
to sensitive medical data and to financial data. "rem" is for a locum
physician and blocks statistics and financial data instead. This is the
clearest primary-source match for option (b) found in this research: the
software vendor itself treats "secretary" and "sensitive medical data" as
mutually exclusive by default. BIG Informatique's own product page names
WinMed as its cabinet-médical product directly ("Le Logiciel WinMed de BIG
Informatique..."), confirming winmed.africa and BIG Informatique are the
same vendor and not two different products that happen to share a name.

Sources: https://www.winmed.africa/aide_med.html (fetched and read
directly; the password table is under "LANCEMENT DE WINMED - MOT DE
PASSE"); https://biginformatique.com/produits/gestion-de-cabinet-m%C3%A9dical
(fetched and read directly, names WinMed as BIG Informatique's product).

### CabiSanté

Confirmed directly from the vendor's homepage: CabiSanté has an
"assistant(e)" profile. That profile can use the waiting-room screen, but
"l'accès aux informations concernant les payements est limité pour les
profils assistant(e) et remplaçant(e)" (access to payment information is
limited for the assistant and locum profiles). The page does not say, one
way or the other, whether the assistant(e) profile can open a patient's
free-text medical notes; the restriction it documents is on payment and
accounting data, not clinical content. Not found: an explicit statement
that CabiSanté hides clinical notes from the assistant(e) profile.

Source: https://cabisante.com/ (fetched and read directly).

### ClinixaPro

ClinixaPro markets itself as a large clinic ERP (500+ features, 20+
modules including a DPE, CNAS billing, PACS/RIS, lab and operating-room
modules) with CNAS and RGPD-style traceability tooling, but the material
found does not describe a secretary role or what it can or cannot see in
the dossier médical. Not found.

Source: https://www.clinixapro.com/solution/.

### GCDOC

Marketed as the top cabinet-management software in Annaba and nationally,
aimed at general practitioners, specialists, dentists, psychologists,
nutritionists and speech therapists. No role or permission detail for a
secretary account was found in what this search could reach (its own site
and its Facebook page).

Source: https://gcdoc.dzdoc.com/.

### Other Algerian tools noted but not detailed

Medica Pro advertises a "mode secrétaire" that runs on the cabinet's local
network once the doctor turns it on, but no detail on what that mode
excludes was found
(surfaced via https://medica-pro.com/, not independently confirmed).
EasyClinic, NourDoc, SmartAvicenne and BIG Informatique's own product page
were located but yielded no role-permission detail: not found for those.

## 3. French comparison

### Code de la santé publique L1110-4, and R.4127-72

L1110-4 states that secret professionnel covers all information concerning
a patient that comes to the knowledge of a health professional, of any
staff member of a health establishment, or of any other person in contact
with that establishment by virtue of their activity there; it binds every
health professional and every professional working in the health system.
R.4127-72 puts the obligation on the physician directly: he must see that
the people who assist him in his practice are instructed in their
professional-secrecy obligations and comply with them, and that nothing in
his surroundings breaches the secrecy attached to his professional
correspondence. Read together, French law does not forbid a secretary from
touching clinical material (mail, clinical sheets, medical records, lab
reports); it makes her criminally responsible if she discloses what she
sees, and makes the doctor responsible for training and supervising her.
Egora's article draws the concrete conclusion doctors ask about: a
secretary who reads the dossier médical as part of her job is not breaking
the law by having that access, but she is bound by the same secrecy the
doctor is, and can be prosecuted for breaching it.

Sources: Légifrance's article text
(https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000043895798);
Egora, "Ma secrétaire est-elle tenue au secret médical ?"
(https://www.egora.fr/gestion-du-cabinet/juridique/ma-secretaire-est-elle-tenue-au-secret-medical),
which quotes both L1110-4 and R.4127-72 directly.

### CNIL guidance

The CNIL's page on dossier médical access (05 mai 2009) covers the
patient's own right to obtain their file; it does not set out a rule for a
secretary's access. But a second CNIL page, "RGPD et professionnels de
santé libéraux : ce que vous devez savoir" (1 June 2018), states the
secretary rule directly, and this is the strongest single source found in
this whole research. Quoting it: "Vous devez limiter l'accès aux données de
santé de vos patients : seules certaines personnes sont autorisées, au
regard de leurs missions, à accéder à celles-ci (ex : équipe de soins d'un
établissement de santé intervenant dans la prise en charge sanitaire du
patient, secrétaire médicale, organismes d'assurance maladie...). Ces
personnes n'accèdent qu'aux données nécessaires à l'exercice de leur
mission (ex : le secrétaire médical accède aux données administratives
permettant de gérer les prises de rendez-vous, mais n'accède pas à la
totalité du dossier médical)." That is the CNIL naming the exact split this
research was asked to decide between, and picking (b): a secretary reaches
appointment and administrative data, not the whole medical file. A separate
joint CNIL/CNOM "Guide pratique sur la protection des données personnelles"
(2018) also exists, but this session could not extract readable body text
from that PDF (only its metadata came through), so nothing from it is
cited here.

Sources: https://www.cnil.fr/fr/lacces-au-dossier-medical (read directly,
covers only the patient's own access, not the secretary question);
https://www.cnil.fr/fr/rgpd-et-professionnels-de-sante-liberaux-ce-que-vous-devez-savoir
(read directly, contains the quoted secretary rule above); the CNIL/CNOM
guide at
https://www.cnil.fr/sites/default/files/atoms/files/guide-cnom-cnil.pdf
(located, not readable this session).

### Doctolib

Confirmed directly from Doctolib's own help center: user accounts can be
given one of several roles, including "Secrétaire" and "Assistant(e)".
Both are described the same way for medical-file purposes: they get
"accès aux dossiers médicaux jusqu'au niveau 3 (si accordé par les
praticiens)" and a "niveau d'accès aux données médicales limité, selon les
accès donnés par les praticiens", and neither role can edit consultations
or prescriptions, which stays reserved to the file's owning practitioner.
So the practitioner decides how far a secretary's view of the medical file
goes, capped below full access by default, and clinical authorship is
doctor-only regardless of that setting.

Source: https://doctolib.zendesk.com/hc/fr/articles/40834657017108-Comprendre-les-diff%C3%A9rents-r%C3%B4les-pouvant-%C3%AAtre-attribu%C3%A9s-%C3%A0-un-utilisateur-de-mon-%C3%A9tablissement
(fetched and read directly, including the role table).

### Weda, HelloDoc, Crossway, MédiStory, AxiSanté

All five are described, in vendor and comparison material, as offering
per-profile access control that names "secrétaire" as one of the profiles
(alongside médecin, remplaçant, infirmier, coordinateur). Weda's own pages
describe "une gestion fine des droits d'accès par profil utilisateur", set
from Paramètres > Sécurité > Gestion des Droits d'accès. HelloDoc's vendor
page says access to medical files is restricted to authorized people, with
every access logged, but does not spell out a default cap for a secretary
role specifically. Cegedim's Crossway help center has an article titled
"Accéder au dossier médical partagé en tant que secrétaire ou médecin non
autorisé", which by its title alone confirms the software distinguishes a
secretary's route into the shared medical file from an unauthorized
doctor's, though this research did not open that article's body. MédiStory
and AxiSanté were found only in comparison-site summaries stating the same
per-profile pattern, with no primary confirmation. Net finding: fine-grained,
configurable, per-profile access control naming a secretary role is the
norm across this whole French market segment; a single shared view for
every signed-in user, is not.

Sources: https://weda.fr/logiciel-secretaire-medicale and
https://assistance-weda.zendesk.com/hc/fr/articles/1500005043661-Comment-param%C3%A9trer-les-acc%C3%A8s-des-utilisateurs-sur-l-ensemble-des-dossiers-des-patients;
https://www.cgm.com/fra_fr/tous-nos-produits/logiciels-metiers/decouvrir/hellodoc/dossiers-patients.html;
https://www.cegedim-logiciels.com/dyn/espace_client/Aide_en_ligne/crossway/24.00/webhelp/content/ch14s04.html
(title read, body not fetched); comparison coverage at
https://healthcare.orisha.com/blog/professionnels-de-sante/medecin/choisir-logiciel-dossier-patient/.

## Sources

- Loi n° 18-11 du 2 juillet 2018 relative à la santé: FAOLEX
  (https://www.fao.org/faolex/results/details/en/c/LEX-FAOC181216); UNEP
  LEAP mirror
  (https://leap.unep.org/en/countries/dz/national-legislation/loi-ndeg-18-11-du-18-chaoual-1439-correspondant-au-2-juillet-2018);
  legal-doctrine.com summary
  (https://legal-doctrine.com/edition/Loi-n-18-11-du-2-juillet-2018-relative-%C3%A0-la-sant%C3%A9-comp%C3%A9tences-et-droits/).
- Décret exécutif n° 92-276 du 6 juillet 1992 (code de déontologie
  médicale): Wikipédia
  (https://fr.wikipedia.org/wiki/Conseils_de_l'Ordre_des_professions_m%C3%A9dicales_(Alg%C3%A9rie));
  Faculté de Médecine Constantine
  (https://facmed.univ-constantine3.dz/wp-content/uploads/2024/04/D%C3%A9ontologie-m%C3%A9dicale.pdf);
  Faculté de Médecine Oran
  (https://facmed-univ-oran.dz/ressources/fichiers_produits/fichier_produit_3025.pdf).
- Loi n° 18-07 du 10 juin 2018 (protection des données personnelles, ANPDP):
  CNA
  (https://cna.dz/entree-en-vigueur-de-la-loi-sur-la-protection-des-donnees-a-caractere-personnel-en-algerie/14/08/2023/);
  HALKORB (https://halkorb.com/blog/protection-donnees-pdp-18-07-algerie.html).
- WinMed (BIG Informatique): https://www.winmed.africa/aide_med.html and
  https://biginformatique.com/produits/gestion-de-cabinet-m%C3%A9dical.
- CabiSanté: https://cabisante.com/.
- ClinixaPro: https://www.clinixapro.com/solution/.
- GCDOC: https://gcdoc.dzdoc.com/.
- Code de la santé publique L1110-4: Légifrance
  (https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000043895798).
- R.4127-72 and the secretary question: Egora
  (https://www.egora.fr/gestion-du-cabinet/juridique/ma-secretaire-est-elle-tenue-au-secret-medical).
- CNIL, l'accès au dossier médical: https://www.cnil.fr/fr/lacces-au-dossier-medical.
- CNIL, RGPD et professionnels de santé libéraux (states the secretary
  split directly):
  https://www.cnil.fr/fr/rgpd-et-professionnels-de-sante-liberaux-ce-que-vous-devez-savoir.
- Doctolib roles help article:
  https://doctolib.zendesk.com/hc/fr/articles/40834657017108-Comprendre-les-diff%C3%A9rents-r%C3%B4les-pouvant-%C3%AAtre-attribu%C3%A9s-%C3%A0-un-utilisateur-de-mon-%C3%A9tablissement.
- Weda: https://weda.fr/logiciel-secretaire-medicale.
- HelloDoc: https://www.cgm.com/fra_fr/tous-nos-produits/logiciels-metiers/decouvrir/hellodoc/dossiers-patients.html.
- Crossway (Cegedim): https://www.cegedim-logiciels.com/dyn/espace_client/Aide_en_ligne/crossway/24.00/webhelp/content/ch14s04.html.
