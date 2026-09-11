// One dictionary per language, each in its own file, so L2 can rewrite the
// copy without touching src/layouts/Base.astro or the page routes.

import { ar } from "./ar";
import { en } from "./en";
import { fr } from "./fr";

export const dictionaries = { fr, en, ar } satisfies Record<string, unknown>;

export type { Dict, Locale } from "./types";
