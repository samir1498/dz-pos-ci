// Where the UI finds the API. In the Tauri window the desktop process
// injects the port it bound; in a browser (`pnpm desktop dev`) it is the
// port `just api` uses. Same routes either way, so no screen knows which
// mode it is in (architecture.md rule 1).

import { createClient } from "@dzpos/shared";

const FALLBACK = "http://127.0.0.1:4317";

function injectedBaseUrl(): string | null {
  const global: unknown = globalThis;
  if (typeof global !== "object" || global === null) return null;
  if (!("__DZPOS_API_URL__" in global)) return null;
  const value = global.__DZPOS_API_URL__;
  return typeof value === "string" && value !== "" ? value : null;
}

export function apiBaseUrl(): string {
  const injected = injectedBaseUrl();
  if (injected !== null) return injected;
  const configured = import.meta.env.VITE_API_URL;
  return typeof configured === "string" && configured !== "" ? configured : FALLBACK;
}

export const api = createClient(apiBaseUrl());

export const productsQueryKey: readonly string[] = ["products"];
export const categoriesQueryKey: readonly string[] = ["categories"];
