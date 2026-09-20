// One supplier's statement, addressable. Anything that names a supplier links
// here: the purchase screen does, and so does every row of the list.
//
// The trailing underscore on `suppliers_` keeps this out from under the list
// screen's route rather than nesting inside it, the way the customer fiche's
// does: `/suppliers` is a whole page of its own and not a layout with an
// outlet. The fiche itself lives in `-suppliers/fiche.tsx` beside the panel
// that shows the same fiche, so the two ways in cannot drift apart.

import { Link, createFileRoute } from "@tanstack/react-router";

import { Button } from "@/components/ui/button";
import { useTranslation } from "@/i18n";

import { SupplierFiche } from "./-suppliers/fiche";

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
    <section className="flex flex-col items-start gap-4">
      <p role="alert" className="text-sm text-fg-danger">
        {t("error_not_found")}
      </p>
      <Button variant="link" asChild>
        <Link to="/suppliers">{t("action_back_to_suppliers")}</Link>
      </Button>
    </section>
  );
}
