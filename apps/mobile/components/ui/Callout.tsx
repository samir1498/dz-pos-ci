// What the server said, where the cashier can see it. A refusal is not a
// toast that disappears before it is read: it stays until the next attempt
// replaces it.

import { View } from "react-native";

import { useTheme } from "../../design/theme";
import { Text } from "./Text";

export type CalloutTone = "danger" | "success" | "info";

export function Callout({ tone, children }: { tone: CalloutTone; children: string }) {
  const theme = useTheme();
  const edge: Record<CalloutTone, string> = {
    danger: theme.colors.danger,
    success: theme.colors.success,
    info: theme.colors.info,
  };
  return (
    <View
      accessibilityRole="alert"
      style={{
        padding: theme.space[3],
        borderRadius: theme.radius.sm,
        backgroundColor: theme.colors.surface.raised,
        // A bar down the leading edge rather than a tinted block: it reads at
        // a glance and survives a dark theme without a second colour.
        borderStartWidth: 3,
        borderStartColor: edge[tone],
      }}
    >
      <Text variant="body" tone={tone === "danger" ? "danger" : "primary"}>
        {children}
      </Text>
    </View>
  );
}
