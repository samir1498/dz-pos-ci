// One select. It writes the shop's choice and nothing else; the attribute and
// the repaint are the provider's job (src/lib/theme.tsx), and the colours are
// the stylesheet's.
//
// This file and the provider are the only two in the app allowed to name a
// theme, because a label has to say "Comptoir" somewhere. Everywhere else a
// component wears `bg-background` and `text-muted-foreground` and never learns
// which theme is on.

import { DEFAULT_THEME, THEMES } from "@dzpos/design";
import type { ThemeName } from "@dzpos/design";
import type { ThemeDto } from "@dzpos/shared";
import { Palette } from "lucide-react";

import { Icon } from "@/components/Icon";
import { useTranslation, type Key } from "@/i18n";
import { useTheme } from "@/lib/theme";

/**
 * The label key of each theme, spelled out rather than built from the name:
 * a key the i18n files do not carry has to fail the parity test, and a
 * template string would hide it from that check.
 *
 * Keyed by `ThemeName` rather than `string`, so a fifth theme added to the
 * package fails to compile here until it has a label. It used to be a
 * `Record<string, Key>` with a `?? "theme_label"` at the point of use, which
 * compiled fine and shipped a select whose new entry read "Theme". The type
 * is the check now, and `src/theme.test.ts` proves the keys the three
 * dictionaries carry.
 */
export const THEME_LABEL: Readonly<Record<ThemeName, Key>> = {
  comptoir: "theme_comptoir",
  registre: "theme_registre",
  observe: "theme_observe",
  "observe-dark": "theme_observe_dark",
};

/** The value the select shows. Guards the read rather than asserting it. */
function toChoice(value: string): ThemeDto {
  return THEMES.find((name): name is ThemeDto => name === value) ?? DEFAULT_THEME;
}

export function ThemeSwitcher({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { choice, setChoice, saving } = useTheme();
  return (
    <label className={className === undefined ? "flex items-center gap-2" : `flex items-center gap-2 ${className}`}>
      <Icon as={Palette} size={18} className="text-muted-foreground" />
      <span className="sr-only">{t("theme_label")}</span>
      <select
        data-testid="theme-switcher"
        aria-label={t("theme_label")}
        disabled={saving}
        className="rounded-md border border-border bg-card px-2 py-1 text-sm text-foreground"
        value={choice ?? DEFAULT_THEME}
        onChange={(e) => setChoice(toChoice(e.target.value))}
      >
        {THEMES.map((name) => (
          <option key={name} value={name}>
            {t(THEME_LABEL[name])}
          </option>
        ))}
      </select>
    </label>
  );
}
