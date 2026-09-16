// Where the app opens: whichever of the three states this phone is in.
// It renders nothing of its own — the credentials decide, and until they
// have been read off disk it shows nothing rather than a screen the phone
// is about to be redirected away from.

import { Redirect } from "expo-router";

import { Screen } from "../components/ui";
import { useSession } from "../providers/SessionProvider";

export default function Index() {
  const { device, session, ready } = useSession();

  if (!ready) return <Screen />;
  if (device === null) return <Redirect href="/pair" />;
  if (session === null) return <Redirect href="/sign-in" />;
  return <Redirect href="/till" />;
}
