// The retail router root's index: opens on the till, the screen a retail
// build's counter spends its day on. See `screens/Index.tsx` for the shared
// three-state logic and `app-clinic/index.tsx` for the other root's home.

import Index from "../screens/Index";

export default function RetailIndex() {
  return <Index home="/till" />;
}
