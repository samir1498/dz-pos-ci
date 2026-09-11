#!/usr/bin/env bash
# The decision logic for .github/workflows/release.yml (M5 T6,
# docs/architecture.md § Release), pulled out of the workflow file so it can
# be run and proven here, off a real GitHub Actions runner.
#
# A tag push and a manual dispatch are the only two events this script
# knows. A manual dispatch is always a dry run: it has no tag, so there is
# nothing it could honestly publish. A tag push is a release candidate, and
# is refused (exit 1, an ::error:: line) unless all four hold: the ref is a
# tag, the tag matches the release shape `v<semver>`, the commit the tag
# names is reachable from main, and the version the tag names is the same
# version already checked into Cargo.toml and tauri.conf.json. The ancestry
# check itself needs a real git history, so it is computed by the workflow
# (a two-line `git merge-base --is-ancestor`) and handed in as ON_MAIN
# rather than done here, which is what keeps this script runnable with
# nothing but plain arguments and two fixture files.
#
# Usage:
#   release-gate.sh EVENT_NAME REF SHA ON_MAIN CARGO_TOML_PATH TAURI_CONF_PATH
#
#   EVENT_NAME        "push" or "workflow_dispatch"
#   REF               e.g. refs/tags/v0.1.0 (ignored for workflow_dispatch)
#   SHA               the commit the ref points at, for the error messages
#   ON_MAIN           "true" or "false"; ignored for workflow_dispatch
#   CARGO_TOML_PATH   path to the workspace Cargo.toml (reads
#                     [workspace.package] version, the same CARGO_PKG_VERSION
#                     crates/core/src/build_info.rs bakes into the binary)
#   TAURI_CONF_PATH   path to apps/desktop/src-tauri/tauri.conf.json (reads
#                     .version, what the bundler names the installer with)
#
# On success it writes MODE, VERSION and PRERELEASE to $GITHUB_OUTPUT when
# that variable is set (inside a real workflow step), and always prints a
# "MODE: ..." line to stdout so a human reading the run's log sees the
# decision without opening the job summary.

set -euo pipefail

fail() {
    echo "::error::$1" >&2
    exit 1
}

event_name="${1:-}"
ref="${2:-}"
sha="${3:-}"
on_main="${4:-}"
cargo_toml="${5:-}"
tauri_conf="${6:-}"

[ -n "$event_name" ] || fail "release-gate.sh: no event name given"

emit() {
    # name=value, to $GITHUB_OUTPUT when running as a step and always to
    # stdout, so a bare local run still shows what would have been set.
    echo "$1=$2"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        echo "$1=$2" >> "$GITHUB_OUTPUT"
    fi
}

read_versions() {
    [ -f "$cargo_toml" ] || fail "cannot read '$cargo_toml' to check the workspace version"
    [ -f "$tauri_conf" ] || fail "cannot read '$tauri_conf' to check the bundle version"
    cargo_version="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$cargo_toml" | head -n1)"
    [ -n "$cargo_version" ] || fail "no [workspace.package] version found in '$cargo_toml'"
    tauri_version="$(sed -n 's/.*"version" *: *"\([^"]*\)".*/\1/p' "$tauri_conf" | head -n1)"
    [ -n "$tauri_version" ] || fail "no \"version\" found in '$tauri_conf'"
}

if [ "$event_name" = "workflow_dispatch" ]; then
    read_versions
    [ "$cargo_version" = "$tauri_version" ] || fail "Cargo.toml has $cargo_version but tauri.conf.json has $tauri_version; these must agree before even a dry-run build carries a meaningful version"
    echo "MODE: dry-run (manual dispatch, no tag involved)"
    echo "This run builds the Windows installer, version $cargo_version, and uploads it as a run artifact."
    echo "It does not create a release and does not touch any tag."
    emit mode dry-run
    emit version "$cargo_version"
    emit prerelease "false"
    exit 0
fi

[ "$event_name" = "push" ] || fail "release.yml does not run on '$event_name'; only a tag push or a manual dispatch reach here"

case "$ref" in
refs/tags/*) ;;
*) fail "ref '$ref' from a push event is not a tag; releases are only cut from a tag, never from a branch push" ;;
esac

tag="${ref#refs/tags/}"

semver_re='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
[[ "$tag" =~ $semver_re ]] || fail "tag '$tag' does not match the release shape v<major>.<minor>.<patch>[-pre][+build]; not publishing"

version="${tag#v}"
if [[ "$version" == *-* ]]; then
    prerelease="true"
else
    prerelease="false"
fi

[ "$on_main" = "true" ] || fail "tag '$tag' (commit $sha) is not reachable from main; releases are only cut from a tag on main"

read_versions

if [ "$cargo_version" != "$version" ] || [ "$tauri_version" != "$version" ]; then
    fail "tag '$tag' names version $version, but Cargo.toml has $cargo_version and tauri.conf.json has $tauri_version; all three must agree before a release is cut (this is the same version the About screen reads, crates/core/src/build_info.rs)"
fi

echo "MODE: release (tag $tag on main, version $version, Cargo.toml and tauri.conf.json agree)"
emit mode "release"
emit version "$version"
emit prerelease "$prerelease"
