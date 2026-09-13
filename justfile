# dz-pos dev commands. `just` lists them; `just <recipe>` runs one.
# Rust on the WSL box lives under ~/.cargo; the PATH line covers non-login shells.

set shell := ["bash", "-euo", "pipefail", "-c"]
export PATH := env_var("HOME") + "/.cargo/bin:" + env_var("PATH")

# Every cargo command builds into one shared folder for every checkout of
# this repo: a target/ per worktree is what filled the disk on 2026-09-10.
# The folder sits next to the main checkout's .git, so every worktree and
# the laptop clone find the same one without an env var (set
# CARGO_TARGET_DIR yourself to override). Four build jobs: the lock below
# makes it one cargo run at a time across worktrees, and it was several
# builds at once, not one build with four jobs, that starved the box. The e2e runner reads
# both variables too.
export CARGO_TARGET_DIR := env_var_or_default("CARGO_TARGET_DIR", `dirname "$(git rev-parse --path-format=absolute --git-common-dir)"` + "/.cargo-target")
export CARGO_BUILD_JOBS := env_var_or_default("CARGO_BUILD_JOBS", "4")

# Cargo names an artifact of our own crates the same in every worktree and
# decides freshness by mtime, so after a build in another checkout the
# shared folder holds that checkout's dzpos-core, and a checkout whose
# sources are older reuses it without a word (2026-09-10: clippy in one
# worktree failed on a type only the other branch had). This records which
# checkout built last and, when it changes, touches this checkout's crate
# sources so cargo rebuilds the workspace's own members; the dependencies stay
# cached, they are identical in every checkout. Every cargo recipe below
# depends on it; a bare `cargo` needs both `just claim` run first and
# `CARGO_TARGET_DIR` exported into the same shell (`just claim` alone does
# not export it there: the export only lives inside a recipe's own
# subprocess).
#
# The recipes that compile and then run something hold a lock on the
# folder for the whole run: cargo's own lock only covers compilation, so
# while one worktree's `cargo test` was running its binaries another
# worktree's build replaced the rlib the doc-tests were about to link
# ("extern location for dzpos_core does not exist", 2026-09-10). One cargo
# invocation at a time across every checkout; the second one waits.
claim:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$CARGO_TARGET_DIR"
    me="{{justfile_directory()}}"
    marker="$CARGO_TARGET_DIR/.owner"
    owner="$(cat "$marker" 2>/dev/null || true)"
    if [ "$owner" != "$me" ]; then
        echo "claim: build folder last used by ${owner:-nobody}; touching this checkout's crates so cargo rebuilds them" >&2
        find crates apps/desktop/src-tauri -name target -prune -o -type f -print0 | xargs -0 touch
        printf '%s\n' "$me" > "$marker"
    fi

default:
    @just --list

# ---- gates (the six in context/processes/quality-gates) ----

fmt:
    cargo fmt --all --check

# `dzpos-desktop`'s `generate_context!()` embeds `apps/desktop/dist/index.html`
# at compile time (M5 T0's `custom-protocol` dev-dependency feature made a
# plain `cargo test`/`cargo clippy` exercise the real asset-embedding path
# instead of Tauri's dev bypass, which is the point of that feature, but it
# means the crate no longer compiles with no `dist/` at all). A fresh
# checkout has none yet: `gates` only produces one in the `build` step,
# which runs after `clippy` and `test`. Both need it built first now.
desktop-dist:
    pnpm --filter dzpos-desktop build

clippy: claim desktop-dist
    flock "$CARGO_TARGET_DIR/.lock" cargo clippy --workspace --all-targets -- -D warnings

# the desktop's one eslint rule: no bare input, button, select, textarea or
# table outside components/ui and the kit. The screens written before the kit
# are exempted by name in apps/desktop/src/lint/allowlist.json, and the
# vitest beside it refuses an entry whose file has nothing left to fix, so
# the list can only shrink. No cargo, so it runs early and cheap.
lint:
    pnpm --filter dzpos-desktop lint

test: claim desktop-dist
    flock "$CARGO_TARGET_DIR/.lock" cargo test --workspace
    pnpm -r test

build:
    pnpm -r build

