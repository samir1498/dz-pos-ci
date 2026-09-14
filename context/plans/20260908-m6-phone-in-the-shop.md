---
title: 'M6 phone in the shop'
slug: 'm6-phone-in-the-shop'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'LAN mode, QR pairing, Expo thin client; stub'
priority: 10
tasks:
  - id: 'T1'
    desc: 'LAN mode: desktop binds to LAN (not just 127.0.0.1), mDNS discovery (Bonjour/Avahi), one desktop serves, Windows Firewall banner, TLS decision (http on trusted Wi-Fi vs self-signed + fingerprint in QR) written to docs/architecture.md'
    status: 'pending'
  - id: 'T2'
    desc: 'QR pairing: 60s single-use short-lived token, stored with expiry, owner|manager may show QR, token is single-use and revoked after pair'
    status: 'pending'
  - id: 'T3'
    desc: 'Expo thin client scaffold: apps/mobile from placeholder, navigation (pair, till, cart, pay, ticket, products, customers), packages/shared types, Expo plugin + MCP installed here'
    status: 'pending'
  - id: 'T4'
    desc: 'Paired devices: settings lists paired devices with revoke, ManageUsers gate for QR, audit row for pair/revoke'
    status: 'pending'
  - id: 'T5'
    desc: 'Thin client till: cart, pay, ticket via API (Sell, SeeCost redaction reused), retry queue for offline LAN'
    status: 'pending'
  - id: 'T6'
    desc: 'Print through desktop: phone POST /sales/{id}/ticket/escpos proxied, desktop spools to file / TCP 9100 (write_ticket_escpos_to_file / send_ticket_escpos_tcp)'
    status: 'pending'
  - id: 'T7'
    desc: 'Maestro flows on real phone over Tailscale: pair, sell from shop floor, ticket prints on desktop, verified on fedora laptop via 100.111.55.62'
    status: 'pending'
  - id: 'T8'
    desc: 'Password-reset follow-up: POST /users/{id}/password (ManageUsers), mirrors set_pin, ends other sessions, audit row — pulled into M6 as T8 or separate if you prefer'
    status: 'pending'
  - id: 'T9'
    desc: 'Closing sweep: docs/features.md §6, docs/architecture.md, docs/roadmap.md M6 demo, dz-review (roles+money+fixture) before checkpoint PR'
    status: 'pending'
acceptance: []
---
# M6: the phone in the shop

Stub. Tasks get added when M5 closes. The milestone text is
`docs/roadmap.md` § M6.

Demo that closes it: a phone pairs by QR, sells from the shop floor, and
the ticket prints on the desktop.

In: LAN mode with one desktop serving; mDNS, QR and a short-lived token;
the Windows Firewall banner; the Expo thin client (pair, till, cart, pay,
ticket, products, customers, more) with its retry queue; printing through
the desktop; Maestro flows on a real phone over Tailscale. The Expo plugin
and MCP trial (R9) start here.
