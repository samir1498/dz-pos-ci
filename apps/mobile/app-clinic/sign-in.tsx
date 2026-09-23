// The clinic router root's sign-in: no till to land on, so settings is the
// floor. See `screens/SignIn.tsx` for the shared screen and
// `app/sign-in.tsx` for the retail root's landing.

import SignIn from "../screens/SignIn";

export default function ClinicSignIn() {
  return <SignIn afterSignIn="/settings" />;
}
