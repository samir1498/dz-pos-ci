# Dinar Mobile

The phone. A thin client over a shop's own core (`crates/api` on the till
computer), speaking the same HTTP contract as the desktop and never knowing
which mode the shop runs in (rule 1). Expo SDK 57, expo-router, TypeScript
only.

## Layout

```
app/                 routes; the folder is the URL
  _layout.tsx        theme → query → session → cart, then the router
  index.tsx          decides which of the three states this phone is in
  pair.tsx           trades a QR's pairing token for a device token
  sign-in.tsx        trades a PIN for a session token
  (signed-in)/       everything that needs both; the group's layout is the gate
    till.tsx
    settings.tsx
components/ui/       the design system's primitives; screens import the barrel
design/theme.tsx     the phone's end of @dzpos/design — no tokens of its own
features/till/       the till's queries, its ring/retry hook, its rows
lib/                 plain TypeScript, and the only part with unit tests
providers/           what outlives a route change
maestro/             the flow driven against a real phone and a real server
```

Two rules hold that shape together. Money is priced through `@dzpos/shared`
(`lib/basket.ts`), the same code the desktop till prices with, so the two
screens cannot drift from `crates/kernel/src/money`. And the basket lives above
the auth gate (`providers/CartProvider.tsx`), so a session that idles out
mid-sale costs a PIN, not a re-scan in front of the customer.

## Running it

The dev server runs on the machine with the phone's Wi-Fi in reach — for us
that is the laptop, not the WSL box (`.claude/skills/laptop-dev`).

```sh
REACT_NATIVE_PACKAGER_HOSTNAME=<this machine's LAN or Tailscale IP> \
EXPO_PUBLIC_API_URL=http://<till computer>:4317 \
EXPO_PUBLIC_API_TOKEN=<the launch token the desktop shows> \
pnpm --filter dinar-mobile start
```

The till computer has to be in LAN mode for anything but loopback to reach
it (`just api` with `bind_lan`), and the device gate means the phone must
pair before it can do anything else.

## Checks

`pnpm --filter dinar-mobile test` (vitest over `lib/`) and
`pnpm --filter dinar-mobile build` (`tsc --noEmit`) both ride along in
`just gates`. The screens are proven by `maestro/pair-and-sell.yaml` against
a real phone and a real shop file, because a mocked till proves nothing
about a till.
