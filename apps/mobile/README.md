# Dinar Mobile (M6 T3 scaffold)

Expo thin client, placeholder. Talks the same HTTP contract as the desktop (`packages/shared`), never knows which mode it is in (rule 1). Screens: `pair` (QR), `till`, `cart`, `pay`, `ticket`, `products`, `customers`, `more` + retry queue (T5). Printing through desktop (T6). Maestro over Tailscale (T7).

`expo` + `expo-router` + Expo MCP installed here, not before (R9). `pnpm install` from the workspace root pulls this package too (`pnpm-workspace.yaml` `apps/*`).
