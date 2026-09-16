// The entry `package.json`'s `main` field points at. Expo dropped
// `expo/AppEntry.js` in SDK 50, so an app that is not using expo-router
// registers its own root component here; this is the SDK 52 shape.
//
// It was missing entirely until 2026-09-16. `main` said `index.js`, no such
// file existed, and `expo start` answered "Cannot resolve entry file" to
// anyone who tried — which is the plainest possible proof that the phone
// app had never once been launched, however much of it was written.
import { registerRootComponent } from "expo";

import App from "./App";

// Sets up the app as the root component whether it is running in Expo Go
// or in a build of its own.
registerRootComponent(App);
