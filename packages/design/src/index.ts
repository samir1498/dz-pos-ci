// The package API. `primitives` is deliberately not re-exported: tier 1 is
// reachable only at "@dzpos/design/primitives", a path the apps never
// import. A convention for now; no lint rule enforces it yet.

export { theme } from "./theme";
export type { Theme } from "./theme";
export { toCss } from "./css";
export {
  borderColor,
  color,
  fontFamily,
  fontSize,
  layout,
  radius,
  rtl,
  shadow,
  space,
  surface,
  textColor,
} from "./semantic";
export type { PxGroup, Token, TokenGroup, TokenLiteral, TokenPx, TokenRef } from "./semantic";
