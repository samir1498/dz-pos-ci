export type Locale = "fr" | "en" | "ar";

export interface Dict {
  readonly lang: Locale;
  readonly dir: "ltr" | "rtl";
  readonly title: string;
  /** Same two words as apps/desktop/src/components/Wordmark.tsx's brand_word
   *  and brand_suffix: the coin never changes, the word beside it does.
   *  "Dinar POS" is a placeholder pending Anouar's final name
   *  (docs/features.md, open decision 2). */
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
  };

  /** Three languages as a feature (docs/features.md, Scope: "Arabic (RTL),
   * French, English on every screen and every printed document,
   * independently selectable"), not a footnote. */
  readonly languages: {
    readonly heading: string;
    readonly body: string;
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
  };
}
