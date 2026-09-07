import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Card, Button, Tag } from "@/components/ui";

describe("Card", () => {
  it("renders children", () => {
    render(<Card><p>hello</p></Card>);
    expect(screen.getByText("hello")).toBeInTheDocument();
  });

  it("applies className", () => {
    const { container } = render(<Card className="mt-4"><p>x</p></Card>);
    expect(container.firstChild).toHaveClass("mt-4");
  });

  it("has surface background style", () => {
    const { container } = render(<Card><p>x</p></Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.style.background).toBe("var(--color-surface)");
  });

  it("has border style", () => {
    const { container } = render(<Card><p>x</p></Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.style.border).toBe("1px solid var(--color-border)");
  });
});

describe("Button", () => {
  it("renders label", () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole("button", { name: /save/i })).toBeInTheDocument();
  });

  it("applies primary variant inline style", () => {
    const { container } = render(<Button variant="primary">Go</Button>);
    const btn = container.firstChild as HTMLElement;
    expect(btn.style.background).toBe("var(--color-text)");
    expect(btn.style.color).toBe("var(--color-bg)");
    expect(btn.style.border).toBe("1px solid var(--color-text)");
  });

  it("applies sm size inline style", () => {
    const { container } = render(<Button size="sm">X</Button>);
    const btn = container.firstChild as HTMLElement;
    expect(btn.style.height).toBe("26px");
    expect(btn.style.paddingInline).toBe("10px");
    expect(btn.style.fontSize).toBe("11px");
  });

  it("applies md size", () => {
    const { container } = render(<Button size="md">X</Button>);
    const btn = container.firstChild as HTMLElement;
    expect(btn.style.height).toBe("30px");
    expect(btn.style.paddingInline).toBe("14px");
    expect(btn.style.fontSize).toBe("12px");
  });

  it("calls onClick", () => {
    let clicked = false;
    render(<Button onClick={() => { clicked = true; }}>Click</Button>);
    screen.getByRole("button").click();
    expect(clicked).toBe(true);
  });

  it("disables button", () => {
    render(<Button disabled>No</Button>);
    expect(screen.getByRole("button")).toBeDisabled();
  });
});

describe("Tag", () => {
  it("renders text", () => {
    render(<Tag>Active</Tag>);
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("applies good color inline style", () => {
    const { container } = render(<Tag color="good">OK</Tag>);
    const el = container.firstChild as HTMLElement;
    expect(el.style.color).toBe("var(--color-good)");
    expect(el.style.background).toContain("var(--color-good)");
  });

  it("applies bad color inline style", () => {
    const { container } = render(<Tag color="bad">KO</Tag>);
    const el = container.firstChild as HTMLElement;
    expect(el.style.color).toBe("var(--color-bad)");
    expect(el.style.background).toContain("var(--color-bad)");
  });

  it('defaults to accent', () => {
    const { container } = render(<Tag>Default</Tag>);
    const el = container.firstChild as HTMLElement;
    expect(el.style.color).toBe("var(--color-accent)");
  });
});
