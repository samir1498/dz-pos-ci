import { describe, it, expect } from "vitest";
import { fmtDA, fmtDate, today, clamp, marginPct } from "@/lib/utils";

describe("fmtDA", () => {
  it("formats amounts with DA suffix", () => {
    expect(fmtDA(1500)).toMatch(/DA$/);
    expect(fmtDA(0)).toMatch(/DA$/);
    expect(fmtDA(-500)).toMatch(/DA$/);
  });

  it("includes decimal places", () => {
    expect(fmtDA(0)).toContain(",00");
    expect(fmtDA(99.5)).toContain(",50");
  });

  it("uses comma as decimal separator", () => {
    expect(fmtDA(99.5)).toContain(",");
  });
});

describe("fmtDate", () => {
  it("formats ISO date", () => {
    const result = fmtDate("2026-06-28T12:00:00");
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{2}/);
  });

  it("returns empty string for empty input", () => {
    expect(fmtDate("")).toBe("");
  });
});

describe("today", () => {
  it("returns YYYY-MM-DD format", () => {
    expect(today()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});

describe("clamp", () => {
  it("clamps to min", () => {
    expect(clamp(5, 10, 20)).toBe(10);
  });

  it("clamps to max", () => {
    expect(clamp(25, 10, 20)).toBe(20);
  });

  it("returns value within range", () => {
    expect(clamp(15, 10, 20)).toBe(15);
  });
});

describe("marginPct", () => {
  it("calculates margin percentage", () => {
    expect(marginPct(100, 150)).toBe(50);
  });

  it("returns 0 when cost is 0", () => {
    expect(marginPct(0, 100)).toBe(0);
  });

  it("returns 0 when cost is negative", () => {
    expect(marginPct(-10, 100)).toBe(0);
  });

  it("handles negative margin", () => {
    expect(marginPct(100, 80)).toBe(-20);
  });
});
