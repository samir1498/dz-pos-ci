// Reading a pairing token out of whatever the camera saw.
//
// The desktop's QR carries the token on its own today, so this could have
// been `data` used as-is. It is a pattern instead because the payload is
// already scheduled to grow: the TLS plan adds the server's certificate
// fingerprint to the same QR, and `docs/architecture.md` notes it already
// carries `shop` and `v` in the shape it was designed for. A phone that took
// the whole string would start failing at a till the day the desktop adds a
// field, which is the worst possible moment to find out.
//
// Case-insensitive because hexadecimal is, and a QR generator that emits
// upper case is not the cashier's problem.

// The word boundaries are load-bearing. Without them a run of 65 hex
// characters yields its first 64, and the phone claims a token nobody
// minted — the server refuses it, and the cashier reads that as a broken
// QR rather than as a bad scan.
const TOKEN = /\b[0-9a-f]{64}\b/i;

/** The 64-hex pairing token inside a scanned payload, or null when there is
 *  none — which is what a cashier scanning a product barcode, a Wi-Fi QR or
 *  somebody's business card will produce. */
export function tokenFromScan(data: string): string | null {
  const found = TOKEN.exec(data);
  return found === null ? null : found[0];
}
