// The cash panel shows the server's figures and works none of them out. What
// is checked here is the one figure that stopped being a hard-coded zero with
// ruling 5 of the 2026-09-20 loop: the cash a reversal handed back over the
// counter. The expenses and dashboard suites next door check that each screen
// mounts the panel with the position it fetched; this checks what the panel
// does with one.

import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import type { CashPositionDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";

import { CashPanel } from "../../src/components/CashPanel";

/** One month's answer as the API sends it, every figure written here by hand.
 *
 *  The three totals deliberately disagree with the lines above them:
 *  `cash_in`'s lines come to 1 500 000 c and its total says 1 490 000 c,
 *  `cash_out`'s come to 325 000 c and its total says 350 000 c, and
 *  `cash_centimes` is neither difference. No answer the core gives looks like
 *  this, and that is the point: the totals are the core's, so a panel that
 *  quietly re-derived one from the lines beside it would pass a fixture that
 *  added up and fails this one.
 *
 *  `refunds_centimes` is 45 000 c — 450,00 DA handed back over the counter. */
const position: CashPositionDto = {
  from: "2026-09-01",
  to: "2026-09-30",
  cash_in: {
    sales_centimes: 1_200_000,
    stamp_centimes: 12_000,
    customer_payments_centimes: 300_000,
    total_centimes: 1_490_000,
  },
  cash_out: {
    refunds_centimes: 45_000,
    supplier_payments_centimes: 200_000,
    expenses_centimes: 80_000,
    total_centimes: 350_000,
  },
  cash_centimes: 1_111_000,
  card_in: {
    sales_centimes: 400_000,
    stamp_centimes: 0,
    customer_payments_centimes: 0,
    total_centimes: 400_000,
  },
};

function mount(answer: CashPositionDto = position) {
  return render(
    <I18nProvider lang="fr">
      <CashPanel position={answer} />
    </I18nProvider>,
  );
}

describe("the cash panel", () => {
  test("shows the cash a reversal handed back, straight off the answer", () => {
    mount();
    // 45 000 c written as the shop reads it.
    expect(screen.getByTestId("cash-out-refunds").textContent).toBe("450,00");
    // Under the outgoings heading and with the label the month's other
    // outgoings wear, so it reads as money that left rather than as a note.
    expect(screen.getByText(fr.cash_refunds)).toBeTruthy();
  });

  test("shows the totals the core summed rather than adding the lines up itself", () => {
    // Each of the three is the fixture's stated total and none of them is the
    // sum of the lines beside it, so a panel that added anything up fails
    // here. The thousands separator is the narrow no-break space
    // `formatCentimes` groups with, spelled as its escape.
    mount();
    expect(screen.getByTestId("cash-in-total").textContent).toBe("14 900,00");
    expect(screen.getByTestId("cash-out-total").textContent).toBe("3 500,00");
    expect(screen.getByTestId("cash-net").textContent).toBe("11 110,00");
  });

  test("a month where nothing came back shows a zero rather than an empty line", () => {
    // Zero is an answer: it says the shop handed nothing back, and a blank
    // there reads as a figure that failed to load.
    mount({ ...position, cash_out: { ...position.cash_out, refunds_centimes: 0 } });
    expect(screen.getByTestId("cash-out-refunds").textContent).toBe("0,00");
  });
});
