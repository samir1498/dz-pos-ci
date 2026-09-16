import { describe, expect, test } from "vitest";

import { tokenFromScan } from "./pairing";

const TOKEN = "a1b2c3d4".repeat(8);

describe("reading a pairing token off a scan", () => {
  test("the bare token, which is what the desktop's QR carries today", () => {
    expect(tokenFromScan(TOKEN)).toBe(TOKEN);
  });

  test("upper case, because hexadecimal is case-insensitive", () => {
    expect(tokenFromScan(TOKEN.toUpperCase())).toBe(TOKEN.toUpperCase());
  });

  // The payload is scheduled to grow: the TLS plan puts the certificate
  // fingerprint in this same QR. A phone that only accepted the bare token
  // would start refusing to pair the day the desktop adds a field.
  test("a token wrapped in something else", () => {
    expect(tokenFromScan(`dinar://pair?shop=1&v=1&t=${TOKEN}`)).toBe(TOKEN);
    expect(tokenFromScan(JSON.stringify({ v: 1, pairing_token: TOKEN }))).toBe(TOKEN);
  });

  test("anything else is nothing, not a guess", () => {
    expect(tokenFromScan("WIFI:S:Boutique;T:WPA;P:hunter2;;")).toBeNull();
    expect(tokenFromScan("6133456789012")).toBeNull();
    // Sixty-three is not sixty-four. A truncated token would be claimed and
    // refused, which reads to the cashier as a broken QR.
    expect(tokenFromScan("a".repeat(63))).toBeNull();
  });

  test("a longer hex run is not a token with something after it", () => {
    // 65 hex characters is not a 64-character token. Without the word
    // boundaries the first 64 would be taken, claimed, and refused by the
    // server, which reads to the cashier as a broken QR.
    expect(tokenFromScan("f".repeat(65))).toBeNull();
  });
});
