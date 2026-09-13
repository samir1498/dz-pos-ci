---
title: 'Public-ready releases and landing downloads'
slug: 'public-ready-releases-and-landing-downloads'
status: 'active'
category: 'other'
created: 20260913
tldr: 'All three installers built on the mirror, published as org releases (private until the cert), landing download section with OS detect'
tasks:
  - id: 'T1'
    desc: 'Release matrix Windows+macOS+Linux in release.yml (runs on the mirror on v* tags); update release-gate script, its tests and docs for three artifacts'
    status: 'done'
  - id: 'T2'
    desc: 'Publish job creates the release on Dinar-dz/dz-pos via a PAT in mirror secrets (zero org minutes, API call only); unsigned stays draft until the cert lands. Blocked on Samir creating ORG_RELEASE_TOKEN (contents:write on the org repo)'
    status: 'done'
  - id: 'T3'
    desc: 'just release vX recipe: tag main, push tag to origin and mirror (mirror tag run is the build trigger)'
    status: 'done'
  - id: 'T4'
    desc: 'Landing Download section in fr/en/ar with client OS detect (Windows/macOS/Linux buttons, all three always listed), wired to org releases/latest URLs; honest unsigned note until the cert'
    status: 'done'
  - id: 'T5'
    desc: 'Prove it: workflow_dispatch dry-run builds all three installers as artifacts on the mirror, then docs + light review + merge'
    status: 'done'
acceptance: []
---
# Public-ready releases and landing downloads

## Goal

TODO: define goal

## Scope

TODO: define scope
