// Where a spec that seeds its own rows finds the API. The browser reaches
// it through the app, which was given VITE_API_URL; a `request` call has to
// be told, and the port is the same one playwright.config.ts binds, from the
// same environment variable with the same fallback (4319, the main
// checkout's; a worktree passes its own pair).
//
// The launch token comes from the same place, the variable the config mints
// it into. It is not put on `use.extraHTTPHeaders`: that header goes on the
// browser context too, where it overrides the one the app itself sends, and
// every screen then reads 401.

const FALLBACK_API_PORT = 4319;

export function apiUrl(): string {
  const raw = process.env.DZPOS_E2E_API_PORT;
  const port = raw === undefined || raw === "" ? FALLBACK_API_PORT : Number(raw);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) {
    throw new Error(`DZPOS_E2E_API_PORT must be a port between 1024 and 65535, got ${raw}`);
  }
  return `http://127.0.0.1:${port}`;
}

/** The bearer every direct API call has to show, as one header object. */
export function apiHeaders(): Record<string, string> {
  const token = process.env.DZPOS_E2E_TOKEN;
  if (token === undefined || token === "") {
    throw new Error("DZPOS_E2E_TOKEN is unset: playwright.config.ts mints it, so the config never loaded");
  }
  return { authorization: `Bearer ${token}` };
}
