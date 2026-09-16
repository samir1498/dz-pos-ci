// Four boxes for the four-digit PIN, the phone's copy of the desktop's
// `PinBoxes`. A real `TextInput` sits under them, one point square and
// invisible (a zero-size input raises no keyboard on Android), so the number
// pad comes up and the OS's own paste rules still hold; the boxes only draw
// how many digits are in, as dots. Tapping anywhere on the row focuses the
// input, which is what a thumb will do when the keyboard has gone away.
//
// Masked because a phone is read over a shoulder at a counter, and never
// `secureTextEntry`: that flag makes iOS offer a saved password over a PIN.
// Nor `textContentType="oneTimeCode"`: that is iOS's hook for lifting a
// code out of an incoming SMS, and a till PIN is not one.

import { PIN_DIGITS } from "@dzpos/shared";
import { useRef, useState } from "react";
import { Pressable, Text as RnText, TextInput, View } from "react-native";

import { useTheme } from "../../design/theme";
import { Text } from "./Text";

const SLOTS: readonly number[] = Array.from({ length: PIN_DIGITS }, (_, index) => index);
const BOX = 52;

export function PinBoxes({
  label,
  value,
  onChange,
  autoFocus = false,
  problem = null,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  autoFocus?: boolean;
  /** What the server, or the screen, has to say about what was typed. */
  problem?: string | null;
}) {
  const theme = useTheme();
  const input = useRef<TextInput>(null);
  const [focused, setFocused] = useState(false);

  return (
    <View style={{ gap: theme.space[1] }}>
      <Text variant="label" tone="secondary">
        {label}
      </Text>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={label}
        onPress={() => input.current?.focus()}
        style={{ flexDirection: "row", gap: theme.space[2] }}
      >
        {SLOTS.map((index) => {
          const active = focused && index === Math.min(value.length, PIN_DIGITS - 1);
          return (
            <View
              key={index}
              style={{
                width: BOX,
                height: BOX,
                borderRadius: theme.radius.sm,
                borderWidth: active ? 2 : 1,
                borderColor:
                  problem !== null
                    ? theme.colors.border.danger
                    : active
                      ? theme.colors.border.focus
                      : theme.colors.border.default,
                backgroundColor: theme.colors.surface.card,
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <RnText style={{ fontSize: theme.fontSize.xl, color: theme.colors.text.primary }}>
                {index < value.length ? "•" : ""}
              </RnText>
            </View>
          );
        })}
        <TextInput
          ref={input}
          accessibilityLabel={label}
          value={value}
          onChangeText={(typed) => onChange(typed.replace(/\D/g, "").slice(0, PIN_DIGITS))}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          keyboardType="number-pad"
          maxLength={PIN_DIGITS}
          autoComplete="off"
          textContentType="none"
          caretHidden
          autoFocus={autoFocus}
          style={{ position: "absolute", opacity: 0, width: 1, height: 1 }}
        />
      </Pressable>
      <Text variant="caption" tone={problem === null ? "tertiary" : "danger"}>
        {problem ?? " "}
      </Text>
    </View>
  );
}
