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
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    DZPOS_TS_OUT_DIR="$tmp" cargo test -p dzpos-api --test export_bindings
    diff -r "$tmp" packages/shared/src/generated

# everything a PR needs, in order; stops at the first failure
gates: fmt clippy types-check test build

# ---- dev ----

# the API against a development database; the browser UI talks to this one
api port="4317" db=".dev/dev.db":
    cargo run -p dzpos-api -- --db {{db}} --port {{port}}

# web UI only, reachable from the laptop over Tailscale. Needs `just api`.
dev:
    pnpm desktop dev --host

# native window; needs a display, so run on the laptop
tauri:
    pnpm desktop tauri dev

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
