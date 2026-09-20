import { describe, expect, it, beforeEach, vi } from "vitest";

import AsyncStorage from "@react-native-async-storage/async-storage";

import { clear, enqueue, forgetCachedQueue, list, newIdempotencyKey, retry } from "./queue";

// A Map standing in for the disk, the same stand-in the session store's
// tests use. Native is absent under vitest, so without it every read falls
// into the catch and the queue is only ever the in-memory copy: nothing
// here would touch the code that reads a stored blob back.
vi.mock("@react-native-async-storage/async-storage", () => {
  const disk = new Map<string, string>();
  return {
    default: {
      getItem: async (key: string) => disk.get(key) ?? null,
      setItem: async (key: string, value: string) => {
        disk.set(key, value);
      },
      removeItem: async (key: string) => {
        disk.delete(key);
      },
    },
  };
});

beforeEach(async () => {
  await clear();
  // The Map above is one disk for the whole file, and the bin for
  // unreadable entries is appended to rather than replaced, so it is
  // emptied here or the second test to drop something sees the first
  // test's entry beside its own.
  await AsyncStorage.removeItem("dzpos:retry-queue:unreadable");
});

describe("what comes back off the store", () => {
  const SOUND = {
    id: "a",
    method: "POST",
    url: "http://127.0.0.1:4317/sales",
    body: "{}",
    createdAt: "2026-09-20T09:00:00Z",
    sessionToken: null,
  };

  /** One unsent sale written by an older build should cost that sale and
   *  not the queue, so the check is per item and every field it needs is
   *  named here on its own. A record short of `url` or `body` would be
   *  posted as the string "undefined" to a path that does not exist, and
   *  the retry would keep earning the refusal this module was written to
   *  stop looping on; one short of `id` could never be removed after a
   *  successful send, so it would ring the same sale on every retry.
   *
   *  A row per field rather than one blob missing several: a blob short
   *  of three fields goes on being refused with two of the three checks
   *  deleted, and says nothing about either. */
  it.each(["id", "method", "url", "body", "createdAt"])(
    "drops an entry with no %s and keeps the rest",
    async (missing) => {
      const short: Record<string, unknown> = { ...SOUND, id: "b" };
      delete short[missing];
      await AsyncStorage.setItem("dzpos:retry-queue", JSON.stringify([SOUND, short]));
      forgetCachedQueue();

      expect(await list()).toEqual([SOUND]);
    },
  );

  /** And the bytes of the one it dropped are kept. `load` drops what it
   *  cannot resend and the next `save` writes the shorter list back, so
   *  without the bin a cashier's unsent sale leaves the phone with
   *  nothing for a support bundle to find. */
  it("keeps the bytes of an entry it dropped", async () => {
    const unreadable = { id: "b", method: "POST", createdAt: "2026-09-20T09:01:00Z" };
    await AsyncStorage.setItem("dzpos:retry-queue", JSON.stringify([unreadable]));
    forgetCachedQueue();

    await list();

    const kept: unknown = JSON.parse(
      (await AsyncStorage.getItem("dzpos:retry-queue:unreadable")) ?? "null",
    );
    expect(kept).toEqual([unreadable]);
  });

  /** The store itself refusing. Native is absent in a web preview and a
   *  read can throw there, which is what the in-memory fallback in this
   *  module's header promises to survive; the Map above means nothing in
   *  this file would notice if that promise stopped holding. */
  it("still rings and retries when the store refuses every call", async () => {
    const working = AsyncStorage.getItem;
    const writing = AsyncStorage.setItem;
    AsyncStorage.getItem = () => Promise.reject(new Error("no native module"));
    AsyncStorage.setItem = () => Promise.reject(new Error("no native module"));
    forgetCachedQueue();

    try {
      await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "{}" });
      expect((await list()).length).toBe(1);
      expect((await retry(async () => true)).succeeded).toBe(1);
    } finally {
      AsyncStorage.getItem = working;
      AsyncStorage.setItem = writing;
    }
  });

  /** A sale rung offline before the Expo SDK 57 rebuild, which is when
   *  the signer was added to the type. Its stored JSON has no
   *  `sessionToken` key at all, so a check that demanded one would erase
   *  a real unsent sale on the first read after the update, with nothing
   *  in the queue count to say it had gone. It comes back with a null
   *  signer, which is what `retry` needs to send it under whoever is
   *  signed in now. */
  it("keeps a sale queued before the signer existed, and sends it", async () => {
    await AsyncStorage.setItem(
      "dzpos:retry-queue",
      JSON.stringify([
        {
          id: "a",
          method: "POST",
          url: "http://127.0.0.1:4317/sales",
          body: "{}",
          createdAt: "2026-09-14T09:00:00Z",
        },
      ]),
    );
    forgetCachedQueue();

    expect((await list())[0]?.sessionToken).toBeNull();
    const { succeeded, skipped } = await retry(async () => true, "s".repeat(64));
    expect({ succeeded, skipped }).toEqual({ succeeded: 1, skipped: 0 });
  });

  // A store holding something that is not a list at all had a test here
  // and does not any more: `load`'s catch answers an empty queue for
  // anything that throws on the way out, so deleting the `Array.isArray`
  // check in front of it leaves every assertion green. The check stays
  // because it says what is expected; a test of it could not fail.
});

describe("retry queue (M6 T5)", () => {
  it("enqueues a failed sale and retries it", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "{}" });
    expect((await list()).length).toBe(1);
    const queued = await list();
    expect(queued[0]?.id).toBeTruthy();
    expect(queued[0]?.createdAt).toBeTruthy();
    const { succeeded } = await retry(async () => true);
    expect(succeeded).toBe(1);
    expect((await list()).length).toBe(0);
  });

  it("keeps a request that still fails", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "{}" });
    const { succeeded, failed } = await retry(async () => false);
    expect(succeeded).toBe(0);
    expect(failed).toBe(1);
    expect((await list()).length).toBe(1);
  });

  it("keeps order and handles partial success", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "1" });
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "2" });
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "3" });
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
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "{}" });
    const { succeeded, failed } = await retry(async () => {
      throw new Error("offline");
    });
    expect(succeeded).toBe(0);
    expect(failed).toBe(1);
    expect((await list()).length).toBe(1);
  });

  it("mints distinct retry keys", async () => {
    expect(newIdempotencyKey()).not.toBe(newIdempotencyKey());
  });

  it("skips another signer's sale instead of ringing it as you", async () => {
    await enqueue({
      method: "POST",
      url: "http://127.0.0.1:4317/sales",
      sessionToken: "anna",
      body: "{}",
    });
    await enqueue({
      method: "POST",
      url: "http://127.0.0.1:4317/sales",
      sessionToken: null,
      body: "{}",
    });
    let calls = 0;
    const { succeeded, failed, skipped } = await retry(
      async () => {
        calls += 1;
        return true;
      },
      "yacine",
    );
    expect(calls).toBe(1);
    expect(succeeded).toBe(1);
    expect(failed).toBe(0);
    expect(skipped).toBe(1);
    expect((await list()).length).toBe(1);
    // Her own retry sends it.
    const again = await retry(async () => true, "anna");
    expect(again).toEqual({ succeeded: 1, failed: 0, skipped: 0 });
    expect((await list()).length).toBe(0);
  });

  it("is empty after clear", async () => {
    await enqueue({ method: "POST", url: "http://127.0.0.1:4317/sales", sessionToken: null, body: "{}" });
    await clear();
    expect((await list()).length).toBe(0);
    const { succeeded, failed } = await retry(async () => true);
    expect(succeeded).toBe(0);
    expect(failed).toBe(0);
  });
});
