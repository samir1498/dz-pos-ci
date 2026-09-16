import { Button, Text, View } from "react-native";

import { saveMode, type Mode } from "../lib/mode";

export function Onboarding({
  role,
  onPick,
}: {
  role: string;
  onPick: (mode: Mode) => void;
}) {
  const isCashier = role === "cashier";

  async function pick(mode: Mode) {
    await saveMode(mode);
    onPick(mode);
  }

  // Cashier has no choice — till only, no manager screens.
  if (isCashier) {
    return (
      <View style={{ flex: 1, padding: 16, gap: 12, justifyContent: "center" }}>
        <Text style={{ fontWeight: "600" }}>Till only</Text>
        <Text>Cashier on this phone — till, products, queue only.</Text>
        <Button title="Open till" onPress={() => pick("till")} />
      </View>
    );
  }

  return (
    <View style={{ flex: 1, padding: 16, gap: 12, justifyContent: "center" }}>
      <Text style={{ fontWeight: "600" }}>How will this phone be used?</Text>
      <Button title="Till only — shop floor" onPress={() => pick("till")} />
      <Button title="Full — owner/manager" onPress={() => pick("full")} />
      <Text style={{ fontSize: 12, color: "#666" }}>
        Till only shows the till. Full adds the owner screens. You can change this by signing out.
      </Text>
    </View>
  );
}
