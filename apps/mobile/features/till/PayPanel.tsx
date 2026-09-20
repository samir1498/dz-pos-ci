// The bottom half of the till: what the basket comes to, what the customer
// handed over, and the one button that rings it.
//
// The amount owed is the largest thing on the screen because it is the one
// number the customer also cares about, and it is read out from the basket
// through `@dzpos/shared` — the same code the desktop till prices with. What
// is shown here is still only a preview; the sale's real amounts are the
// ones the server rings.

import { TextInput, View } from "react-native";

import { Button, Callout, Text } from "../../components/ui";
import { useTheme } from "../../design/theme";
import { formatCentimes, type CartLine } from "../../lib/basket";
import { useTranslation } from "../../providers/LanguageProvider";

export function PayPanel({
  lines,
  owed,
  tendered,
  onTenderedChange,
  onPay,
  onClear,
  busy,
  short,
  canPay,
  noRegime,
}: {
  lines: readonly CartLine[];
  /** What the shop is owed, or null while the régime has not arrived or the
   * basket is one the money rules cannot express. */
  owed: number | null;
  tendered: string;
  onTenderedChange: (typed: string) => void;
  onPay: () => void;
  onClear: () => void;
  busy: boolean;
  short: boolean;
  canPay: boolean;
  noRegime: boolean;
}) {
  const theme = useTheme();
  const { t } = useTranslation();
  const count = lines.reduce((total, line) => total + line.qty, 0);

  return (
    <View
      style={{
        gap: theme.space[3],
        paddingTop: theme.space[3],
        borderTopWidth: 1,
        borderTopColor: theme.colors.border.default,
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "flex-end", justifyContent: "space-between" }}>
        <View style={{ gap: theme.space[1], flex: 1 }}>
          <Text variant="label" tone="secondary">
            {count === 0 ? t("till_basket_empty") : t("till_basket_count", { count })}
          </Text>
          <Text variant="amount">
            {owed === null ? t("till_basket_no_total") : `${formatCentimes(owed)} ${t("currency_suffix")}`}
          </Text>
        </View>
        {count > 0 && <Button title={t("till_basket_clear")} variant="secondary" onPress={onClear} />}
      </View>

      {noRegime && <Callout tone="danger">{t("till_no_settings")}</Callout>}

      <TenderedInput value={tendered} onChangeText={onTenderedChange} />

      <View style={{ flexDirection: "row", gap: theme.space[2] }}>
        {owed !== null && (
          <Button
            title={t("till_tendered_exact")}
            variant="secondary"
            onPress={() => onTenderedChange(formatCentimes(owed))}
            style={{ flex: 1 }}
          />
        )}
        <Button
          title={t("till_pay_cash")}
          onPress={onPay}
          busy={busy}
          disabled={!canPay}
          style={{ flex: 2 }}
        />
      </View>

      {short && <Callout tone="danger">{t("till_tendered_short")}</Callout>}
    </View>
  );
}

// Split out only so the panel above reads as a layout. It is the one input
// on the screen, so it gets the large control height rather than the
// ordinary one.
function TenderedInput({
  value,
  onChangeText,
}: {
  value: string;
  onChangeText: (typed: string) => void;
}) {
  const theme = useTheme();
  const { t } = useTranslation();
  return (
    <TextInput
      accessibilityLabel={t("till_tendered_label")}
      placeholder={t("till_tendered_placeholder")}
      placeholderTextColor={theme.colors.text.tertiary}
      keyboardType="decimal-pad"
      value={value}
      onChangeText={onChangeText}
      style={{
        minHeight: theme.layout["control-h-lg"],
        paddingHorizontal: theme.space[3],
        borderRadius: theme.radius.sm,
        borderWidth: 1,
        borderColor: theme.colors.border.default,
        backgroundColor: theme.colors.surface.card,
        color: theme.colors.text.primary,
        fontSize: theme.fontSize.xl,
      }}
    />
  );
}
