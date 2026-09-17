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

# No huge files. A ratchet, not a big-bang rule: the files already over the
# line limit are pinned at today's length in scripts/file-sizes.json, they
# may not grow, and an entry back under the limit has to be deleted, so the
# list only shrinks. Nothing new joins it. Same shape as the eslint allowlist
# above, and the same reason: crates/api/src/dto.rs reached 3071 lines holding
# every domain at once, which made it the first place two branches collided.
# No cargo, so it runs early and cheap.
sizes:
    node scripts/file-sizes.mjs

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
gates: fmt lint sizes clippy types-check test build

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
# A real phone needs the API on the LAN, not loopback: append `lan`
# (`just api 4317 .dev/dev.db "" lan`). That binds 0.0.0.0 and announces
# mDNS `Dinar-<shop>`; the launch token, device gate and session still
# guard every call, so only run it on a Wi-Fi you trust, with the dev file.
api port="4317" db=".dev/dev.db" origin="" lan="": claim
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .dev
    umask 077
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > .dev/api-token
    chmod 600 .dev/api-token
    DZPOS_API_TOKEN="$(cat .dev/api-token)" cargo run -p dzpos-api -- --db {{db}} --port {{port}} {{ if origin != "" { "--allow-origin " + origin } else { "" } }} {{ if lan != "" { "--lan" } else { "" } }}

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
# price does not exist, and the Arabic translation has not been read by a
# native speaker (context/plans/20260911-landing-page.md, L5 brief).
# This page is NOT CLEARED TO GO PUBLIC YET.
landing-deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${DZPOS_LANDING_PUBLISH:-}" != "1" ]; then
        echo "landing-deploy: refusing. This page is not cleared to go public yet" >&2
        echo "(no price, unreviewed Arabic). Set DZPOS_LANDING_PUBLISH=1" >&2
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
# Mutation testing over the one module where a passing test proves least:
# the burst rule that tells a barcode scanner from a cashier typing. Stryker
# changes each threshold and each comparison in turn and the unit tests have
# to notice. It runs vitest many times over, so it is its own recipe and not
# part of `gates`; the report lands in
# apps/desktop/e2e/.artifacts/mutation/index.html.
mutate:
    pnpm desktop exec stryker run

e2e: claim
    #!/usr/bin/env bash
    set -euo pipefail
    for project in fr en ar; do
        echo "=== e2e: $project ==="
        pnpm desktop e2e --project "$project"
    done

