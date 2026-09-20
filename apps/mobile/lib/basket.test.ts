// The phone must ask for what the shop is owed, not for the sum of the
// price tags. Under the réel régime a price tag is HT, so the two differ by
// the TVA and, past 300,00 DA, by the droit de timbre as well.
//
// Every expectation about a total is written out by hand from
// docs/features.md §3, never by calling the thing under test. The one
// exception is the round trip at the bottom, whose operands come from
// `priceBasket` on purpose: what it holds is that two functions agree on
// a string, so a hand-written amount would only test the pair on a number
// the Exact button never produces. The first case is the one the
// adversarial review of 2026-09-16 proved against a running API: the old
// screen sent 11 000 where the server owed 13 090 and was refused.

import { describe, expect, it } from "vitest";

import { formatCentimes, priceBasket, readChange, readTendered, type Product } from "./basket";

const sucre = (rateBps: number): Product => ({
  id: 1,
  name: "Sucre",
  selling_centimes: 11_000,
  rate_bps: rateBps,
});

describe("what the shop is owed", () => {
  it("is the price plus its TVA under reel, not the price tag", () => {
    const owed = priceBasket([{ product: sucre(1900), qty: 1 }], "reel").netToPay;
    // 110,00 HT + 19% = 130,90. The old screen sent 110,00 and was refused.
    expect(owed).toBe(13_090);
    expect(owed).not.toBe(11_000);
  });

  it("carries the stamp once a cash basket clears 300,00 DA", () => {
    // 40 × 110,00 = 4 400,00 TTC at 0 bps. Over 300,00 and at or under
    // 30 000,00, so 44 tranches of 100,00 at 1,00 DA = 44,00.
    const totals = priceBasket([{ product: sucre(0), qty: 40 }], "ifu");
    expect(totals.totalTtc).toBe(440_000);
    expect(totals.stamp).toBe(4_400);
    expect(totals.netToPay).toBe(444_400);
  });

  it("charges no TVA under the IFU, where the price tag is the whole price", () => {
    const totals = priceBasket([{ product: sucre(1900), qty: 1 }], "ifu");
    expect(totals.tva).toBe(0);
    expect(totals.netToPay).toBe(11_000);
  });

  it("adds each line, so two of a thing costs twice one of it", () => {
    const one = priceBasket([{ product: sucre(1900), qty: 1 }], "reel").netToPay;
    const two = priceBasket([{ product: sucre(1900), qty: 2 }], "reel").netToPay;
    expect(two).toBe(one * 2);
  });
});

describe("what the customer handed over", () => {
  it("reads dinars and centimes, with either mark", () => {
    expect(readTendered("130,90")).toBe(13_090);
    expect(readTendered("130.90")).toBe(13_090);
    expect(readTendered("200")).toBe(20_000);
    expect(readTendered(" 50,5 ")).toBe(5_050);
  });

  it("answers null for a box nobody has typed in yet", () => {
    // Blank is a cashier still counting notes, not a customer who paid zero.
    expect(readTendered("")).toBeNull();
    expect(readTendered("   ")).toBeNull();
  });

  it("answers null for something that is not an amount", () => {
    expect(readTendered("abc")).toBeNull();
    expect(readTendered("-50")).toBeNull();
    expect(readTendered("1,234")).toBeNull();
  });

  /** Both ends of the Exact button, and the second is the one that broke.
   *
   *  Showing an amount groups its thousands with a narrow no-break space
   *  (`packages/shared/src/money.ts`), so a basket over a thousand dinars
   *  goes into the box as "1 234,00". The parser this file used to have
   *  wanted `^\d+(\.\d{0,2})?$` and read that as nothing at all, which
   *  means moving the phone onto the shared formatter without also moving
   *  it onto the shared parser leaves Exact dead on exactly the baskets
   *  worth the most. The small case alone could not have said so. */
  it("round-trips what the Pay exact button puts in the box, grouped or not", () => {
    const small = priceBasket([{ product: sucre(1900), qty: 1 }], "reel").netToPay;
    expect(readTendered(formatCentimes(small))).toBe(small);

    const large = priceBasket([{ product: sucre(1900), qty: 90 }], "reel").netToPay;
    expect(large).toBeGreaterThan(100_000);
    expect(formatCentimes(large)).toContain("\u202f");
    expect(readTendered(formatCentimes(large))).toBe(large);
  });
});

describe("showing an amount", () => {
  it("keeps both centimes, padded", () => {
    expect(formatCentimes(13_090)).toBe("130,90");
    expect(formatCentimes(4_400)).toBe("44,00");
    expect(formatCentimes(5)).toBe("0,05");
    expect(formatCentimes(0)).toBe("0,00");
  });
});

describe("the change the server answered", () => {
  /** The phone's own `call<T>` hands a body back without a schema, and
   *  `formatCentimes` throws on anything that is not a safe integer, so
   *  a garbled `change_centimes` would take the till screen down in the
   *  middle of a sale with no error boundary under it. */
  it("is null for anything that is not a whole number of centimes", () => {
    expect(readChange(2500)).toBe(2500);
    expect(readChange(0)).toBe(0);
    expect(readChange(25.5)).toBeNull();
    expect(readChange("2500")).toBeNull();
    expect(readChange(null)).toBeNull();
    expect(readChange(undefined)).toBeNull();
    expect(readChange(Number.MAX_SAFE_INTEGER + 2)).toBeNull();
  });
});
