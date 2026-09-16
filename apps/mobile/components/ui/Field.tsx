// A labelled text input with somewhere for a refusal to go. The message slot
// is always rendered, not conditionally mounted, so a screen does not jump
// when the server says no.

import { TextInput, View, type TextInputProps } from "react-native";

import { useTheme } from "../../design/theme";
import { Text } from "./Text";

export type FieldProps = TextInputProps & {
  label: string;
  /** What the server, or the screen, has to say about what was typed. */
  problem?: string | null;
};

export function Field({ label, problem = null, style, ...rest }: FieldProps) {
  const theme = useTheme();
  return (
    <View style={{ gap: theme.space[1] }}>
      <Text variant="label" tone="secondary">
        {label}
      </Text>
      <TextInput
        accessibilityLabel={label}
        placeholderTextColor={theme.colors.text.tertiary}
        style={[
          {
            minHeight: theme.layout["control-h"],
            paddingHorizontal: theme.space[3],
            borderRadius: theme.radius.sm,
            borderWidth: 1,
            borderColor: problem === null ? theme.colors.border.default : theme.colors.border.danger,
            backgroundColor: theme.colors.surface.card,
            color: theme.colors.text.primary,
            fontSize: theme.fontSize.md,
          },
          style,
        ]}
        {...rest}
      />
      <Text variant="caption" tone={problem === null ? "tertiary" : "danger"}>
        {problem ?? " "}
      </Text>
    </View>
  );
}