# the demo footage: run the recording project (apps/desktop/e2e/demo, one
# scene per spec, French, 1920x1080), then convert each clip to an mp4 the
# Remotion project reads (webm seeks badly there). Clips are gitignored on
# both sides; a recording is one command away.
demo-clips: claim
    #!/usr/bin/env bash
    set -euo pipefail
    # DZPOS_DEMO is what makes the demo project exist at all
    # (apps/desktop/playwright.config.ts): without it a bare
    # `playwright test` would record five scenes over the e2e shop file.
    DZPOS_DEMO=1 pnpm desktop e2e --project demo
    out="${DZPOS_DEMO_OUT:-$HOME/dinar-remotion/public/recordings}"
    mkdir -p "$out"
    for webm in apps/desktop/demo-clips/*.webm; do
        name="$(basename "$webm" .webm)"
        ffmpeg -y -loglevel error -i "$webm" -c:v libx264 -crf 18 -pix_fmt yuv420p -movflags +faststart "$out/$name.mp4"
        echo "$out/$name.mp4"
    done

# the phone scene: Maestro drives Expo Go on the `dinar` emulator while
# `adb shell screenrecord` films it (scripts/demo-phone.sh says which
# variables point it at a Windows-side Maestro and adb).
demo-phone:
    ./scripts/demo-phone.sh

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

# Copy this commit to the public personal mirror samir1498/dz-pos-ci
# (Actions are free there) and watch the runs. Dinar-dz never starts a
# runner. Restricted (fmt, eslint, release-gate) runs on every push.
# Full (clippy, tests, build; Windows and coverage on main) runs on main,
# or on a branch with `just ci <branch> full`.
#
# The mirror is always a copy: this force-pushes. Work does not live there.
ci branch="" extra="":
    #!/usr/bin/env bash
    set -euo pipefail
    b="{{branch}}"
    [ -n "$b" ] || b="$(git rev-parse --abbrev-ref HEAD)"
    sha="$(git rev-parse "$b")"
    git remote get-url ci >/dev/null 2>&1 || git remote add ci git@github.com:samir1498/dz-pos-ci.git
    git push -q --force ci "$b:$b"
    echo "pushed $b @$sha to the mirror"
    # The run is found by the commit it is testing, never by "the newest run
    # on this branch". A push starts Restricted itself; Full starts itself
    # only on main. Asking for a run that already started used to launch a
    # second one that concurrency then killed.
    find_run() {
        local workflow="$1"
        gh run list --repo samir1498/dz-pos-ci --workflow "$workflow" --branch "$b" --limit 20 \
            --json databaseId,headSha \
            --jq "[.[] | select(.headSha == \"$sha\")] | .[0].databaseId" 2>/dev/null
    }
    wait_for() {
        local workflow="$1"
        local dispatch="${2:-}"
        local id=""
        for _ in $(seq 1 8); do
            id="$(find_run "$workflow" || true)"
            [ -n "${id:-}" ] && [ "$id" != "null" ] && break
            id=""
            sleep 5
        done
        if [ -z "$id" ]; then
            echo "no $workflow run started itself; asking for one"
            if [ "$dispatch" = "windows" ]; then
                gh workflow run "$workflow" --repo samir1498/dz-pos-ci --ref "$b" -f windows=true
            else
                gh workflow run "$workflow" --repo samir1498/dz-pos-ci --ref "$b"
            fi
            for _ in $(seq 1 12); do
                id="$(find_run "$workflow" || true)"
                [ -n "${id:-}" ] && [ "$id" != "null" ] && break
                id=""
                sleep 5
            done
        fi
        [ -n "$id" ] || { echo "no $workflow run appeared for $sha; look at https://github.com/samir1498/dz-pos-ci/actions" >&2; exit 1; }
        echo "watching $workflow run $id on $sha"
        gh run watch "$id" --repo samir1498/dz-pos-ci --exit-status
    }
    wait_for Restricted
    if [ "$b" = "main" ] || [ "{{extra}}" = "full" ] || [ "{{extra}}" = "windows" ]; then
        wait_for CI "{{extra}}"
    fi

# Cut a release: tags main as vX.Y.Z and pushes the tag to the org and to
# the public mirror. The mirror tag run is what builds (org minutes are
# capped); the publish job then creates the release on Dinar-dz/dz-pos.
# Versions must already agree (Cargo.toml, tauri.conf.json) or the gate
# refuses the run. Usage: just release v0.1.0
release tag:
    #!/usr/bin/env bash
    set -euo pipefail
    [ "$(git rev-parse --abbrev-ref HEAD)" = "main" ] || { echo "release: run from main, not $(git rev-parse --abbrev-ref HEAD)" >&2; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "release: working tree is not clean" >&2; exit 1; }
    [[ "{{tag}}" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]] || { echo "release: '{{tag}}' is not v<major>.<minor>.<patch>[-pre][+build]" >&2; exit 1; }
    git fetch -q origin main
    [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || { echo "release: main is behind origin/main; pull first" >&2; exit 1; }
    git tag "{{tag}}"
    git push origin "{{tag}}"
    git remote get-url ci >/dev/null 2>&1 || git remote add ci git@github.com:samir1498/dz-pos-ci.git
    # Explicit tag refspec: a bare `git push ci <tag>` can resolve as a
    # branch push, which would silently create a branch named like a
    # version and never trigger the release workflow (tags: ["v*"]).
    git push ci "refs/tags/{{tag}}:refs/tags/{{tag}}"
    echo "tag {{tag}} on origin and mirror; watching the mirror release run"
    sha="$(git rev-parse "{{tag}}^{commit}")"
    id=""
    for _ in $(seq 1 12); do
        id="$(gh run list --repo samir1498/dz-pos-ci --workflow Release --limit 20 --json databaseId,headSha --jq "[.[] | select(.headSha == \"$sha\")] | .[0].databaseId" 2>/dev/null || true)"
        [ -n "${id:-}" ] && [ "$id" != "null" ] && break
        id=""
        sleep 10
    done
    [ -n "$id" ] || { echo "no Release run appeared for $sha; look at https://github.com/samir1498/dz-pos-ci/actions" >&2; exit 1; }
    gh run watch "$id" --repo samir1498/dz-pos-ci --exit-status

# Scan this checkout against sonar.observeone.com. Not CI and not a PR
# check: same as ObserveOne, run on the machine before merge and again on
# main after. The Rust plugin shells out to `cargo clippy`, so this has to
# run on the host (the scanner-cli image is Amazon Linux 2023 and cannot
# exec our glibc-2.39 cargo). Coverage reports are used if they already
# exist; this does not regenerate them. Token from SONARQUBE_TOKEN or
# SONAR_ANALYSIS_TOKEN. Scanner: ~/.local/share/sonar-scanner (8.0.1.6346).
sonar: claim desktop-dist
    #!/usr/bin/env bash
    set -euo pipefail
    token="${SONARQUBE_TOKEN:-${SONAR_ANALYSIS_TOKEN:-}}"
    [ -n "$token" ] || { echo "SONARQUBE_TOKEN is unset" >&2; exit 1; }
    host="${SONARQUBE_URL:-${SONAR_HOST_URL:-https://sonar.observeone.com}}"
    scanner="${SONAR_SCANNER:-$HOME/.local/share/sonar-scanner/bin/sonar-scanner}"
    [ -x "$scanner" ] || { echo "sonar-scanner not at $scanner — install sonar-scanner-cli 8.0.1.6346 linux-x64 there" >&2; exit 1; }
    cwd="$(pwd -P)"
    branch="$(git rev-parse --abbrev-ref HEAD)"
    extra=()
    if [ -f "$cwd/.git" ]; then
        if [ "$branch" = "main" ] || [ "$branch" = "master" ]; then
            echo "refusing: only the main checkout may publish the main dashboard" >&2
            exit 1
        fi
        extra+=(-Dsonar.scm.exclusions.disabled=true)
    fi
    echo "scanning $cwd as dz-pos on branch $branch"
    flock "$CARGO_TARGET_DIR/.lock" \
        env SONAR_TOKEN="$token" SONAR_HOST_URL="$host" \
        "$scanner" \
        -Dsonar.javascript.node.maxspace=1536 \
        -Dsonar.branch.name="$branch" \
        "${extra[@]}"

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
