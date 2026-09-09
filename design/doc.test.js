// The A4 facture decides from the totals recap whether it carries a TVA
// column, never from the lines: a réel facture whose lines are all exempt
// still shows the column (décret 05-468 art. 3), an IFU facture never does.
// regime_ifu_prints_no_tva
import { describe, expect, it } from "vitest";
import { renderA4 } from "./shared/doc.js";
import { t } from "./shared/i18n.js";
import { computeTotals } from "./shared/money.js";

const exemptLines = [
  { product: { name: "Pain", ar: "خبز" }, qtyMilli: 2000, unitPrice: 1500, lineDiscount: 0, rateBps: 0 },
];

function sale(regime) {
  return {
    lines: exemptLines,
    totals: computeTotals(exemptLines, { regime, paymentMode: "cash", stampEnabled: false }),
    number: "F-2026-0001",
    date: "2026-09-09",
    paymentMode: "cash",
  };
}

const tvaHeader = `<th class="n">${t("tva")}</th>`;

describe("A4 facture TVA column follows the recap, not the lines", () => {
  it("keeps the column on a réel facture whose lines are all exempt", () => {
    const html = renderA4(sale("reel"), "fr");
    expect(html).toContain(tvaHeader);
    expect(html).toContain(`${t("tva")} 0%`);
  });

  it("drops the column under the IFU", () => {
    const html = renderA4(sale("ifu"), "fr");
    expect(html).not.toContain(tvaHeader);
  });
});
