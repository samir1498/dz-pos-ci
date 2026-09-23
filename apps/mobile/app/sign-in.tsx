// The retail router root's sign-in: lands on the till once signed in. See
// `screens/SignIn.tsx` for the shared screen and `app-clinic/sign-in.tsx`
// for the other root's landing.

import SignIn from "../screens/SignIn";

export default function RetailSignIn() {
  return <SignIn afterSignIn="/till" />;
}
