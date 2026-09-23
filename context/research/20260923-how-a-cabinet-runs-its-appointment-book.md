---
title: 'How a cabinet runs its appointment book'
date: 2026-09-23
status: 'draft'
tldr: 'Across Algerian and French cabinet software, the appointment book is
  built from the same small set of parts: weekly working hours you can close
  for a day, a block for an absence that offers to move every appointment it
  hits, a visit type that carries its own duration, a search for the next
  free slot, a no-show mark, and a printed day list that has to include
  walk-ins because every Algerian product pairs the agenda with a waiting
  room. Fill-rate caps, waiting lists for a freed slot, per-patient no-show
  counts and SMS reminders are French-market extras built on top of that
  base, not part of it, and the first version should leave them out. Algeria
  fixes 14 legal holidays by a 1963 law last amended in 2023 to give both
  Aids three days each, the Hijri ones announced the evening before by a
  lunar-sighting commission, and Friday as the legal weekly rest day since
  1990 with Saturday added for the public sector in 2009; Ramadan hours and
  a Friday-prayer break for private cabinets, and any rule on missed-
  appointment fees, were not found.'
---
# How a cabinet runs its appointment book

## Recommendation

The smallest set of book tools that shows up, in some form, across every
product looked at below, ordered by how many products carry it and how much
of a cabinet's day it touches without it:

1. **Weekly working hours as recurring open/close ranges, with a way to
   close a whole day.** Every product needs this before anything else works;
   WinMed calls it "l'horaire des consultations" in its settings, Doctolib
   calls it "plages d'ouverture." Edge case: a slot booked inside a range
   that later gets closed (a holiday declared after bookings exist) has to
   surface, not silently vanish.
2. **An absence block for a period, offering to move every appointment it
   hits.** Doctolib's "Créer une absence" and HelloDoc's closure/replacement
   blocks both span whole days by drag-and-drop. Edge case: every moved
   appointment must still pass the same double-booking and off-grid checks
   as a fresh booking, and the patient has to be told, even without SMS, by
   aline on the printed day list or the queue screen.
3. **Visit types that carry their own duration**, so the grid can hold
   different slot sizes per visit rather than one fixed length for
   everyone. Doctolib, Weda and Doctoranytime all key duration off a
   "motif." Edge case: changing a type's duration must not resize
   appointments already booked under the old duration.
4. **A next-free-slot search** across the open days, respecting the visit
   type's duration and skipping blocked periods (Doctolib's "Trouver un
   créneau"). Edge case: the search has to skip a slot that is technically
   free but shorter than the requested visit type needs.
5. **A no-show mark on a past appointment.** Doctolib marks "absence non
   excusée"; SmartAvicenne already has this pattern for exam follow-ups
   (effectué / en retard / non effectué) that a book can reuse. Edge case:
   the mark should only be offered once the slot's time has passed, not
   while it is still upcoming.
6. **A printed day list that includes walk-ins, not just booked slots.**
   Doctolib prints "la liste des rendez-vous de la journée" with column
   choices; every Algerian product found (CabiSanté, GCDOC, SmartAvicenne)
   pairs its agenda with a "salle d'attente," which is the strongest signal
   here that a cabinet's day list is never booked-only.

**Leave out of the first version**: a fill-rate cap on the agenda (Doctolib
only), a waiting list for a slot that frees up (Doctolib only, and even
there it only ever advances a booking the patient already holds, it does
not grant a booking from nothing), per-patient no-show counts and
statistics (Maiia's honoré/annulé breakdown), SMS reminders (there is no
patient app to send them from yet), recurring or multi-step appointment
series, and any named overbooking or "urgent slot" carve-out, which was not
found as a distinct feature in any product; the walk-in queue already
covers the same need.

## Per-product evidence

### Algerian products

Every Algerian vendor found publishes marketing pages, not user
documentation; only WinMed has a real help page, and even that page barely
touches the agenda.

