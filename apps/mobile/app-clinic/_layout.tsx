// The clinic router root's own copy of the root layout, byte-for-byte the
// same re-export as `app/_layout.tsx` (C7 of
// `the-first-clinic-module-patients-queue-appointments`). See
// `screens/RootLayout.tsx` for why the cart still mounts here: it holds no
// money math and nothing this build's screens reach for.

export { default } from "../screens/RootLayout";
