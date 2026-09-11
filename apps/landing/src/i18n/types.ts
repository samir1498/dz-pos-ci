export type Locale = "fr" | "en" | "ar";

export interface Dict {
  readonly lang: Locale;
  readonly dir: "ltr" | "rtl";
  readonly title: string;
  /** Same two words as apps/desktop/src/components/Wordmark.tsx's brand_word
   *  and brand_suffix: the coin never changes, the word beside it does. */
  readonly brandWord: string;
  readonly brandSuffix: string;
  /**
   * The one honest line L0 is allowed (context/plans/20260911-landing-page.md,
   * L0: "do not write marketing prose beyond one honest line per section").
   * L2 owns the real copy; this line only has to be true.
   */
  readonly tagline: string;
}
