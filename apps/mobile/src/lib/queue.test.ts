import { describe, expect, it, beforeEach } from "vitest";

import { clear, enqueue, list, retry } from "./queue";

beforeEach(async () => {
  await clear();
});

describe("retry queue (M6 T5)", () => {
  it("enqueues a failed sale and retries it", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "{}" });
    expect((await list()).length).toBe(1);
    const queued = await list();
    expect(queued[0]?.id).toBeTruthy();
    expect(queued[0]?.createdAt).toBeTruthy();
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

  it("keeps order and handles partial success", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "1" });
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "2" });
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "3" });
    const calls: string[] = [];
    const { succeeded, failed } = await retry(async (req) => {
      calls.push(req.body ?? "");
      return req.body !== "2";
    });
    expect(calls).toEqual(["1", "2", "3"]);
    expect(succeeded).toBe(2);
    expect(failed).toBe(1);
    expect((await list()).map((r) => r.body)).toEqual(["2"]);
  });

  it("keeps a request whose sender throws", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "{}" });
    const { succeeded, failed } = await retry(async () => {
      throw new Error("offline");
    });
    expect(succeeded).toBe(0);
    expect(failed).toBe(1);
    expect((await list()).length).toBe(1);
  });

  it("is empty after clear", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", body: "{}" });
    await clear();
    expect((await list()).length).toBe(0);
    const { succeeded, failed } = await retry(async () => true);
    expect(succeeded).toBe(0);
    expect(failed).toBe(0);
  });
});
