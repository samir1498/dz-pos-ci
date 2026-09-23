// The retail router root's own copy of the root layout: a thin re-export so
// `expo-router`'s file discovery (which only ever looks inside whichever
// directory `app.config.js` names as the router root, C7 of
// `the-first-clinic-module-patients-queue-appointments`) finds a `_layout`
// here. The layout itself, providers and all, lives in `screens/RootLayout`
// so both roots share one copy.

export { default } from "../screens/RootLayout";
