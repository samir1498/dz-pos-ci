// One customer's fiche, addressable. Anything that names a customer links
// here: the till's credit refusal, and the documents screen when it lands.
//
// The trailing underscore on `customers_` keeps this out from under the list
// screen's route rather than nesting inside it: `/customers` is a whole page
// of its own and not a layout with an outlet, and a fiche is a whole page
// too. The fiche itself lives in `customers.tsx` beside the panel that shows
// the same two components, so the two ways in cannot drift apart.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useTranslation } from "@/i18n";
import { CustomerFiche } from "./customers";

export const Route = createFileRoute("/customers_/$id")({ component: OneCustomer });

function OneCustomer() {
  const { id } = Route.useParams();
  // A path is text, and `/customers/abc` is a link somebody mistyped rather
  // than a customer this shop does not have. `Number` on it is NaN, which
  // would go to the API as `/customers/NaN` and come back a bad request; the
  // page answers it here instead, and says the one thing there is to say.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotACustomer />;
  return <CustomerFiche id={parsed} />;
}

function NotACustomer() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <p role="alert" className="text-red-700">
        {t("error_not_found")}
      </p>
      <Link to="/customers" className="underline">
        {t("action_back_to_customers")}
      </Link>
    </section>
  );
}
