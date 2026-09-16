import { describe, expect, it, vi } from "vitest";

import {
  clearDevice,
  clearSession,
  forgetCachedDevice,
  forgetCachedSession,
  loadDevice,
  loadSession,
  saveDevice,
  saveSession,
} from "./session";

// Native is absent under vitest (its import resolves, its calls throw on
// `window`), so the store behind the fallback: a Map standing in for the
// disk proves the serialization round trip, not just the warm cache.
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

const SIGNED_IN = {
  deviceToken: "d".repeat(64),
  sessionToken: "s".repeat(64),
  name: "Samir",
  role: "owner",
};

describe("session store", () => {
  it("round-trips a sign-in through the store", async () => {
    await clearSession();
    expect(await loadSession()).toBeNull();
    await saveSession(SIGNED_IN);
    forgetCachedSession();
    expect(await loadSession()).toEqual(SIGNED_IN);
  });

  it("a sign-out clears it", async () => {
    await saveSession(SIGNED_IN);
    await clearSession();
    expect(await loadSession()).toBeNull();
  });

  it("garbage on the disk signs no one in", async () => {
    // Same key as session.ts; the point is the corrupted read, not the name.
    const { default: disk } = await import(
      "@react-native-async-storage/async-storage"
    );
    await disk.setItem("dzpos:session", "not-json{");
    forgetCachedSession();
    expect(await loadSession()).toBeNull();
    await clearSession();
  });

  it("a newer sign-in replaces the older one", async () => {
    await saveSession(SIGNED_IN);
    const again = { ...SIGNED_IN, name: "Yacine" };
    await saveSession(again);
    forgetCachedSession();
    expect(await loadSession()).toEqual(again);
    await clearSession();
  });
});

// The two credentials die for different reasons, so forgetting one must not
// forget the other. Before the M6+M7 review they shared a blob, and a
// session that idled out after 15 minutes sent the cashier back to a QR
// screen only a manager could get them past.
describe("the phone and the person are forgotten separately", () => {
  it("keeps the phone paired when the person's session ends", async () => {
    await saveDevice("device-abc");
    await saveSession({
      deviceToken: "device-abc",
      sessionToken: "session-1",
      name: "Yacine",
      role: "cashier",
    });

    await clearSession();

    forgetCachedSession();
    forgetCachedDevice();
    expect(await loadSession()).toBeNull();
    expect(await loadDevice()).toBe("device-abc");
  });

  it("forgets the phone when the device itself is revoked", async () => {
    await saveDevice("device-abc");

    await clearDevice();

    forgetCachedDevice();
    expect(await loadDevice()).toBeNull();
  });

  it("answers null for a phone that has never paired", async () => {
    await clearDevice();
    forgetCachedDevice();
    expect(await loadDevice()).toBeNull();
  });
});
