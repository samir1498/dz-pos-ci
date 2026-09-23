// What a drop does to the waiting room's order (C6b), without a pointer.

import { describe, expect, test } from "vitest";

import { inOrder, moveEntry } from "../../../src/routes/-queue/order";

const ids = ["a", "b", "c", "d"];

describe("moveEntry", () => {
  test("dropped on a later row, the entry lands after it; on an earlier one, before it", () => {
    expect(moveEntry(ids, "a", "c")).toEqual(["b", "c", "a", "d"]);
    expect(moveEntry(ids, "d", "b")).toEqual(["a", "d", "b", "c"]);
    expect(moveEntry(ids, "c", "d")).toEqual(["a", "b", "d", "c"]);
  });

  test("the same list back, the very array, when nothing moves", () => {
    expect(moveEntry(ids, "b", "b")).toBe(ids);
    expect(moveEntry(ids, "x", "b")).toBe(ids);
    expect(moveEntry(ids, "b", "x")).toBe(ids);
  });
});

describe("inOrder", () => {
  const entries = [
    { id: "a", position: 1 },
    { id: "b", position: 2 },
    { id: "c", position: 3 },
  ];

  test("puts the listed entries first in the order given and numbers every place again", () => {
    expect(inOrder(entries, ["c", "a", "b"])).toEqual([
      { id: "c", position: 1 },
      { id: "a", position: 2 },
      { id: "b", position: 3 },
    ]);
  });

  test("an entry the list leaves out follows in the order it had, an unknown id is ignored", () => {
    expect(inOrder(entries, ["c", "x"])).toEqual([
      { id: "c", position: 1 },
      { id: "a", position: 2 },
      { id: "b", position: 3 },
    ]);
  });
});
