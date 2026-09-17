# Maestro flows

Run on a real phone over Tailscale (`100.111.55.62` for the desktop, `EXPO_PUBLIC_API_URL` for the phone).

```sh
# on the phone (Expo Go or dev client)
maestro test apps/mobile/maestro/pair-and-sell.yaml
```

The flow assumes the desktop is in LAN mode (`bind_lan` + `mDNS` `_dzpos._tcp`, firewall banner), and the QR has been shown (`POST /pairing/qr` as owner|manager). The phone's `pair` screen would normally scan the QR; Maestro asserts the screen exists, pairs with the typed code, signs in by tapping a name and typing a PIN, rings an online sale, kills the network to queue one, retries it, revokes the phone from the desktop and signs out. The file's seven numbered steps are the list. Ticket spool is checked on the desktop: `spool/ticket-*.bin` beside the shop file.

No CI run for Maestro yet: it needs a real phone and Tailscale, not a runner.

## The demo flows

`demo-open.yaml` and `demo.yaml` are the phone scene of the demo video, run
by `just demo-phone` against the `dinar` Android emulator rather than a real
phone. The first opens Expo Go and waits on the pairing screen, off camera,
because a cold start spends about forty seconds fetching the bundle. Then
`adb shell screenrecord` starts and the second performs: pair by the typed
code, tap the name, four digits, two things in the basket, exact cash, the
change. `mint-pairing.js` mints the pairing code from inside the flow at the
moment it is typed, because a code lives sixty seconds and the flow spends
most of that getting to the field.

`scripts/demo-phone.sh` says which environment variables point it at a
Maestro and an adb on the Windows side of the WSL box, and the plan page
`the-demo-video-playwright-records-remotion-cuts` says why they are there.
