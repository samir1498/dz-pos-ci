---
title: 'TLS on the shop LAN, and the token the phone ships with'
slug: 'tls-on-the-shop-lan-and-the-token-the-phone-ships-with'
status: 'paused'
category: 'security'
created: 20260916
tldr: 'Three credentials cross the shop Wi-Fi in plaintext, and the phone''s launch token is inlined into its JS bundle. Fix: a self-signed cert the desktop makes on first run, its fingerprint carried in the pairing QR, and the launch token moved out of the bundle into what pairing hands back. Not before the demo.'
priority: 65
tasks:
  - id: 'T1'
    desc: 'rustls on the LAN bind, self-signed cert generated on first run beside the shop file, SANs for the mDNS name and every bound address; loopback stays plain'
    status: 'pending'
  - id: 'T2'
    desc: 'the QR payload gains fp, the cert''s SHA-256; the desktop panel renders whatever the payload is, so this is server plus one field'
    status: 'pending'
  - id: 'T3'
    desc: 'phone pins the fingerprint through a native module; this ends Expo Go and forces a dev build. A mismatch refuses, never falls back to HTTP'
    status: 'pending'
  - id: 'T4'
    desc: 'launch token rides back in the pairing answer and is stored beside the device token; EXPO_PUBLIC_API_TOKEN deleted so nothing is inlined into the APK'
    status: 'pending'
acceptance:
  - 'A packet capture on the shop Wi-Fi while a sale is rung from the phone shows no credential and no amount in the clear'
  - 'A phone pointed at a server whose fingerprint does not match the one it paired against refuses and says so; it never falls back to HTTP'
  - 'grep of a release APK''s bundle finds no launch token, and a freshly installed unpaired phone can reach nothing'
---
# TLS on the shop LAN, and the token the phone ships with

Not an oversight. `docs/architecture.md:70` records the decision from M6 T1 —
"plain HTTP on a trusted shop Wi-Fi is the v1" — and names this exact
upgrade: "TLS with a self-signed fingerprint in the QR stays the candidate
for a shop that does not trust its Wi-Fi. The QR already carries `shop`+`v`,
adding `fp` later does not change the flow." This plan is that sentence
coming due, plus one thing that sentence does not cover.

## What is actually exposed

The attacker is someone already on the shop's Wi-Fi. On WPA2-PSK that is
anyone who has ever been given the password — a delivery driver, a former
employee, the café next door — because a shared key lets any client decrypt
or ARP-spoof any other.

Travelling in plaintext on every guarded call
(`apps/mobile/lib/api.ts::headersFor`):

- `Authorization: Bearer <launch token>`
- `X-Dzpos-Device: <device token>`
- `X-Dzpos-Session: <session token>`

Holding all three is holding a signed-in till. They can ring sales, read the
catalogue and the customer list, and read every response body.

Not exposed, and worth saying so the plan does not overclaim: PINs and
passwords are never stored or sent in the clear beyond the sign-in body
itself; the device token is stored server-side as a hash; a pairing token
dies after sixty seconds and one use.

## The second problem, which the architecture note does not cover

`EXPO_PUBLIC_API_TOKEN` is read at `apps/mobile/lib/api.ts:19`. Expo inlines
every `EXPO_PUBLIC_*` variable into the JavaScript bundle at build time, so
the launch token is a string sitting in the APK. Anyone who has the app has
it.

Today that costs nothing, because `just api` mints a fresh token into
`.dev/api-token` on every start and the old one dies with the process. A
shipped app cannot work that way: it would either carry one token for every
shop in the country, or need rebuilding per shop.

So the launch token has to stop being a build-time constant. The natural
place for it is the pairing answer: the phone already trades a QR for a
device token over a channel the owner physically controls, and the launch
token can ride back in the same response and be stored beside the device
token. A phone that has not paired has nothing, which is correct — an
unpaired phone has no business reaching the server at all.

## Order of work

### T1 — The desktop makes a cert and serves TLS

`rustls` behind a flag, cert and key generated on first LAN bind and kept
beside the shop file, `subjectAltName` covering the mDNS name and every
address the machine is bound to. Loopback stays plain HTTP: the desktop's
own webview talks to `127.0.0.1` and terminating TLS there buys nothing
against a process that is already inside the machine
(`docs/architecture.md:62` already says the core never terminates TLS for
the desktop).

Regeneration when the machine's addresses change, because a shop that moves
to a new router should not need a reinstall.

### T2 — The fingerprint rides in the QR

The QR's payload gains `fp`, the cert's SHA-256. The QR is an authenticated
out-of-band channel — an owner is physically showing a screen to a person
standing there — so trust-on-first-use over it needs no CA and shows no
certificate warning anywhere.

The desktop panel built in the pairing plan renders whatever the payload is,
so this is a server change plus the phone reading one more field.

### T3 — The phone pins it

React Native's `fetch` will not validate a self-signed cert and cannot be
told to pin one from JavaScript. This needs a native module, which **ends
Expo Go** and makes a development build the only way to run the app. That is
the real cost of this plan and the reason it is not scheduled before the
demo.

A pin mismatch is a hard refusal with its own message, never a silent
fallback to HTTP. A fallback would make the whole exercise decorative.

### T4 — The launch token leaves the bundle

`POST /pairing/claim` answers with the launch token beside the device token.
The phone stores both under `dzpos:device`. `EXPO_PUBLIC_API_TOKEN` is
deleted, and `apps/mobile/lib/api.ts` reads the stored one.

`DeviceTokenDto` gains a field, so `just types-check` and the schema drift
assertions in `packages/shared/src/schemas/pairing.ts` both have to be
updated with it.

## What this plan does not do

- No public CA, no Let's Encrypt, no DNS. A shop's till has no domain name
  and no inbound internet, and asking one to get either is asking it to stop
  using the product.
- No mutual TLS. The device token already identifies the phone, and a client
  certificate would be a fourth credential proving the same thing.
- Nothing about the desktop's own loopback. It is not a boundary
  (`crates/api/src/lib.rs:590` says so) and pretending otherwise would be
  theatre.

## Checked, not assumed (2026-09-16)

- `crates/api` has no TLS dependency of any kind today.
- All three credentials are headers on plain HTTP
  (`apps/mobile/lib/api.ts::headersFor`).
- `docs/architecture.md:70` is where the v1 decision and the TLS candidate
  are written down.