- **WinMed** (BIG Informatique). Its help page lists "Agenda: pour la
  gestion des rendez-vous" as one menu button among many, and its settings
  screen ("Paramètres") is described only as holding "des données
  concernant le cabinet, dont l'horaire des consultations pour les RDV."
  No detail on visit types, no-shows, or printing was found on the page
  itself. Marketing copy elsewhere claims the agenda "réduit les absences
  de 40%" with no mechanism named.
  Source: https://www.winmed.africa/aide_med.html
- **CabiSanté**. The homepage lists appointments as one field inside the
  single-window patient file ("Ordonnance, Documents, Rendez-Vous,
  payement") and separately advertises "salle d'attente" management,
  restricted to the assistant profile in networked mode. No agenda detail
  (working hours, visit types, no-shows) is published.
  Source: https://cabisante.com/
- **GCDOC**. Search-indexed marketing copy lists "la gestion des
  rendez-vous et salles d'attentes" together as one feature, alongside
  reports (ordonnances, certificats), per-user access limits between doctor
  and secretary, and daily cash tracking. The site itself could not be
  fetched directly (TLS certificate mismatch on gcdoc.dzdoc.com); this is
  from its Facebook page and directory listings.
  Source: https://www.facebook.com/GCDOC.Logiciel.de.Gestion.de.Cabinet.Medical/
- **SmartAvicenne** (SmartrPartner, on the market since 2002, network
  version since 2010). Its "Agenda Intégré" is described as managing
  "les rendez-vous avec vos patients, les visites des délégués, les
  rendez-vous personnels," with alarms that fire when one comes due. Its
  vaccine/exam follow-up feature already has a three-state mark (effectué,
  en retard, non effectué) that is the closest documented pattern to a
  no-show mark, though it is applied to exams, not appointments.
  Source: https://www.smart-dz.com/produits-services/smartavicenne
- **ClinixaPro**. Described only in marketing terms as a full clinic ERP
  (500+ features, DPE, CNAS billing, PACS/RIS, operating theatre); no
  agenda-specific documentation was found.
  Source: https://www.clinixapro.com/solution/

### French and other products (public help pages)

- **Doctolib** has by far the most public documentation of any product
  checked. Its agenda pairs "motifs de consultation" (visit types) with
  "plages d'ouverture" (recurring or one-off open ranges); each motif's
  duration runs from 5 minutes to 10 hours, editable either for all future
  appointments of that motif or for one appointment at booking time.
  Absences ("Créer une absence") block online booking for the period and
  offer to move every existing appointment to a new time, with automatic
  SMS or email notice to affected patients or a manual call. A fill-rate
  cap can be set per motif on a given opening range so a doctor keeps
  headroom rather than filling every slot. The day list prints from list
  view with a choice of morning, afternoon or full day and selectable
  columns. A missed appointment is marked "absence non excusée," which
  triggers a warning email to the patient and can be escalated to blocking
  that patient from booking online at all. The waiting list only ever
  advances a booking a patient already holds to an earlier freed slot; it
  notifies the first 8 people on the list and does not create a booking
  from nothing.
  Sources:
  https://doctolib.zendesk.com/hc/fr/articles/23521218336276-G%C3%A9rer-le-param%C3%A9trage-de-votre-agenda-et-de-vos-motifs-de-consultation
  https://doctolib.zendesk.com/hc/fr/articles/12272323452436-Modifier-la-dur%C3%A9e-des-consultations
  https://doctolib.zendesk.com/hc/fr/articles/360059391171-Limiter-le-taux-de-remplissage-de-mon-agenda
  https://doctolib.zendesk.com/hc/fr/articles/4418152068116-Prendre-plusieurs-rendez-vous-pour-un-patient
  https://doctolib.zendesk.com/hc/fr/articles/207811593-Cr%C3%A9er-une-absence
  https://doctolib.zendesk.com/hc/fr/articles/4402689041812-Pr%C3%A9parer-mes-vacances-pour-partir-sereinement
  https://doctolib.zendesk.com/hc/fr/articles/203188789-Imprimer-la-liste-des-rendez-vous-de-la-journ%C3%A9e
  https://doctolib.zendesk.com/hc/fr/articles/360031239451-R%C3%A9duire-vos-rendez-vous-non-honor%C3%A9s
  https://doctolib.zendesk.com/hc/fr/articles/4402394446868-Consulter-et-g%C3%A9rer-les-listes-d-attente-patients
  https://doctolibpatient.zendesk.com/hc/fr/articles/360025470173-M-inscrire-%C3%A0-la-liste-d-attente
- **Weda**. Each practitioner has a separate agenda with visit types tied
  to preset durations that automatically set the slot length, and a
  multi-practitioner view for the whole team. Reminders go out by SMS and
  email at configurable delays (its own example: 48h and 2h before).
  Source: https://weda.fr/logiciel-gestion-rdv-medical
- **HelloDoc** (CGM, Agenda Web manual, 115 pages, the fullest agenda
  manual found for any product here). Absences and closures (office
  closed, a replacement doctor in) are entered as periods spanning whole
  or multiple days and manipulated by drag-and-drop and resizing in the
  graphical agenda, distinct from single-slot appointments. It offers Week
  and Month views alongside a Day view, and menu access to scheduling
  features depends on the logged-in user's role. Its own marketing page
  separately states appointments booked online sync to the agenda "sans
  risque de chevauchement" (no overlap risk).
  Sources: https://www.hellodoc-agenda.com/Documentations/HelloDocAgendaWeb-V3.pdf
  https://www.cgm.com/fra_fr/tous-nos-produits/logiciels-metiers/produit/hellodoc/agenda-et-rappel.html
- **MédiStory** (Mac/iPad). Consultations sync automatically to the agenda
  to avoid double entry and planning errors; it interoperates with
  DoctoDispo for online booking. No further agenda mechanics (visit
  types, absences, no-shows) were found on its public pages.
  Source: https://medistory.com/logiciel-medecin/
- **Maiia** (Cegedim). Its agenda has a patient waiting list "pour
  optimiser votre agenda et fidéliser votre patientèle," a configurable
  cancellation delay aimed at absenteeism, and statistics that break down
  appointments taken, honoured and cancelled. No public page gave the
  exact mechanics of visit-type durations.
  Source: https://maiia.zendesk.com/hc/fr/categories/360002444820-Maiia-Agenda
- **Doctoranytime**. Each consultation type is tied to a duration; the
  system sends an automatic reminder by email and SMS the day before and
  can carry consultation-specific instructions, and claims a 70% cut in
  missed appointments from those reminders. The agenda can sync to
  external calendars (iCloud, Google, Office365, Exchange, Outlook) and be
  shared across several practitioners at one centre.
  Source: https://pro.doctoranytime.be/fr/blog/comment-creer-mon-agenda-medical-en-ligne-et-quelles-en-sont-toutes-les-possibilites

"Revoir dans X jours" as a named follow-up-booking feature was not found in
any product's public documentation. The closest documented feature is
Doctolib's ability to book a recurring series or several appointments for
one patient in a single flow, which is a related but different thing: it
schedules a known sequence up front rather than suggesting a date from a
free-text instruction.

## Algeria specifics

**Public holidays.** Algeria's legal holidays are fixed by loi n° 63-278 du
26 juillet 1963, as amended by ordonnances n° 66-153 and n° 68-149 and,
most recently, by loi n° 23-10 du 26 juin 2023, which extended both Aïd
el-Fitr and Aïd al-Adha from two to three paid non-working days each. The
list holds 14 holidays: five fixed civil dates (1 January, 12 January
Yennayer since a 2017 presidential decision effective 2018, 1 May, 5 July
Independence Day, 1 November Revolution Day) and the Islamic holidays,
whose Gregorian date moves every year because it is set by the lunar
calendar. For 2026 the Islamic dates given by one calendar site are: Aïd
el-Fitr 20 March, Aïd al-Adha 27 May (plus the two following days under the
2023 law), 1 Muharram (Islamic new year) 17 June, Achoura 26 June, and
Mawlid Ennabawi 26 August; a diplomatic mission's own 2026 list put 1
Muharram and Achoura a day earlier, on 25 June, which is a normal
consequence of how these dates are fixed, not a source conflict to
resolve. Each Hijri date is only confirmed the evening before, after
Maghreb prayer, by the Commission nationale d'observation du croissant
lunaire under the Ministère des Affaires religieuses et des Wakfs; a book
that hardcodes a Hijri holiday's date more than a day ahead of the sighting
is guessing.
Sources:
https://fr.wikipedia.org/wiki/F%C3%AAtes_et_jours_f%C3%A9ri%C3%A9s_en_Alg%C3%A9rie
https://natlex.ilo.org/dyn/natlex2/r/natlex/fe/details?p3_isn=107481
https://gms-dz.com/wp-content/uploads/2025/01/Loi-n%C2%B0-23-10-Liste-des-fetes-legales-Modif.pdf
https://lepetitjournal.com/alger/comprendre-algerie/jours-feries-algerie-328951
https://www.bmeia.gv.at/fileadmin/user_upload/Vertretungen/Algier/Dokumente/2026_Jours_feries.pdf
https://www.elmoudjahid.dz/fr/info-en-continu/commission-nationale-d-observation-du-croissant-lunaire-lundi-1er-jour-du-mois-beni-de-ramadhan-en-algerie-27461

**The Friday-Saturday weekend.** Article 33 of loi n° 90-11 du 21 avril
1990 relative aux relations de travail sets Friday as the normal weekly
rest day for workers generally, which is the legal basis for the private
sector, a cabinet included. Décret exécutif n° 09-244 du 22 juillet 2009
moved the public administration's weekly rest from Thursday-Friday to
Friday-Saturday starting 14 August 2009; that decree governs public
institutions and administrations by its own text, and no source found
extends it by name to private practice, though in practice a private
cabinet in Algeria keeps the same Friday-Saturday rest because everyone
else, including public hospitals and referring institutions, does. Treat
Friday-Saturday as the working default and Saturday specifically as a
convention rather than a private-sector legal duty.
Sources:
https://natlex.ilo.org/dyn/natlex2/natlex2/files/download/9557/DZA-9557.pdf
https://www.france24.com/fr/20090722-jours-repos-hebdomadaires-decales-vendredi-samedi-

**Ordre and public-health obligations that bound the book.** The Code de
déontologie médicale algérien (décret exécutif n° 92-276 du 6 juillet
1992) obliges any physician to give emergency assistance to a person in
immediate danger, except in a case of force majeure; this is the ethical
basis for a cabinet having to fit an emergency in outside its booked grid,
though the exact article number could not be confirmed since the source
PDFs found could not be read as text (see below). Loi n° 18-11 du 2
juillet 2018 relative à la santé makes participation in garde
(on-call duty) a legal obligation for health personnel, including private
practitioners who can be called on to staff a public health structure's
garde; this is an obligation on the doctor's calendar as a whole (a duty
day elsewhere), not a rule about how their own cabinet's book must be
built, so a book should be able to block a garde day the same way it
blocks any other absence.
Sources:
https://www.fao.org/faolex/results/details/en/c/LEX-FAOC181216
http://www.snapo-dz.net/site/wp-content/uploads/2021/08/Decret-90-276-code-deontologie-medicale.pdf

