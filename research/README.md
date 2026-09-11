# dz-pos research

Research for this product, kept in the repo next to `context/` (planning)
and `docs/` (the spec). pc-ctx's research domain is bound here through
`PC_CTX_RESEARCH_DIR`, so `ctx research list` / `research show` from
`context/` browse it. Nothing dz-pos lives in the ObserveOne workspace.

## Layout

| Folder | What goes there |
|---|---|
| `legal-fiscal/` | what the law says: droit de timbre, TVA, facture mentions, IFU, numbering, amount in words. Each doc cites code + article + year. `sources/` holds the official PDFs the docs quote (DGI codes fiscaux, Journal Officiel). |
| `tooling/` | official skills, MCP servers and plugins evaluated for the stack (Tauri, Expo, Rust, Playwright), with install commands and a verdict. Also conventions read out of another codebase before adopting them here. |
| `competitors/` | Lumina POS teardown, MonStock gap analysis, market landscape, brief for Anouar. |
| `market/` | pricing, distribution, shop interviews when they happen (checked 2026-09-11: no folder yet, no pages gathered; the market pass done so far is `competitors/2026-09-07-lumina-and-market/market-landscape.md`). |

Name a dated doc `YYYY-MM-DD-slug.md`. One topic per file.

## Rules

- **Nothing from the Lumina teardown becomes source code.** It is a
  competitor's proprietary code obtained by unpacking their installer.
  Copying it is infringement, and it encodes their reading of the law at
  the time they wrote it, which is already wrong for the droit de timbre
  (flat 1 % capped at 2 500 DA; the 2025 finance law made it progressive).
  The teardown is evidence of features and of what the market accepts,
  nothing more.
- **A fiscal claim cites a primary source or says it does not.** Primary:
  the DGI code PDFs in `legal-fiscal/sources/`, the Journal Officiel on
  joradp.dz, commerce.gov.dz regulation pages. Secondary (vendor blogs,
  accountants' sites) may point the way but do not settle a rule.
- A rule that lands in `docs/features.md` in the code repo carries the
  citation in its `Source` column and the fixture that pins it.
