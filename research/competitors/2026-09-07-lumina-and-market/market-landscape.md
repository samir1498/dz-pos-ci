# Algerian POS / stock / invoicing — market landscape

Surfaced 2026-09-07 while checking product-name availability. The market is far
more crowded than "Lumina plus Fatora". None of these has been examined beyond
its landing page; this is a list to triage, not a teardown.

| Product | What it claims | Note |
|---|---|---|
| [Lumina POS](lumina-pos-teardown.md) | Desktop POS + mobile, offline, 12,000 DZD one-time | Torn down. |
| [Fatoura](https://fatoura.app/) | Online invoicing + [Fatoura Stock](https://fatoura.app/stock) for SMEs | **Listed as a product by [Algérie Télécom](https://www.algerietelecom.dz/fr/produits/fatoura-prod191)** — that is a distribution channel nobody else has. |
| [Almawarid](https://almawarid.app/) | POS, cash sessions, 80mm tickets, stock, TVA 19% invoicing, proforma, avoir, bon de commande | Content-marketing heavy; ranks for most French queries. |
| [AlgoStock](https://algosynapse.com/algostock/) | "Meilleur logiciel POS & gestion de stock en Algérie", 100% offline | Direct Lumina competitor on the offline pitch. |
| [FooRa](https://foora-facture.com/) | Algerian invoicing, regulation-compliant | Invoicing only. |
| [Factury](https://factury.app/) | Free invoicing + gestion commerciale, devis, trésorerie | "Gratuit" — pricing pressure. |
| [GestiumPRO](https://cirtasoft.com/gestiumpro/) (Cirtasoft) | Gestion commerciale for PME | Older-style vendor. |
| [COMMSoft](https://megatheque.net/logiciel-gestion-stock-commerciale-algerie/) | Stock + commerciale | Older-style vendor. |
| [SmartCom](https://www.smart-dz.com/produits-services/smartcom) | Gestion commerciale | Older-style vendor. |
| [Motakamel Plus](https://www.solutionsinformatiques.dz/Motakamel-Plus-43) | Gestion commerciale | Older-style vendor. |
| [Inabex](https://inabex.com/logiciels-de-gestion-en-algerie/) | Commerciale, stock, ERP, mobile | Older-style vendor. |
| [Mizan](https://mizane.app/) | Invoicing + fiscal for auto-entrepreneurs, Law 22-23 / ANAE / IFU | Adjacent — the auto-entrepreneur niche, not shops. |
| [Doliflex](https://doliflex.net/) | Dolibarr ERP localised for Algeria | Open-source base. |

## What this changes

- The "one incumbent selling well" framing was wrong. There are at least a
  dozen. Positioning has to be against a category, not against Lumina.
- Fatoura's Algérie Télécom listing is the single most important fact on this
  page. Ask Anouar whether that channel is reachable.
- Two clusters: modern web/mobile apps with content marketing (Almawarid,
  Fatoura, Factury) and older desktop vendors selling through resellers
  (GestiumPRO, COMMSoft, SmartCom, Motakamel). Lumina sits between — modern
  stack, phone-call sales.
- Free tiers exist (Factury). One-time 12,000 DZD is not the floor.

## Product name — availability check

Checked 2026-09-07: RDAP for `.com/.dz/.app/.io`, GitHub org/user, and a web
search for products in the space.

| Name | Verdict | Why |
|---|---|---|
| Daftar | Dead | [Daftar on Google Play](https://play.google.com/store/apps/details?id=com.bennu.daftar) is an offline sales/inventory/invoicing app with Bluetooth thermal printing; [Daftra](https://www.daftra.com/en/pos/) is a major Arab-world POS. |
| Hanout | Dead | [hanout.net](https://www.hanout.net/) is a boutique-management product; [Lhanout](https://lhanout.ma/) (Morocco); [Hanout OS](https://gethanout.store/). |
| Mizan | Dead | [mizane.app](https://mizane.app/) is an Algerian invoicing/fiscal app; several other Mizan apps. |
| Zimam / Zmam | Dead | [getzimam.com](https://getzimam.com/) is an Algerian COD e-commerce platform. Same word, same market. |
| Qayd | Dead | [qayd.app](https://qayd.app/) is Arabic double-entry accounting. |
| Tijara, Hisab | Dead | Everything taken, generic. |
| **Sijil** | Only survivor | No product in commerce/POS found. `sijil.dz`, `sijil.io` free; GitHub org free. `sijil.app` is an unrelated team-commitments tool; `sijil.com` registered, no response. |

Sijil (سجل — register, record; as in السجل التجاري) is usable but not
distinctive. Worth a second round of candidates rather than settling.
