// Every route's outermost element. It owns the safe area, the background
// token and the one gutter width, so no screen sets its own padding and they
// cannot drift apart.

import type { ReactNode } from "react";
import { ScrollView, View, type ViewStyle } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { useTheme } from "../../design/theme";

export function Screen({
  children = null,
  scroll = false,
  style,
}: {
  /** Optional: an empty frame is a real state — the gate renders one while
   * the credentials are still being read off disk. */
  children?: ReactNode;
  /** A till does not scroll — its own list does. A settings page does. */
  scroll?: boolean;
  style?: ViewStyle;
}) {
  const theme = useTheme();
  const insets = useSafeAreaInsets();

  const frame: ViewStyle = {
    flex: 1,
    backgroundColor: theme.colors.surface.bg,
    paddingTop: insets.top,
    paddingBottom: insets.bottom,
    paddingHorizontal: theme.space[4],
  };

  if (!scroll) return <View style={[frame, style]}>{children}</View>;
  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: theme.colors.surface.bg }}
      contentContainerStyle={[frame, { flexGrow: 1 }, style]}
      keyboardShouldPersistTaps="handled"
    >
      {children}
    </ScrollView>
  );
}
