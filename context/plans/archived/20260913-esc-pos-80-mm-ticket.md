---
title: 'ESC/POS 80 mm ticket'
slug: 'esc-pos-80-mm-ticket'
status: 'active'
category: 'other'
created: 20260913
tldr: 'Thermal ticket as ESC/POS bytes with dump goldens; USB send later. No printer required to prove the bytes.'
priority: 25
tasks:
  - id: 'T1'
    desc: 'ESC/POS renderer for the 80 mm ticket plus dump goldens in fr/en/ar for reel, IFU and card. No USB.'
    status: 'done'
  - id: 'T2'
    desc: 'File and TCP senders for the ESC/POS bytes (write_ticket_escpos_to_file, send_ticket_escpos_tcp) with round-trip tests against a temp file and a local listener; no USB, no API route yet'
    status: 'done'
acceptance: []
---
# ESC/POS 80 mm ticket

M1 T6 was cancelled because there is no printer here. The live USB demo can wait. The bytes a printer would eat, and a dump a reviewer can read, still have to exist.