# regenerate the TS types from crates/api and fail if the commit is stale.
# `git diff --exit-code` used to be the check and it ignores untracked files,
# so a brand new DTO passed the gate; it also never noticed an orphan left
# behind by a DTO that was deleted. Generating into a temp directory and
# running `diff -r` both ways catches each of those.
types-check: claim
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -e crates/api/bindings ]; then
        echo "crates/api/bindings exists: a DTO used a bare #[ts(export)]; use export_to and the FILES list" >&2
        exit 1
    fi
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    DZPOS_TS_OUT_DIR="$tmp" flock "$CARGO_TARGET_DIR/.lock" cargo test -p dzpos-api --test export_bindings
    diff -r "$tmp" packages/shared/src/generated

# regenerate the committed TS types after a DTO change (the test never
# writes there on its own, so `cargo test` cannot mask a stale commit)
# Absolute: cargo runs a test from the crate's own directory, and a relative
# path here wrote crates/api/packages/shared/src/generated the first time a
# DTO was added after the recipe was written.
types: claim
    DZPOS_TS_OUT_DIR="{{justfile_directory()}}/packages/shared/src/generated" flock "$CARGO_TARGET_DIR/.lock" cargo test -p dzpos-api --test export_bindings

# regenerate apps/desktop/src/theme.css from the token source. The check
# that a stale file fails the gates is a vitest in packages/design, so it
# rides along in `just test`; this is the `just types` beside it.
theme:
    pnpm --filter @dzpos/design gen:theme

# everything a PR needs, in order; stops at the first failure
gates: fmt lint clippy types-check test build

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
api port="4317" db=".dev/dev.db" origin="": claim
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .dev
    umask 077
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > .dev/api-token
    chmod 600 .dev/api-token
    DZPOS_API_TOKEN="$(cat .dev/api-token)" cargo run -p dzpos-api -- --db {{db}} --port {{port}} {{ if origin != "" { "--allow-origin " + origin } else { "" } }}

# ---- the development shop file (.dev/dev.db) ----
#
# Dev only, and enforced in three places, not one: `dzpos-seed` is its own
# crate that neither the API nor the desktop depends on, so no release build
# can produce it (crates/api/tests/no_seed_entrypoint.rs holds that); the
# binary refuses to run without DZPOS_DEV=1, refuses any file that is not
# directly inside .dev/, and refuses one whose settings carry a real shop's
# name and identifiers; and these two recipes take no path at all.

# fill .dev/dev.db with a catalogue, twelve customers, five suppliers and
# thirty days of trading, so the dashboard, the statements and the exports
# have something to show. Deterministic: the same file on every machine, and
# repeatable, because it deletes the file first and fills a fresh one.
seed: claim
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .dev
    if [ -e .dev/dev.db ] && command -v fuser >/dev/null 2>&1 && fuser .dev/dev.db >/dev/null 2>&1; then
        echo ".dev/dev.db is open in another process (the dev API?); stop it first" >&2
        exit 1
    fi
    # The binary deletes the file itself, so the run is repeatable; the guard
    # above is here rather than in `just seed-clean` because re-entering just
    # from a recipe body runs whatever else the recipe list has grown.
    # Under the same lock every other cargo recipe takes: this one compiles,
    # and a build in another checkout pulling the rlib out from under it is
    # what the lock exists for.
    DZPOS_DEV=1 flock "$CARGO_TARGET_DIR/.lock" cargo run -p dzpos-seed --bin dzpos-seed -- --db .dev/dev.db

# delete .dev/dev.db so the next `just api` starts an empty shop.
#
# A whole file and never a row. The ledgers are append only and the document
# series are gapless by rule, so there is no honest way to take a seeded sale
# back out of a shop from the inside: cleaning up a development file is `rm`.
# Only ever .dev/dev.db, and it takes no argument, so no path a caller typed
# can reach a real shop's database.
seed-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .dev
    # Unlinking a file the API still holds open succeeds on Linux and leaves
    # the server writing into an inode nobody else can see, so the running
    # server is what stops us rather than the file system.
    if [ -e .dev/dev.db ] && command -v fuser >/dev/null 2>&1 && fuser .dev/dev.db >/dev/null 2>&1; then
        echo ".dev/dev.db is open in another process (the dev API?); stop it first" >&2
        exit 1
    fi
    rm -f .dev/dev.db .dev/dev.db-wal .dev/dev.db-shm
    echo "removed .dev/dev.db"

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

