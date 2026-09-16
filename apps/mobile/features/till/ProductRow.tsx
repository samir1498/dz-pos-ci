// One product, and the whole reason the cashier is holding the phone: a
// target big enough to hit without looking. The row itself is the button —
// a small "Add" on the right of a wide row is the shape that makes a cashier
// miss and ring the line above.

import { Pressable, View } from "react-native";

import { Text } from "../../components/ui";
import { useTheme } from "../../design/theme";
import { formatCentimes, type Product } from "../../lib/basket";

export function ProductRow({
  product,
  inCart,
  onAdd,
}: {
  product: Product;
  /** How many of this one are already in the basket, 0 if none. */
  inCart: number;
  onAdd: () => void;
}) {
  const theme = useTheme();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`Add ${product.name}`}
      onPress={onAdd}
      style={({ pressed }) => ({
        minHeight: theme.layout["control-h-lg"],
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        gap: theme.space[3],
        paddingHorizontal: theme.space[3],
        paddingVertical: theme.space[2],
        marginBottom: theme.space[2],
        borderRadius: theme.radius.sm,
        borderWidth: 1,
        borderColor: inCart > 0 ? theme.colors.primary : theme.colors.border.default,
        backgroundColor: pressed ? theme.colors.surface.raised : theme.colors.surface.card,
      })}
    >
      <View style={{ flex: 1, gap: theme.space[1] }}>
        <Text variant="body" numberOfLines={1}>
          {product.name}
        </Text>
        <Text variant="caption" tone="secondary">
          {formatCentimes(product.selling_centimes)} DA
        </Text>
      </View>
      {inCart > 0 && (
        <View
          style={{
            minWidth: theme.space[6],
            paddingHorizontal: theme.space[2],
            paddingVertical: theme.space[1],
            borderRadius: theme.radius.full,
            backgroundColor: theme.colors.primary,
            alignItems: "center",
          }}
        >
          <Text variant="label" tone="inverse">
            {String(inCart)}
          </Text>
        </View>
      )}
    </Pressable>
  );
}
