---
title: 'Legal, fiscal and tooling research for dz-pos'
slug: 'legal-fiscal-and-tooling-research-for-dz-pos'
status: 'active'
category: 'research'
created: 20260908
tldr: 'Read the law before pinning a fiscal rule: droit de timbre, TVA, facture mentions, IFU, numbering, amount in words; catalog official skills and MCPs; diff Lumina against the law; comptable review'
tasks:
  - id: 'R1'
    desc: 'Droit de timbre: read Code du timbre 2026 art. 100 (whole article incl. II) and 258 ff.; settle marginal vs whole-amount banding and the 300 DA edge; write legal-fiscal/droit-de-timbre.md; rewrite the fixture spec as stamp_progressive_tranches'
    status: 'pending'
  - id: 'R2'
    desc: 'TVA: CTCA 2026 art. 21/23 list of 9 % goods relevant to a supérette, LF 2026 changes; download CIDTA 2026 and read art. 324 (rounding) via CTCA art. 80bis; write legal-fiscal/tva-and-rounding.md'
    status: 'pending'
  - id: 'R3'
    desc: 'Facture and ticket: décret 05-468 full text (numbering, bon de livraison, facture récapitulative), arrêté 1er août 2013, loi 04-02 on what a consumer ticket must carry; write legal-fiscal/facture-and-ticket.md with the field list per document kind'
    status: 'pending'
  - id: 'R4'
    desc: 'IFU regime: threshold, who qualifies, what an IFU seller''s facture shows (no TVA line); decide whether the shop needs a régime setting; write legal-fiscal/ifu-regime.md'
    status: 'pending'
  - id: 'R5'
    desc: 'Gapless numbering: find which text requires it (05-468 numéro d''ordre, code de commerce, procédures fiscales) and what a void/cancel must look like'
    status: 'pending'
  - id: 'R6'
    desc: 'Amount in words: French wording on printed factures, Arabic convention (native review), whether English is ever needed; feed the golden fixtures'
    status: 'pending'
  - id: 'R7'
    desc: 'Lumina versus the law: complete the diff table in fiscal-sources-and-findings.md; every row cites an article or says assumption'
    status: 'pending'
  - id: 'R8'
    desc: 'Comptable review with Anouar: send R1–R6 as one page of questions; record answers with who and when in docs/features.md Source column'
    status: 'pending'
  - id: 'R9'
    desc: 'Tooling: read the tooling catalog; try anthropics webapp-testing and frontend-design on the first real screen; install expo plugin when mobile starts; one trial of hypothesi Tauri MCP on the laptop; record verdicts in tooling/'
    status: 'pending'
acceptance:
  - 'Every row of the fiscal rules table in docs/features.md cites code, article and edition, or the comptable''s answer with date; no row says assumption'
references:
  - 'research:dz-pos/legal-fiscal/2026-09-08-fiscal-sources-and-findings.md'
---
# Legal, fiscal and tooling research for dz-pos

## Goal

TODO: define goal

## Scope

TODO: define scope
