// The till's customer panel on its own: the search box, the picked fiche and
// the answers. No router and no server: the panel takes the rows the till
// already fetched as a prop, so what is under test is what it says about
// them.

import { describe, expect, test } from "vitest";
import { render, screen } from "@testing-library/react";
import type { CustomerDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import ar from "@/i18n/ar.json";
import fr from "@/i18n/fr.json";
import { CustomerPanel } from "../../../src/routes/-till/customer";

const benali: CustomerDto = {
  id: 3,
  shop_id: 1,
  name: "Épicerie Benali",
  party_kind: "company",
  phone: null,
  address: null,
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 200_000,
  warn_threshold_centimes: null,
  notes: null,
  active: true,
  balance_centimes: 100_000,
};

function panel(props: {
  search: string;
  rows: CustomerDto[];
  picked?: CustomerDto | null;
  lang?: Lang;
}) {
  return render(
    <I18nProvider lang={props.lang ?? "fr"}>
      <CustomerPanel
        picked={props.picked ?? null}
        rows={props.rows}
        search={props.search}
        onSearch={() => undefined}
        onPick={() => undefined}
        kind="ticket"
        onKind={() => undefined}
      />
    </I18nProvider>,
  );
}

// T7 (and the empty-box half of T16): "no matching customer" answers a
// search, and only one that found nobody.
describe("no matching customer", () => {
  test("is not said while the box is empty, even in a shop with no customers yet", () => {
    panel({ search: "", rows: [] });
    expect(screen.queryByText(fr.till_no_customer)).not.toBeInTheDocument();
  });

  test("is said after a search that found nobody", () => {
    panel({ search: "Zzz", rows: [] });
    expect(screen.getByText(fr.till_no_customer)).toBeInTheDocument();
  });

  test("is not said under a picked customer who is the search's only match (Arabic till)", () => {
    panel({ search: "Benali", rows: [benali], picked: benali, lang: "ar" });
    expect(screen.getByTestId("till-customer-balance")).toBeInTheDocument();
    expect(screen.queryByText(ar.till_no_customer)).not.toBeInTheDocument();
  });

  test("is said when the search matched only closed fiches, which are never offered", () => {
    panel({ search: "Benali", rows: [{ ...benali, active: false }] });
    expect(screen.getByText(fr.till_no_customer)).toBeInTheDocument();
  });
});
