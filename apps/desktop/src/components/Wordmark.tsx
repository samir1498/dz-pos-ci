// The brand in the sidebar: one coin and one word. The coin is the same file
// in every language, the word is the language's own, and the pair sits on the
// reading side because the row inherits the page's direction rather than a
// left or a right.
//
// The word is HTML, not the SVG wordmark beside the coin in
// packages/design/assets. The SVG files carry live <text> (there is no font
// binary on this box to outline them with, see the README there), so a
// renderer without Plex substitutes its own face; the app has Plex vendored
// and can simply set the words with it.

import markUrl from "@dzpos/design/assets/mark.svg";

import { useTranslation } from "@/i18n";

export function Wordmark({ className }: { className?: string }) {
  const { t } = useTranslation();
  return (
    <span
      data-testid="wordmark"
      className={`flex items-center gap-2 ${className ?? ""}`.trimEnd()}
    >
      {/* alt="" so the coin is decoration: the word beside it is already the
          accessible name, and an alt here would have a screen reader read
          the brand twice. */}
      <img data-testid="wordmark-coin" src={markUrl} alt="" width={28} height={28} />
      <span className="text-lg font-bold leading-none">
        {t("brand_word")} <span className="font-semibold text-money">{t("brand_suffix")}</span>
      </span>
    </span>
  );
}
