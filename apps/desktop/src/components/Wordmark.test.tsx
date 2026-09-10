// The word follows the language and the coin does not. One test per
// language, because the whole point of the pair is that fr and en share the
// Latin word while ar swaps it for the Arabic one and nothing else moves.

import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { I18nProvider, type Lang } from "@/i18n";
import ar from "@/i18n/ar.json";
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";

import { Wordmark } from "./Wordmark";

const DICT = { ar, en, fr };

function renderIn(lang: Lang) {
  return render(
    <I18nProvider lang={lang}>
      <Wordmark />
    </I18nProvider>,
  );
}

describe("Wordmark", () => {
  test.each<[Lang]>([["fr"], ["en"], ["ar"]])("sets the word of %s", (lang) => {
    renderIn(lang);
    const mark = screen.getByTestId("wordmark");
    expect(mark).toHaveTextContent(DICT[lang].brand_word);
    expect(mark).toHaveTextContent(DICT[lang].brand_suffix);
  });

  test("fr and en share the Latin word; ar does not", () => {
    expect(fr.brand_word).toBe(en.brand_word);
    expect(ar.brand_word).not.toBe(en.brand_word);
  });

  test("the coin is the same file in every language", () => {
    const sources = (["fr", "en", "ar"] satisfies Lang[]).map((lang) => {
      const { unmount } = renderIn(lang);
      const source = screen.getByTestId("wordmark-coin").getAttribute("src");
      unmount();
      return source;
    });
    expect(new Set(sources).size).toBe(1);
    // Vite inlines an SVG this small as a data URI rather than emitting a
    // file, so the assertion is on what the coin is, not on its filename.
    expect(sources[0]).toMatch(/^data:image\/svg\+xml,|\.svg$/);
  });

  /**
   * The coin is decoration beside the word, not a second name: announced as
   * an image it would make a screen reader read the brand twice.
   */
  test("leaves the coin out of the accessible name", () => {
    renderIn("fr");
    expect(screen.getByTestId("wordmark-coin")).toHaveAttribute("alt", "");
  });
});