## What stays unsourced

- Any rule, fee or Ordre position specific to a missed or late-cancelled
  appointment in Algeria. Not found; French practice (no fee may be
  charged for an act not actually performed, per France's own code de la
  santé publique) is not evidence for Algerian rules and is not carried
  over.
- A Friday-prayer break as a named custom or rule for cabinets. Not found
  in any source consulted.
- Ramadan working-hour customs for private medical cabinets specifically.
  The only sourced figure (Sunday-Thursday 8h30-15h00) is the public
  administration's schedule, republished every year by the government;
  no source ties it to private cabinets, which commonly shorten hours on
  their own but without a documented rule.
- The exact article number in décret exécutif n° 92-276 covering emergency
  assistance and continuity of care. The two full-text PDFs found
  (univ.ency-education.com and snapo-dz.net) could not be extracted as
  readable text by the tools available in this session; the obligation
  itself is corroborated by secondary summaries, not by a quoted article.
- Whether décret exécutif n° 09-244's Friday-Saturday rest binds private
  cabinets by law, as opposed to by convention. Not found either way.
- Detailed agenda mechanics (visit-type durations, no-show marking,
  printing) for CabiSanté, GCDOC and ClinixaPro. Their public pages are
  marketing copy; none publishes user documentation of the agenda module
  itself.