# the landing page (apps/landing), dev server only. `just landing-build`
# builds it; that build already rides `pnpm -r build` via the workspace, so
# a broken page fails `just gates` without a recipe change here. `just
# landing-deploy` is the publish recipe (L5); it is not cleared to run yet.
landing:
    pnpm --filter dzpos-landing dev

landing-build:
    pnpm --filter dzpos-landing build

# regenerate the committed generated art: the product shots (L1) and the
# Open Graph / Twitter card (L5), both re-read from @dzpos/design and the
# desktop's committed screenshots, both then committed as ordinary files.
landing-art:
    pnpm --filter dzpos-landing shots
    pnpm --filter dzpos-landing card

# Publish apps/landing to Cloudflare Pages (project: src/lib/site.ts's
# PAGES_PROJECT). Refuses to run unless DZPOS_LANDING_PUBLISH=1: the
# product name is a placeholder, the price does not exist, and the Arabic
# translation has not been read by a native speaker
# (context/plans/20260911-landing-page.md, L5 brief).
# This page is NOT CLEARED TO GO PUBLIC YET.
landing-deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${DZPOS_LANDING_PUBLISH:-}" != "1" ]; then
        echo "landing-deploy: refusing. This page is not cleared to go public yet" >&2
        echo "(placeholder name, no price, unreviewed Arabic). Set DZPOS_LANDING_PUBLISH=1" >&2
        echo "only once Samir and Anouar have said so." >&2
        exit 1
    fi
    project="$(node -e "import('./apps/landing/src/lib/site.ts').then((m) => console.log(m.PAGES_PROJECT))")"
    pnpm --filter dzpos-landing build
    pnpm dlx wrangler@4 pages deploy apps/landing/dist --project-name "$project" --commit-dirty=true

# ---- e2e (headless chromium; starts its own API and Vite) ----

# the whole suite, once per UI language project (fr, en, ar). Each
# `playwright test` invocation starts its own API and Vite pair and its
# webServer command deletes the db file first, so three invocations give
# three empty databases; three projects in one invocation would share the
# one database the first invocation starts, and the products suite's
# first test needs an empty table. Run one language with
# `pnpm desktop e2e --project ar`.
e2e: claim
    #!/usr/bin/env bash
    set -euo pipefail
    for project in fr en ar; do
        echo "=== e2e: $project ==="
        pnpm desktop e2e --project "$project"
    done

# only the tests that write a committed screenshot; the e2e README says
# which files, under which language.
screenshot: claim
    #!/usr/bin/env bash
    set -euo pipefail
    pnpm desktop e2e -g screenshot --project fr
    pnpm desktop e2e -g screenshot --project ar

# ---- worktrees (one per task when the milestone loop runs tasks in parallel) ----

# a checkout of <branch> under the repository root's .claude/worktrees/<name>
# with its own node_modules; cargo builds into the one shared folder (see the
# top of this file), so nothing here is a target/. The root is found the same
# way CARGO_TARGET_DIR is, from --git-common-dir, so running this from inside
# a worktree adds the new worktree beside the others rather than nesting one
# worktree inside another (it nested on 2026-09-10, run from a worktree).
# The e2e ports are per worktree: pass DZPOS_E2E_API_PORT and
# DZPOS_E2E_WEB_PORT when running `just e2e` there (4319/5174 are the main
# checkout's).
worktree name branch:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
    dir="$root/.claude/worktrees/{{name}}"
    if [ -e "$dir" ]; then echo "$dir exists" >&2; exit 1; fi
    git worktree add -b "{{branch}}" "$dir" HEAD
    (cd "$dir" && pnpm install --frozen-lockfile --silent)
    echo "worktree $dir on {{branch}}; run cargo there through just, never bare"

# remove a worktree once its branch is merged.
# target/ goes first: it is gitignored, so `git worktree remove` refuses to
# touch it and the whole tree gets left behind as an orphan (that is how t5
# survived with 268K of build output and no entry in `git worktree list`).
# A worktree's target/ was 36 GB on 2026-09-10 - see `just disk`.
# The root is found the same way `worktree` above finds it, so this removes
# the right tree whether run from the main checkout or from a worktree.
worktree-rm name:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
    dir="$root/.claude/worktrees/{{name}}"
    [ -d "$dir/target" ] && rm -rf "$dir/target"
    git worktree remove "$dir"
    git worktree prune

