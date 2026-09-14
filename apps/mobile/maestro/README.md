# Maestro flows (M6 T7)

Run on a real phone over Tailscale (`100.111.55.62` for the desktop, `EXPO_PUBLIC_API_URL` for the phone).

```sh
# on the phone (Expo Go or dev client)
maestro test apps/mobile/maestro/pair-and-sell.yaml
```

The flow assumes the desktop is in LAN mode (`bind_lan` + `mDNS` `_dzpos._tcp`, firewall banner), and the QR has been shown (`POST /pairing/qr` as owner|manager). The phone's `pair` screen would normally scan the QR; Maestro asserts the screen exists and then drives `till` → `Add` → `Pay` → `Queued: 0`. Ticket spool is checked on the desktop: `spool/ticket-*.bin` beside the shop file.

No CI run for Maestro yet: it needs a real phone and Tailscale, not a runner.
