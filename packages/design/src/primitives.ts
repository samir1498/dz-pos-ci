// Tier 1. Raw ramps, no roles. Values copied from design/shared/tokens.css,
// which is the hand-written source while the mockups still load it directly.
// Nothing outside src/semantic.ts imports this file; a convention, not yet
// a lint rule.

export type PrimitiveFamily = "stone" | "teal" | "red" | "amber" | "blue";

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
  red: {
    50: "#fdeeed",
    100: "#f9d6d3",
    500: "#c9403a",
    600: "#a8332e",
    700: "#862a26",
  },
  amber: {
    50: "#fdf5e6",
    100: "#f9e6bf",
    500: "#c98a1a",
    700: "#8a5e0f",
  },
  blue: {
    50: "#eaf1fb",
    500: "#2f6fcf",
    700: "#1f4c90",
  },
};

/** Families in emission order, so the CSS block matches the source file. */
export const families: readonly PrimitiveFamily[] = ["stone", "teal", "red", "amber", "blue"];

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
