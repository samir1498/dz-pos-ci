export type Locale = "fr" | "en" | "ar";

export interface Dict {
  readonly lang: Locale;
  readonly dir: "ltr" | "rtl";
  readonly title: string;
  /** Same two words as apps/desktop/src/components/Wordmark.tsx's brand_word
   *  and brand_suffix: the coin is Dinar, the word beside it is the
   *  category (POS / بوس). */
  readonly brandWord: string;
  readonly brandSuffix: string;
  /**
   * The one honest line L0 is allowed (context/plans/20260911-landing-page.md,
   * L0: "do not write marketing prose beyond one honest line per section").
   * L2 owns the real copy; this line only has to be true.
   */
  readonly tagline: string;

  /** The hero: what the till does, in one line, and what the shop is not
   * signing up for, said once. L3 pairs heading with the L1 till shot. */
  readonly hero: {
    readonly heading: string;
    readonly body: string;
    readonly cta: string;
  };

  /** The four pieces named in the plan (L3, task L2): sell, invoice, stock,
   * know. One sentence each, what a shop owner would recognise doing. */
  readonly pieces: {
    readonly heading: string;
    readonly sell: { readonly title: string; readonly body: string };
    readonly invoice: { readonly title: string; readonly body: string };
    readonly stock: { readonly title: string; readonly body: string };
    readonly know: { readonly title: string; readonly body: string };
  };

  /**
   * What the person behind the counter can and cannot do, for a shop with
   * more than one person working it. Every line has to already be true in
   * `crates/core/src/services/permissions.rs`'s `can(role, permission)`
   * table and in `docs/features.md` §5; each field's own comment in fr.ts
   * names what it is checked against. Placed after the four pieces and
   * before the fiscal block, per the 2026-09-11 brief that asked for this
   * section (staff accounts shipped after the landing page's own plan,
   * context/plans/20260911-landing-page.md, was already marked done).
   */
  readonly staff: {
    readonly heading: string;
    readonly intro: string;
    readonly signIn: string;
    readonly cashierScope: string;
    readonly cashierLimits: string;
    readonly manager: string;
    readonly log: string;
  };

  /**
   * The fiscal answers an Algerian owner asks first. Every claim here has to
   * already be true in docs/features.md; each field's own comment in fr.ts
   * names the row. Nothing here is a legal opinion, only what the software
   * does.
   */
  readonly fiscal: {
    readonly heading: string;
    readonly intro: string;
    /** docs/features.md §3, "Numbering" row: gapless yearly series, numbers
     * never reused. */
    readonly numbering: string;
    /** docs/features.md §3, "Droit de timbre" row: cash payments carry the
     * stamp, worked out from the amount. */
    readonly stamp: string;
    /** docs/features.md §3, "TVA rates" row: 19 %, 9 % or 0 %, per product. */
    readonly tva: string;
    /** docs/features.md §3, "Amount in words" row: French, Arabic and
     * English generators. */
    readonly words: string;
    /** docs/features.md §3, "Régime fiscal" row: a dated shop setting, `ifu`
     * or `réel`; under IFU no document names a tax, and a document keeps the
     * regime it was issued under. */
    readonly regime: string;
  };

  /** Three languages as a feature (docs/features.md, Scope: "Arabic (RTL),
   * French, English on every screen and every printed document,
   * independently selectable"), not a footnote. */
  readonly languages: {
    readonly heading: string;
    readonly body: string;
  };

  /** L3: heading for the band of real screens (L1's collage shot). Added by
   * L3 because the dictionary had no words for this section; see the L3
   * report for why. */
  readonly screens: {
    readonly heading: string;
  };

  readonly audience: {
    readonly heading: string;
    readonly body: string;
  };

  /** No price here: it is Anouar's to set (docs/features.md, open decision
   * 1). The form collects a name, a phone and a wilaya (L3). */
  readonly pricing: {
    readonly heading: string;
    readonly body: string;
    readonly formNameLabel: string;
    readonly formPhoneLabel: string;
    readonly formWilayaLabel: string;
    readonly formSubmit: string;
    /** L3: the reason shown beside the disabled submit button, since the
     * form posts nowhere until Anouar names an address (see the L3 report).
     */
    readonly formDisabledNote: string;
  };

  /**
   * One button per installer (lib/site.ts DOWNLOAD_FILES), with the OS
   * sniffing done in the browser: the server renders all three links the
   * same, a small script marks the visitor's own. Unsigned until the
   * Windows certificate lands (docs/release-checklist.md), said plainly
   * under the buttons rather than hidden. Last section on the page, after
   * pricing.
   */
  readonly download: {
    readonly heading: string;
    readonly body: string;
    readonly windowsLabel: string;
    readonly windowsNote: string;
    readonly linuxLabel: string;
    readonly linuxNote: string;
    /** Badge on the button matching the visitor's OS. */
    readonly recommended: string;
    /** Shown while no release is signed (see site.ts: drafts never move
     * `releases/latest`). */
    readonly unsignedNote: string;
    readonly allLink: string;
  };
}
