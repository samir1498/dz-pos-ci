import { describe, expect, it, beforeEach } from "vitest";

import { clear, enqueue, list, retry } from "./queue";

beforeEach(async () => {
  await clear();
});

describe("retry queue (M6 T5)", () => {
  it("enqueues a failed sale and retries it", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "{}" });
    expect((await list()).length).toBe(1);
    const { succeeded } = await retry(async () => true);
    expect(succeeded).toBe(1);
    expect((await list()).length).toBe(0);
  });

  it("keeps a request that still fails", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "{}" });
    const { succeeded, failed } = await retry(async () => false);
    expect(succeeded).toBe(0);
    expect(failed).toBe(1);
    expect((await list()).length).toBe(1);
  });
});
