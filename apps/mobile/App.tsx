import { Text, View } from "react-native";

import { Till } from "./src/screens/Till";

const API_BASE = process.env.EXPO_PUBLIC_API_URL ?? "http://127.0.0.1:4317";

export default function App() {
  return (
    <View style={{ flex: 1 }}>
      <Till apiBase={API_BASE} />
      <View style={{ padding: 8, alignItems: "center" }}>
        <Text style={{ fontSize: 12, color: "#666" }}>pair • ticket • products • customers • more</Text>
      </View>
    </View>
  );
}