# ---- CI on the mirror ----

# After a merge to main: copy this commit to samir1498/dz-pos-ci and watch
# the light run there (fmt, desktop eslint, release-gate script).
#
# Pull requests do not start Actions. Compiles, clippy, tests and builds
# ran on this machine already (`just gates`). The organisation's budget is
# capped, so Dinar-dz jobs refuse to start; the mirror is Samir's account
# and his free minutes. It carries no history of its own: this recipe
# force-pushes, so the mirror is always a copy and never a place work lives.
#
# A feature branch is copied too, so the mirror stays reachable, but no run
# is started. CI is a post-merge check. To force one anyway:
#   gh workflow run CI --repo samir1498/dz-pos-ci --ref <branch>
ci branch="":
    #!/usr/bin/env bash
    set -euo pipefail
    b="{{branch}}"
    [ -n "$b" ] || b="$(git rev-parse --abbrev-ref HEAD)"
    sha="$(git rev-parse "$b")"
    git remote get-url ci >/dev/null 2>&1 || git remote add ci git@github.com:samir1498/dz-pos-ci.git
    git push -q --force ci "$b:$b"
    echo "pushed $b @$sha to the mirror"
    if [ "$b" != "main" ]; then
        echo "CI runs on the mirror after a merge to main, not on a feature branch."
        echo "just gates on this machine is the PR gate."
        exit 0
    fi
    # The run is found by the commit it is testing, never by "the newest run
    # on this branch": that answer was once an hour old and was reported as
    # this push's result. A push to main starts a run by itself, so asking
    # for one as well started two runs a second apart and the concurrency
    # group killed one, which then looked like a failure. Wait for a run on
    # this commit; only start one by hand if none appears (paths-ignore).
    find_run() {
        gh run list --repo samir1498/dz-pos-ci --branch "$b" --limit 20 \
            --json databaseId,headSha \
            --jq "[.[] | select(.headSha == \"$sha\")] | .[0].databaseId" 2>/dev/null
    }
    id=""
    for _ in $(seq 1 6); do
        id="$(find_run || true)"
        [ -n "${id:-}" ] && [ "$id" != "null" ] && break
        id=""
        sleep 5
    done
    if [ -z "$id" ]; then
        echo "no run started itself; asking for one"
        gh workflow run CI --repo samir1498/dz-pos-ci --ref "$b"
        for _ in $(seq 1 12); do
            id="$(find_run || true)"
            [ -n "${id:-}" ] && [ "$id" != "null" ] && break
            id=""
            sleep 5
        done
    fi
    [ -n "$id" ] || { echo "no run appeared for $sha; look at https://github.com/samir1498/dz-pos-ci/actions" >&2; exit 1; }
    echo "watching run $id on $sha"
    gh run watch "$id" --repo samir1498/dz-pos-ci --exit-status

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
    echo "shared build folder ($CARGO_TARGET_DIR, last used by $(cat "$CARGO_TARGET_DIR/.owner" 2>/dev/null || echo nobody)):"
    du -xsh "$CARGO_TARGET_DIR" 2>/dev/null || echo "  none"
    echo
    echo "stray per-checkout target/ (there should be none):"
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

# Delete every stray Rust target/ (main checkout + each worktree) and prune
# orphaned worktree entries; with `shared=yes` the shared build folder goes
# too (the next build is a cold one, minutes). Only build output is
# removed: worktrees carry uncommitted work, so never rm -rf the tree
# itself to reclaim space.
clean-targets shared="no":
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for t in target .claude/worktrees/*/target; do
        [ -d "$t" ] || continue
        echo "removing $t ($(du -xsh "$t" | cut -f1))"
        rm -rf "$t"
    done
    if [ "{{shared}}" = "yes" ] && [ -d "$CARGO_TARGET_DIR" ]; then
        echo "removing $CARGO_TARGET_DIR ($(du -xsh "$CARGO_TARGET_DIR" | cut -f1))"
        rm -rf "$CARGO_TARGET_DIR"
    fi
    git worktree prune
    echo "host disk now:"
    df -h /mnt/c | tail -1
