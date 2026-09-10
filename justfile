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
# Absolute: cargo runs a test from the crate's own directory, and a relative
# path here wrote crates/api/packages/shared/src/generated the first time a
# DTO was added after the recipe was written.
types:
    DZPOS_TS_OUT_DIR="{{justfile_directory()}}/packages/shared/src/generated" cargo test -p dzpos-api --test export_bindings

# regenerate apps/desktop/src/theme.css from the token source. The check
# that a stale file fails the gates is a vitest in packages/design, so it
# rides along in `just test`; this is the `just types` beside it.
theme:
    pnpm --filter @dzpos/design gen:theme

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

# the whole suite, once per UI language project (fr, en, ar). Each
# `playwright test` invocation starts its own API and Vite pair and its
# webServer command deletes the db file first, so three invocations give
# three empty databases; three projects in one invocation would share the
# one database the first invocation starts, and the products suite's
# first test needs an empty table. Run one language with
# `pnpm desktop e2e --project ar`.
e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    for project in fr en ar; do
        echo "=== e2e: $project ==="
        pnpm desktop e2e --project "$project"
    done

# only the tests that write a committed screenshot: fr (products.png) and
# ar (the twelve *-ar.png the e2e README lists); en keeps none.
screenshot:
    #!/usr/bin/env bash
    set -euo pipefail
    pnpm desktop e2e -g screenshot --project fr
    pnpm desktop e2e -g screenshot --project ar

# ---- worktrees (one per task when the milestone loop runs tasks in parallel) ----

# a checkout of <branch> under .claude/worktrees/<name> with its own
# node_modules and its own cargo target (a shared target dir rebuilds
# everything on every switch between checkouts, so each keeps its own).
# The e2e ports are per worktree: pass DZPOS_E2E_API_PORT and
# DZPOS_E2E_WEB_PORT when running `just e2e` there (4319/5174 are the main
# checkout's).
worktree name branch:
    #!/usr/bin/env bash
    set -euo pipefail
    dir=".claude/worktrees/{{name}}"
    if [ -e "$dir" ]; then echo "$dir exists" >&2; exit 1; fi
    git worktree add -b "{{branch}}" "$dir" HEAD
    (cd "$dir" && pnpm install --frozen-lockfile --silent)
    echo "worktree $dir on {{branch}}; run cargo there with CARGO_TARGET_DIR=$dir/target"

# remove a worktree once its branch is merged.
# target/ goes first: it is gitignored, so `git worktree remove` refuses to
# touch it and the whole tree gets left behind as an orphan (that is how t5
# survived with 268K of build output and no entry in `git worktree list`).
# A worktree's target/ was 36 GB on 2026-09-10 - see `just disk`.
worktree-rm name:
    #!/usr/bin/env bash
    set -euo pipefail
    dir=".claude/worktrees/{{name}}"
    [ -d "$dir/target" ] && rm -rf "$dir/target"
    git worktree remove "$dir"
    git worktree prune

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


# ---- disk (context/processes/machines-and-heavy-jobs) ----

# What the disk really looks like. `df -h /` lies: / is a VHDX living on the
# Windows C: drive, so it reports the guest's virtual size, not the host space
# it still has room to grow into. /mnt/c is the number that matters.
disk:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "host disk (the one that matters):"
    df -h /mnt/c | tail -1
    echo
    echo "largest build artifacts:"
    { du -xsh target .claude/worktrees/*/target 2>/dev/null || true; } | sort -rh | grep . || echo "  none"
    echo
    free=$(df --output=avail -BG /mnt/c | tail -1 | tr -dc '0-9')
    if [ "${free:-0}" -lt 5 ]; then
        echo "STOP: ${free}G free on C:. Deleting inside the distro does not reach Windows;" >&2
        echo "Samir must run C:\\Users\\Anwender\\compact-wsl.ps1 as admin." >&2
        exit 1
    elif [ "${free:-0}" -lt 20 ]; then
        echo "WARNING: ${free}G free on C:. Run 'just clean-targets' before any heavy build." >&2
    else
        echo "OK: ${free}G free on C:."
    fi

# Delete every Rust target/ (main checkout + each worktree) and prune orphaned
# worktree entries. Only target/ is removed: worktrees carry uncommitted work,
# so never rm -rf the tree itself to reclaim space.
clean-targets:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for t in target .claude/worktrees/*/target; do
        [ -d "$t" ] || continue
        echo "removing $t ($(du -xsh "$t" | cut -f1))"
        rm -rf "$t"
    done
    git worktree prune
    echo "host disk now:"
    df -h /mnt/c | tail -1
