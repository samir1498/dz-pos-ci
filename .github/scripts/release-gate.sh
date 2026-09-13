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
#   release-gate.sh EVENT_NAME REF SHA ON_MAIN CARGO_TOML_PATH TAURI_CONF_PATH \
#                    [UPDATER_KEY_PRESENT]
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
#   UPDATER_KEY_PRESENT  "true" if the workflow found a
#                     TAURI_SIGNING_PRIVATE_KEY secret, anything else
#                     (default: absent) otherwise. Decides UPDATER below; it
#                     cannot be worked out from the two files above, so the
#                     workflow reads the secret and hands the answer in the
#                     same way it hands in ON_MAIN.
#
# On success it writes MODE, VERSION, PRERELEASE and UPDATER to
# $GITHUB_OUTPUT when that variable is set (inside a real workflow step),
# and always prints a "MODE: ..." line to stdout so a human reading the
# run's log sees the decision without opening the job summary.
#
# UPDATER is "publish" only for a real release with the signing key present,
# "skip" otherwise (a dry run, or a release with no key). It never fails the
# run by itself: a release with no updater manifest still ships an
# installer a shop can download by hand, which is strictly better than the
# alternative this guards against -- a manifest signed with nothing, which
# an existing install would trust as if it were real (docs/architecture.md
# § Release, "somebody holds the key that signs updates").

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
updater_key_present="${7:-}"

[ -n "$event_name" ] || fail "release-gate.sh: no event name given"

emit() {
    # name=value, to $GITHUB_OUTPUT when running as a step and always to
    # stdout, so a bare local run still shows what would have been set.
    echo "$1=$2"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        echo "$1=$2" >> "$GITHUB_OUTPUT"
    fi
}

# Reads exactly one `version = "..."` line out of a file, refusing rather
# than silently taking the first match if there turn out to be more than
# one: a `[dependencies.foo]\nversion = "..."` table in Cargo.toml, or a
# nested "version" key some future tauri.conf.json plugin block adds, would
# both match the pattern below. A script that took the first line would
# read a stranger's version instead of the workspace's and never say so.
read_one_version() {
    local path="$1" pattern="$2" what="$3" matches n
    [ -f "$path" ] || fail "cannot read '$path' to check the $what version"
    matches="$(sed -n "$pattern" "$path")"
    n="$(printf '%s\n' "$matches" | grep -c .)" || true
    [ "$n" -ge 1 ] || fail "no $what version found in '$path'"
    [ "$n" -eq 1 ] || fail "found $n lines matching a $what version in '$path', not 1; make the match specific or remove the extra one before this script can trust either"
    printf '%s' "$matches"
}

read_versions() {
    cargo_version="$(read_one_version "$cargo_toml" 's/^version *= *"\([^"]*\)".*/\1/p' "workspace")"
    tauri_version="$(read_one_version "$tauri_conf" 's/.*"version" *: *"\([^"]*\)".*/\1/p' "bundle")"
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
    # A dry run never publishes anything, the manifest included: there is
    # no tag and no release for a shop's endpoint to ever see this build
    # at, so signing one here would prove nothing and ship nothing.
    emit updater skip
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

if [ "$updater_key_present" = "true" ]; then
    updater="publish"
else
    updater="skip"
    echo "::warning::no updater signing key; this release ships an installer but no update manifest, so an existing install will not discover it on its own"
fi
emit updater "$updater"
