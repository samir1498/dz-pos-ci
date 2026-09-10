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
  | "paper";

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
  // Brass: the one action that moves money, in both themes.
  brass: {
    300: "#e9d9b0",
    500: "#b8862b",
    700: "#8f6820",
  },
  // Warm paper: the text ramp on ink, and the inverse surface on Registre.
  paper: {
    50: "#f4f1e8",
    100: "#d9d2c0",
    200: "#b9b3a2",
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
