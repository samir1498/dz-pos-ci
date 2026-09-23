// The product tile's own tests, in its own file rather than in kit.test.tsx,
// because half of what is being checked here is a pure function the till will
// sort on and the other half is the three chips it drives.
//
// The same three things the rest of the kit is held to: it renders what it
// was given, it carries nothing physically left or right, and the amount goes
// in as integer centimes and comes out spelled the way packages/shared spells
// it.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import { I18nProvider, type Lang } from "@/i18n";

import { ProductTile, stockState } from "../../src/components/ProductTile";

/** The physical utilities a component must not wear. Same list as kit.test. */
const PHYSICAL =
  /(^|\s)-?(ml|mr|pl|pr|left|right|border-l|border-r|rounded-l|rounded-r|text-left|text-right)(-|\s|$)/;

/** U+202F, the narrow no-break space packages/shared groups thousands with. */
const NARROW = "\u202f";

function noPhysicalSides(root: HTMLElement): string[] {
  return [root, ...root.querySelectorAll("*")]
    .filter((node): node is HTMLElement => node instanceof HTMLElement)
    .map((node) => node.className)
    .filter((name) => typeof name === "string" && PHYSICAL.test(name));
}

function renderIn(lang: Lang, node: React.ReactNode) {
  return render(<I18nProvider lang={lang}>{node}</I18nProvider>);
}

/**
 * `stockState(qty, threshold)` answers what the shelf says about one product,
 * in milli-units. Zero and below is a refusal whatever the threshold is, a
 * threshold of zero means the shop set none, and a quantity exactly on the
 * threshold is already low: a shop that says "warn me at five" wants the
 * warning while it still has five, not after it has four.
 */
describe("stockState", () => {
  test("is ok above a threshold the shop set", () => {
    expect(stockState(12_000, 5_000)).toBe("ok");
  });

  test("is low on the threshold and under it", () => {
    expect(stockState(5_000, 5_000)).toBe("low");
    expect(stockState(4_999, 5_000)).toBe("low");
  });

  test("is out at zero and below, whatever the threshold says", () => {
    expect(stockState(0, 5_000)).toBe("out");
    expect(stockState(0, 0)).toBe("out");
    expect(stockState(-2_000, 0)).toBe("out");
  });

  test("a threshold of zero is no threshold, not a warning at nothing", () => {
    expect(stockState(1, 0)).toBe("ok");
  });
});

/**
 * The tile itself: one button carrying a name, a price and one of the three
 * chips. The caller decides whether an empty shelf refuses the sale, so a
 * tile at zero is still pressable unless it was told otherwise.
 */
describe("ProductTile", () => {
  test("shows the name and the price the way an amount is spelled", () => {
    renderIn("fr", <ProductTile name="Farine 5 kg" priceCentimes={1_284_000} qtyMilli={12_000} lowStockAtMilli={5_000} />);
    expect(screen.getByRole("button", { name: /Farine 5 kg/ })).toBeInTheDocument();
    // textContent and not getByText: the default normaliser folds U+202F into
    // an ordinary space, which is exactly the character being asserted.
    expect(screen.getByTestId("tile-price").textContent).toBe(`12${NARROW}840,00`);
  });

  test("shows the quantity when the shelf is full", () => {
    renderIn("fr", <ProductTile name="Farine" priceCentimes={1_000} qtyMilli={12_000} lowStockAtMilli={5_000} />);
    expect(screen.getByTestId("tile-stock")).toHaveTextContent("12");
  });

  test("wears the kit's low pill under the product's own threshold", () => {
    renderIn("fr", <ProductTile name="Sucre" priceCentimes={1_000} qtyMilli={3_000} lowStockAtMilli={5_000} />);
    expect(screen.getByTestId("tile-stock")).toHaveTextContent("Stock bas");
  });

  test("says the shelf is empty at zero", () => {
    renderIn("fr", <ProductTile name="Huile" priceCentimes={1_000} qtyMilli={0} lowStockAtMilli={5_000} />);
    expect(screen.getByTestId("tile-stock")).toHaveTextContent("Rupture");
  });

  test("hands the press back and takes a refusal from the caller", async () => {
    const user = userEvent.setup();
    const chosen = vi.fn();
    const { rerender } = renderIn(
      "fr",
      <ProductTile name="Huile" priceCentimes={1_000} qtyMilli={0} lowStockAtMilli={0} onSelect={chosen} />,
    );
    await user.click(screen.getByRole("button"));
    expect(chosen).toHaveBeenCalledTimes(1);

    rerender(
      <I18nProvider lang="fr">
        <ProductTile name="Huile" priceCentimes={1_000} qtyMilli={0} lowStockAtMilli={0} onSelect={chosen} disabled />
      </I18nProvider>,
    );
    expect(screen.getByRole("button")).toBeDisabled();
  });

  test("carries nothing left or right, so it mirrors in Arabic", () => {
    const { container } = renderIn(
      "ar",
      <ProductTile name="دقيق ٥ كغ" priceCentimes={1_284_000} qtyMilli={2_000} lowStockAtMilli={5_000} />,
    );
    expect(noPhysicalSides(container)).toEqual([]);
  });
});
