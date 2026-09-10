// Tier 1. Raw ramps, no roles. Values copied from design/shared/tokens.css,
// which is the hand-written source while the mockups still load it directly.
// Nothing outside src/semantic.ts imports this file; a convention, not yet
// a lint rule.

export type PrimitiveFamily =
  | "stone"
  | "teal"
  | "red"
  | "amber"
  | "blue"
  | "ink"
  | "brass"
  | "paper"
  | "slate"
  | "night"
  | "emerald"
  | "rose"
  | "gold";

export type Ramp = Readonly<Record<number, string>>;

export const primitives: Readonly<Record<PrimitiveFamily, Ramp>> = {
  stone: {
    0: "#ffffff",
    50: "#f8f7f4",
    100: "#f1efea",
    200: "#e5e2db",
    300: "#d3cfc5",
    400: "#a9a49a",
    500: "#7d786f",
    600: "#5c5850",
    700: "#443f38",
    800: "#2b2823",
    900: "#1a1815",
  },
  teal: {
    50: "#eaf6f3",
    100: "#cfeae2",
    300: "#7fc7b3",
    500: "#1f8a70",
    600: "#17715b",
    700: "#125a49",
    800: "#0e463a",
  },
  // The 300 step of red, amber and blue exists for Registre: the 500s are
  // mixed for white paper and fall under 4.5:1 on an ink surface. The 900
  // step is the soft background those three get on the dark theme, where
  // the 50 would be a white card. src/css.test.ts asserts both.
  red: {
    50: "#fdeeed",
    100: "#f9d6d3",
    300: "#e8867e",
    500: "#c9403a",
    600: "#a8332e",
    700: "#862a26",
    900: "#4a1f1c",
  },
  amber: {
    50: "#fdf5e6",
    100: "#f9e6bf",
    300: "#e9c05f",
    500: "#c98a1a",
    700: "#8a5e0f",
    900: "#4a3610",
  },
  blue: {
    50: "#eaf1fb",
    300: "#8db3ea",
    500: "#2f6fcf",
    700: "#1f4c90",
    900: "#16305c",
  },
  // Ink green: Registre's surfaces and borders, and the sidebar Comptoir
  // borrows from it. Values from the approved direction pages (Registre and
  // the Comptoir blend), not mixed here.
  ink: {
    200: "#7a877f",
    300: "#4a5a53",
    400: "#3a4b44",
    500: "#2c4038",
    600: "#23392f",
    700: "#1b2c25",
    800: "#14211c",
  },
  // Brass: the one action that moves money, in every theme. 400 is the step
  // Observe's dark twin lifts the accent to on an almost-black ground.
  brass: {
    300: "#e9d9b0",
    400: "#d4a548",
    500: "#b8862b",
    700: "#8f6820",
  },
  // Warm paper: the text ramp on ink, and the inverse surface on Registre.
  paper: {
    50: "#f4f1e8",
    100: "#d9d2c0",
    200: "#b9b3a2",
  },
  // The five below belong to Observe and its dark twin, lifted from the
  // approved direction page. Cool greys where Comptoir is warm, an emerald
  // brand where the other two are teal, and their own red and amber: the
  // whole point of the third theme is that it is another palette, not
  // Comptoir with the knobs turned.
  slate: {
    50: "#f8fafc",
    100: "#f1f5f9",
    200: "#e2e8f0",
    300: "#cbd5e1",
    400: "#94a3b8",
    600: "#475569",
    900: "#0f172a",
  },
  night: {
    300: "#7b8b83",
    500: "#2b3a32",
    600: "#1f2b25",
    700: "#17211c",
    800: "#0e1713",
    900: "#09090b",
  },
  emerald: {
    50: "#ecfdf5",
    400: "#34d399",
    500: "#10b981",
    600: "#059669",
    950: "#04150d",
  },
  rose: {
    50: "#fef2f2",
    400: "#f87171",
    600: "#dc2626",
  },
  gold: {
    50: "#fffbeb",
    400: "#fbbf24",
    600: "#d97706",
  },
};

/** Families in emission order, so the CSS block matches the source file. */
export const families: readonly PrimitiveFamily[] = [
  "stone",
  "teal",
  "red",
  "amber",
  "blue",
  "ink",
  "brass",
  "paper",
  "slate",
  "night",
  "emerald",
  "rose",
  "gold",
];

export const rampOf = (family: PrimitiveFamily): Ramp => {
  const ramp = primitives[family];
  if (ramp === undefined) {
    throw new Error(`unknown primitive family: ${family}`);
  }
  return ramp;
};

export const hexOf = (family: PrimitiveFamily, step: number): string => {
  const value = rampOf(family)[step];
  if (value === undefined) {
    throw new Error(`unknown primitive step: ${family}/${step}`);
  }
  return value;
};
