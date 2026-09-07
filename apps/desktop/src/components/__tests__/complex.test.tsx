import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Modal, Pagination, DataTable, FormField, type Column } from "@/components/ui";

describe("Modal", () => {
  it("renders when open", () => {
    render(<Modal open={true} onClose={() => {}} title="Hello"><p>content</p></Modal>);
    expect(screen.getByText("Hello")).toBeInTheDocument();
    expect(screen.getByText("content")).toBeInTheDocument();
  });

  it("does not render when closed", () => {
    render(<Modal open={false} onClose={() => {}} title="Hello"><p>content</p></Modal>);
    expect(screen.queryByText("Hello")).not.toBeInTheDocument();
  });

  it("renders with custom width", () => {
    const { container } = render(
      <Modal open={true} onClose={() => {}} title="X" width={500}>
        <p>hi</p>
      </Modal>
    );
    const inner = container.querySelector('[style*="max-height"]');
    expect((inner as HTMLElement).style.width).toBe("500px");
  });
});

describe("Pagination", () => {
  it("renders page info", () => {
    render(<Pagination page={2} totalPages={5} total={100} perPage={20} onPageChange={() => {}} />);
    expect(screen.getByText("21–40 / 100")).toBeInTheDocument();
  });

  it("renders null when total is 0", () => {
    const { container } = render(<Pagination page={1} totalPages={0} total={0} perPage={20} onPageChange={() => {}} />);
    expect(container.innerHTML).toBe("");
  });

  it("disables prev on first page", () => {
    render(<Pagination page={1} totalPages={5} total={100} perPage={20} onPageChange={() => {}} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toBeDisabled();
  });

  it("disables next on last page", () => {
    render(<Pagination page={5} totalPages={5} total={100} perPage={20} onPageChange={() => {}} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons[buttons.length - 1]).toBeDisabled();
  });

  it("calls onPageChange with prev page", async () => {
    const fn = vi.fn();
    render(<Pagination page={3} totalPages={5} total={100} perPage={20} onPageChange={fn} />);
    const buttons = screen.getAllByRole("button");
    await userEvent.click(buttons[0]);
    expect(fn).toHaveBeenCalledWith(2);
  });

  it("calls onPageChange with next page", async () => {
    const fn = vi.fn();
    render(<Pagination page={3} totalPages={5} total={100} perPage={20} onPageChange={fn} />);
    const buttons = screen.getAllByRole("button");
    await userEvent.click(buttons[buttons.length - 1]);
    expect(fn).toHaveBeenCalledWith(4);
  });
});

describe("DataTable", () => {
  interface Item { id: number; name: string; value: number }
  const columns: Column<Item>[] = [
    { key: "name", header: "Name", render: (i) => i.name },
    { key: "value", header: "Value", align: "right", render: (i) => i.value },
  ];
  const data: Item[] = [
    { id: 1, name: "Foo", value: 10 },
    { id: 2, name: "Bar", value: 20 },
  ];

  it("renders column headers", () => {
    render(<DataTable columns={columns} data={data} />);
    expect(screen.getByText("Name")).toBeInTheDocument();
    expect(screen.getByText("Value")).toBeInTheDocument();
  });

  it("renders data rows", () => {
    render(<DataTable columns={columns} data={data} />);
    expect(screen.getByText("Foo")).toBeInTheDocument();
    expect(screen.getByText("Bar")).toBeInTheDocument();
    expect(screen.getByText("10")).toBeInTheDocument();
    expect(screen.getByText("20")).toBeInTheDocument();
  });

  it("renders empty state", () => {
    const { container } = render(<DataTable columns={columns} data={[]} />);
    expect(container.querySelector("tbody")?.children.length ?? 0).toBe(0);
  });
});

describe("FormField", () => {
  it("renders label", () => {
    render(<FormField label="Email"><input /></FormField>);
    expect(screen.getByText("Email")).toBeInTheDocument();
  });

  it("renders children", () => {
    render(<FormField label="Name"><input data-testid="input" /></FormField>);
    expect(screen.getByTestId("input")).toBeInTheDocument();
  });
});
