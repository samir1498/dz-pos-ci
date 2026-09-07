import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DataTable, StockBar, Tag, Pagination, type Column } from "@/components/ui";
import { fmtDA, clamp, marginPct } from "@/lib/utils";

describe("DataTable edge cases", () => {
  interface Item { id: number; name: string; val: number | null }
  const columns: Column<Item>[] = [
    { key: "name", header: "Name", render: (i) => i.name },
    { key: "val", header: "Val", align: "right", render: (i) => String(i.val ?? "—") },
  ];

  it("shows loading indicator", () => {
    const { container } = render(<DataTable columns={columns} data={[]} isLoading={true} />);
    expect(container.querySelector('[class*="animate-pulse"]')).toBeInTheDocument();
  });

  it("handles null values in data", () => {
    const data: Item[] = [{ id: 1, name: "Test", val: null }];
    render(<DataTable columns={columns} data={data} />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("handles large datasets without crashing", () => {
    const data: Item[] = Array.from({ length: 100 }, (_, i) => ({
      id: i, name: `Item ${i}`, val: i * 1000,
    }));
    const { container } = render(<DataTable columns={columns} data={data} />);
    expect(container.querySelectorAll("tbody tr").length).toBe(100);
  });

  it("handles single item", () => {
    render(<DataTable columns={columns} data={[{ id: 1, name: "Solo", val: 42 }]} />);
    expect(screen.getByText("Solo")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
  });
});

describe("StockBar edge cases", () => {
  it("renders at 0 quantity", () => {
    const { container } = render(<StockBar qty={0} max={50} />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("renders at negative quantity", () => {
    const { container } = render(<StockBar qty={-5} max={50} />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("clamps fill to max", () => {
    const { container } = render(<StockBar qty={100} max={50} />);
    const fill = container.querySelector('[style*="width"]') as HTMLElement;
    expect(fill.style.width).toBe("100%");
  });
});

describe("Tag edge cases", () => {
  it("renders empty children", () => {
    const { container } = render(<Tag color="accent">{""}</Tag>);
    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("Pagination edge cases", () => {
  it("shows correct range on page 1 with partial page", () => {
    render(<Pagination page={1} totalPages={2} total={25} perPage={20} onPageChange={() => {}} />);
    expect(screen.getByText("1–20 / 25")).toBeInTheDocument();
  });

  it("shows correct range on last page with remainders", () => {
    render(<Pagination page={2} totalPages={2} total={25} perPage={20} onPageChange={() => {}} />);
    expect(screen.getByText("21–25 / 25")).toBeInTheDocument();
  });
});

describe("Utils edge cases", () => {
  it("fmtDA formats zero", () => {
    expect(fmtDA(0)).toBe("0,00 DA");
  });

  it("fmtDA formats large numbers", () => {
    const result = fmtDA(1_234_567.89);
    expect(result).toMatch(/DA$/);
  });

  it("fmtDA formats negative", () => {
    expect(fmtDA(-500)).toBe("-500,00 DA");
  });

  it("clamp restrains values", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-5, 0, 10)).toBe(0);
    expect(clamp(15, 0, 10)).toBe(10);
  });

  it("clamp handles edge equalities", () => {
    expect(clamp(0, 0, 10)).toBe(0);
    expect(clamp(10, 0, 10)).toBe(10);
  });

  it("marginPct for zero cost", () => {
    expect(marginPct(0, 100)).toBe(0);
  });

  it("marginPct for zero both", () => {
    expect(marginPct(0, 0)).toBe(0);
  });

  it("marginPct for negative values", () => {
    const m = marginPct(100, 50);
    expect(m).toBeLessThan(0);
  });
});