## Sources

- https://www.winmed.africa/aide_med.html
- https://cabisante.com/
- https://www.facebook.com/GCDOC.Logiciel.de.Gestion.de.Cabinet.Medical/
- https://www.smart-dz.com/produits-services/smartavicenne
- https://www.clinixapro.com/solution/
- https://doctolib.zendesk.com/hc/fr/articles/23521218336276-G%C3%A9rer-le-param%C3%A9trage-de-votre-agenda-et-de-vos-motifs-de-consultation
- https://doctolib.zendesk.com/hc/fr/articles/12272323452436-Modifier-la-dur%C3%A9e-des-consultations
- https://doctolib.zendesk.com/hc/fr/articles/360059391171-Limiter-le-taux-de-remplissage-de-mon-agenda
- https://doctolib.zendesk.com/hc/fr/articles/4418152068116-Prendre-plusieurs-rendez-vous-pour-un-patient
- https://doctolib.zendesk.com/hc/fr/articles/207811593-Cr%C3%A9er-une-absence
- https://doctolib.zendesk.com/hc/fr/articles/4402689041812-Pr%C3%A9parer-mes-vacances-pour-partir-sereinement
- https://doctolib.zendesk.com/hc/fr/articles/203188789-Imprimer-la-liste-des-rendez-vous-de-la-journ%C3%A9e
- https://doctolib.zendesk.com/hc/fr/articles/360031239451-R%C3%A9duire-vos-rendez-vous-non-honor%C3%A9s
- https://doctolib.zendesk.com/hc/fr/articles/4402394446868-Consulter-et-g%C3%A9rer-les-listes-d-attente-patients
- https://doctolibpatient.zendesk.com/hc/fr/articles/360025470173-M-inscrire-%C3%A0-la-liste-d-attente
- https://weda.fr/logiciel-gestion-rdv-medical
- https://www.hellodoc-agenda.com/Documentations/HelloDocAgendaWeb-V3.pdf
- https://www.cgm.com/fra_fr/tous-nos-produits/logiciels-metiers/produit/hellodoc/agenda-et-rappel.html
- https://medistory.com/logiciel-medecin/
- https://maiia.zendesk.com/hc/fr/categories/360002444820-Maiia-Agenda
- https://pro.doctoranytime.be/fr/blog/comment-creer-mon-agenda-medical-en-ligne-et-quelles-en-sont-toutes-les-possibilites
- https://fr.wikipedia.org/wiki/F%C3%AAtes_et_jours_f%C3%A9ri%C3%A9s_en_Alg%C3%A9rie
- https://natlex.ilo.org/dyn/natlex2/r/natlex/fe/details?p3_isn=107481
- https://gms-dz.com/wp-content/uploads/2025/01/Loi-n%C2%B0-23-10-Liste-des-fetes-legales-Modif.pdf
- https://lepetitjournal.com/alger/comprendre-algerie/jours-feries-algerie-328951
- https://www.bmeia.gv.at/fileadmin/user_upload/Vertretungen/Algier/Dokumente/2026_Jours_feries.pdf
- https://www.elmoudjahid.dz/fr/info-en-continu/commission-nationale-d-observation-du-croissant-lunaire-lundi-1er-jour-du-mois-beni-de-ramadhan-en-algerie-27461
- https://natlex.ilo.org/dyn/natlex2/natlex2/files/download/9557/DZA-9557.pdf
- https://www.france24.com/fr/20090722-jours-repos-hebdomadaires-decales-vendredi-samedi-
- https://www.fao.org/faolex/results/details/en/c/LEX-FAOC181216
- http://www.snapo-dz.net/site/wp-content/uploads/2021/08/Decret-90-276-code-deontologie-medicale.pdf
