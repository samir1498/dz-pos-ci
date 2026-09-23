// The clinic router root's index. There is no clinic phone screen yet
// (C7's scope is only "ship no till"), so the floor once signed in is
// settings, the one signed-in screen this root still carries.

import Index from "../screens/Index";

export default function ClinicIndex() {
  return <Index home="/settings" />;
}
