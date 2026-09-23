// The auth gate.
//
// It guards by being a layout rather than by a check inside each screen:
// anything under `(signed-in)/` is unreachable without both credentials,
// and a route added next year inherits that without anyone remembering to
// add a guard to it. The group's parentheses keep it out of the URL, so a
// screen behind it keeps its own path — `/till` in a retail build, `/settings`
// whatever the build.
//
// The gate only decides what renders. It never asks the server whether a
// token is still good — the server answers that on the next call, and
// `outcome.ts` turns a 401 into a redirect. A phone that guesses would sign
// a cashier out on a dropped packet.
//
// Shared across both router roots (C7): `app/(signed-in)/_layout.tsx` and
// `app-clinic/(signed-in)/_layout.tsx` both re-export this file unchanged,
// since the gate itself has no opinion on which trade sits behind it.

import { Redirect, Stack } from "expo-router";

import { Screen } from "../../components/ui";
import { useSession } from "../../providers/SessionProvider";

export default function SignedInLayout() {
  const { device, session, ready } = useSession();

  // Until both have been read off disk, render the empty frame rather than a
  // redirect: a phone that is signed in would otherwise flash the pairing
  // screen on every cold start.
  if (!ready) return <Screen />;
  if (device === null) return <Redirect href="/pair" />;
  if (session === null) return <Redirect href="/sign-in" />;

  return <Stack screenOptions={{ headerShown: false }} />;
}
