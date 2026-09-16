// Typography. Every string on a screen goes through this rather than a bare
// `<Text>`, so a size or a colour is a token name and never a number typed
// into a style. The variants are the ones a till actually needs: a screen
// title, a label over a value, body copy, a refusal, and the big amount the
// customer is being asked for.

import {
  Text as RnText,
  type TextProps as RnTextProps,
  StyleSheet,
  type TextStyle,
} from "react-native";

import { useTheme } from "../../design/theme";

export type TextVariant = "title" | "heading" | "body" | "label" | "amount" | "caption";
export type TextTone =
  | "primary"
  | "secondary"
  | "tertiary"
  | "disabled"
  | "danger"
  | "success"
  | "inverse";

export type TextProps = RnTextProps & {
  variant?: TextVariant;
  tone?: TextTone;
};

export function Text({ variant = "body", tone = "primary", style, ...rest }: TextProps) {
  const theme = useTheme();

  const byVariant: Record<TextVariant, TextStyle> = {
    title: { fontSize: theme.fontSize["2xl"], fontWeight: "700" },
    heading: { fontSize: theme.fontSize.xl, fontWeight: "600" },
    body: { fontSize: theme.fontSize.md, fontWeight: "400" },
    label: { fontSize: theme.fontSize.sm, fontWeight: "500" },
    // The amount owed is the one number in the room the customer also cares
    // about, so it is the largest thing on the screen and tabular, which
    // stops the digits shifting as the basket changes.
    amount: {
      fontSize: theme.fontSize["3xl"],
      fontWeight: "700",
      fontVariant: ["tabular-nums"],
    },
    caption: { fontSize: theme.fontSize.xs, fontWeight: "400" },
  };

  const byTone: Record<TextTone, string> = {
    primary: theme.colors.text.primary,
    secondary: theme.colors.text.secondary,
    tertiary: theme.colors.text.tertiary,
    disabled: theme.colors.text.disabled,
    danger: theme.colors.text.danger,
    success: theme.colors.text.success,
    // What sits on a filled button. The token is named for the fill it sits
    // on, not for being "inverse" — there is no inverse text colour.
    inverse: theme.colors["on-primary"],
  };

  return <RnText style={StyleSheet.flatten([byVariant[variant], { color: byTone[tone] }, style])} {...rest} />;
}
