// What the patients list and the fiche both say about a patient. The
// folder is `-patients`, which the router leaves alone, the way the
// customers screen keeps its own bits in `-customers`.

import type { SexDto } from "@dzpos/shared";

import type { Key } from "@/i18n";

export const SEXES: readonly SexDto[] = ["female", "male"];

export const SEX_KEY: Readonly<Record<SexDto, Key>> = {
  female: "sex_female",
  male: "sex_male",
};
