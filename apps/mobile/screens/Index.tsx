// Where the app opens: whichever of the three states this phone is in.
// It renders nothing of its own — the credentials decide, and until they
// have been read off disk it shows nothing rather than a screen the phone
// is about to be redirected away from.
//
// `home` is the one thing that differs between a retail and a clinic build
// (C7): `app/index.tsx` passes "/till", `app-clinic/index.tsx` passes
// "/settings" — the only signed-in screen a build without the till still
// carries. A plain `string`, the way the desktop switch already types a
// route's `to` (`apps/desktop/src/routes/index.tsx`): the home for a build
// this file cannot know exists in the running router root at all (the other
// root's home is a path this one's own tree may not register) would refuse
// to compile as a literal against a typed route.

import { Redirect } from "expo-router";

import { Screen } from "../components/ui";
import { useSession } from "../providers/SessionProvider";

export default function Index({ home }: { home: string }) {
  const { device, session, ready } = useSession();

  if (!ready) return <Screen />;
  if (device === null) return <Redirect href="/pair" />;
  if (session === null) return <Redirect href="/sign-in" />;
  return <Redirect href={home} />;
}
