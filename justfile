# dz-pos dev commands. `just` lists them; `just <recipe>` runs one.
# Rust on the WSL box lives under ~/.cargo; the PATH line covers non-login shells.

set shell := ["bash", "-euo", "pipefail", "-c"]
export PATH := env_var("HOME") + "/.cargo/bin:" + env_var("PATH")

default:
    @just --list

# ---- gates (the five in context/processes/quality-gates) ----

fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace
    pnpm -r test

build:
    pnpm -r build

# regenerate the TS types from crates/api and fail if the commit is stale.
# `git diff --exit-code` used to be the check and it ignores untracked files,
# so a brand new DTO passed the gate; it also never noticed an orphan left
# behind by a DTO that was deleted. Generating into a temp directory and
# running `diff -r` both ways catches each of those.
types-check:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -e crates/api/bindings ]; then
        echo "crates/api/bindings exists: a DTO used a bare #[ts(export)]; use export_to and the FILES list" >&2
        exit 1
    fi
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    DZPOS_TS_OUT_DIR="$tmp" cargo test -p dzpos-api --test export_bindings
    diff -r "$tmp" packages/shared/src/generated

# regenerate the committed TS types after a DTO change (the test never
# writes there on its own, so `cargo test` cannot mask a stale commit)
types:
    DZPOS_TS_OUT_DIR=packages/shared/src/generated cargo test -p dzpos-api --test export_bindings

# everything a PR needs, in order; stops at the first failure
gates: fmt clippy types-check test build

# ---- dev ----

# the API against a development database; the browser UI talks to this one.
# The API names the origins it answers (the dev Vite port and the Tauri
# ones); a browser on another machine needs its origin passed here:
# `just api 4317 .dev/dev.db http://100.111.55.62:5173`
# The API refuses any call without its launch token. `just api` makes a
# fresh one each run in .dev/api-token (gitignored, owner-only) and
# `just dev` reads it, so start the API first and restart `just dev` after
# restarting the API. Vite inlines VITE_API_TOKEN into the served bundle,
# and `--host` serves that bundle to every machine that can reach the
# port: a per-run token is what keeps that from being a lasting credential.
api port="4317" db=".dev/dev.db" origin="":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .dev
    umask 077
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > .dev/api-token
    chmod 600 .dev/api-token
    DZPOS_API_TOKEN="$(cat .dev/api-token)" cargo run -p dzpos-api -- --db {{db}} --port {{port}} {{ if origin != "" { "--allow-origin " + origin } else { "" } }}

# web UI only, reachable from the laptop over Tailscale. Needs `just api`
# running (it made the token this reads) and started with the laptop's
# origin as its third argument, or the API refuses the browser (CORS names
# its origins).
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -s .dev/api-token ]; then
        echo "no .dev/api-token: start \`just api\` first, it makes the launch token this needs" >&2
        exit 1
    fi
    VITE_API_TOKEN="$(cat .dev/api-token)" pnpm desktop dev --host

# native window; needs a display, so run on the laptop
tauri:
    pnpm desktop tauri dev

# ---- e2e (headless chromium; starts its own API and Vite) ----

# the whole products suite against a throwaway database
e2e:
    pnpm desktop e2e

# only the test that writes apps/desktop/e2e/screenshots/products.png
screenshot:
    pnpm desktop e2e -g screenshot

# ---- mockups (design/) ----

mockup-serve:
    cd design && python3 -m http.server 8766

mockup-drive base="http://127.0.0.1:8766":
    node .claude/skills/dz-mockup/scripts/drive-desktop.mjs {{base}}/desktop/ /tmp/dz-desktop.png
    node .claude/skills/dz-mockup/scripts/drive-mobile.mjs {{base}}/mobile/ /tmp/dz-mobile.png

mockup-deploy:
    pnpm dlx wrangler@4 pages deploy design --project-name dz-pos-design --commit-dirty=true

# ---- context (pc-ctx store in context/) ----

ctx *args:
    cd context && PC_CTX_RESEARCH_DIR="$PWD/../research" ctx {{args}}

# where we are: ladder, active plans
status:
    @sed -n '/## Ladder/,$p' context/progress/now.md
    @cd context && ctx status
