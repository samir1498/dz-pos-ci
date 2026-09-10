// One supplier's fiche, addressable. Anything that names a supplier links
// here: the purchase screen when it lands (T3).
//
// The trailing underscore on `suppliers_` keeps this out from under the list
// screen's route rather than nesting inside it, the way the customer fiche's
// does: `/suppliers` is a whole page of its own and not a layout with an
// outlet. The fiche itself lives in `suppliers.tsx` beside the panel that
// shows the same two components, so the two ways in cannot drift apart.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useTranslation } from "@/i18n";
import { SupplierFiche } from "./suppliers";

export const Route = createFileRoute("/suppliers_/$id")({ component: OneSupplier });

function OneSupplier() {
  const { id } = Route.useParams();
  // A path is text, and `/suppliers/abc` is a link somebody mistyped rather
  // than a supplier this shop does not have. `Number` on it is NaN, which
  // would go to the API as `/suppliers/NaN` and come back a bad request; the
  // page answers it here instead.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotASupplier />;
  return <SupplierFiche id={parsed} />;
}

function NotASupplier() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <p role="alert" className="text-red-700">
        {t("error_not_found")}
      </p>
      <Link to="/suppliers" className="underline">
        {t("action_back_to_suppliers")}
      </Link>
    </section>
  );
}
