// The one button. Variants are roles, not looks: `primary` is the action the
// screen exists for, `secondary` is the way back, `danger` is the one that
// takes something away.
//
// Minimum height comes from the `touch-min` layout token (44), which is the
// smallest target a thumb reliably hits — a cashier is using this one-handed
// beside a customer, not sitting down with a mouse.

import {
  ActivityIndicator,
  Pressable,
  StyleSheet,
  View,
  type PressableProps,
} from "react-native";

import { useTheme } from "../../design/theme";
import { Text } from "./Text";

export type ButtonVariant = "primary" | "secondary" | "danger";

export type ButtonProps = Omit<PressableProps, "children"> & {
  title: string;
  variant?: ButtonVariant;
  busy?: boolean;
};

export function Button({
  title,
  variant = "primary",
  busy = false,
  disabled,
  style,
  ...rest
}: ButtonProps) {
  const theme = useTheme();
  const off = disabled === true || busy;

  const fill: Record<ButtonVariant, string> = {
    primary: theme.colors.primary,
    secondary: theme.colors.surface.raised,
    danger: theme.colors.danger,
  };
  const label: Record<ButtonVariant, "inverse" | "primary"> = {
    primary: "inverse",
    secondary: "primary",
    danger: "inverse",
  };

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled: off, busy }}
      disabled={off}
      style={(state) =>
        StyleSheet.flatten([
          {
            minHeight: theme.layout["touch-min"],
            paddingHorizontal: theme.space[4],
            paddingVertical: theme.space[3],
            borderRadius: theme.radius.md,
            backgroundColor: fill[variant],
            alignItems: "center",
            justifyContent: "center",
            // A press has to be visible without moving anything: a cashier's
            // thumb is already covering the button.
            opacity: off ? 0.5 : state.pressed ? 0.85 : 1,
          },
          typeof style === "function" ? style(state) : style,
        ])
      }
      {...rest}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.space[2] }}>
        {busy && <ActivityIndicator size="small" color={theme.colors["on-primary"]} />}
        <Text variant="label" tone={label[variant]}>
          {title}
        </Text>
      </View>
    </Pressable>
  );
}
