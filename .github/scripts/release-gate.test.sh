#!/usr/bin/env bash
# Proof for release-gate.sh, run by hand:
#   .github/scripts/release-gate.test.sh
# Not wired into `just gates` or CI; the gate script has no git repo and no
# GitHub context to run against outside a real workflow run, so this is
# what stands in for that: two fixture files matching the real Cargo.toml /
# tauri.conf.json shape, and every branch the workflow can reach exercised
# against them.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE="$here/release-gate.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cargo_toml="$tmp/Cargo.toml"
tauri_conf="$tmp/tauri.conf.json"
cat > "$cargo_toml" <<'EOF'
[workspace]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
EOF
cat > "$tauri_conf" <<'EOF'
{
  "productName": "dz-pos",
  "version": "0.1.0",
  "identifier": "com.dzpos.app"
}
EOF

pass=0
fail=0

check() {
    local desc="$1" expect_exit="$2"
    shift 2
    local out rc
    out="$("$@" 2>&1)"
    rc=$?
    if [ "$rc" -eq "$expect_exit" ]; then
        echo "ok   - $desc"
        pass=$((pass+1))
    else
        echo "FAIL - $desc (expected exit $expect_exit, got $rc)"
        echo "       output: $out"
        fail=$((fail+1))
    fi
}

echo "=== 1. manual dispatch: always dry-run, always succeeds, carries the real version ==="
out="$("$GATE" workflow_dispatch "" "deadbeef" "" "$cargo_toml" "$tauri_conf" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^version=0.1.0$" && echo "$out" | grep -q "^mode=dry-run$"; then
    echo "ok   - dispatch dry-run carries the checked-in version, not a timestamp"
    pass=$((pass+1))
else
    echo "FAIL - dispatch dry-run ($rc): $out"
    fail=$((fail+1))
fi
echo

echo "=== 1b. manual dispatch, Cargo.toml and tauri.conf.json disagree: refused ==="
cat > "$tauri_conf.mismatch" <<'EOF'
{ "productName": "dz-pos", "version": "0.2.0" }
EOF
check "dispatch refuses when the two version files disagree" 1 \
    "$GATE" workflow_dispatch "" "deadbeef" "" "$cargo_toml" "$tauri_conf.mismatch"
echo

echo "=== 2. tag push on main, versions agree: release ==="
check "v0.1.0 on main with matching versions releases" 0 \
    "$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml" "$tauri_conf"
echo

echo "=== 3. tag push NOT on main: refused ==="
check "v0.1.0 not on main is refused" 1 \
    "$GATE" push refs/tags/v0.1.0 deadbeef false "$cargo_toml" "$tauri_conf"
echo

echo "=== 4. branch push reaching the script (ref not a tag): refused ==="
check "a branch ref is refused" 1 \
    "$GATE" push refs/heads/main deadbeef true "$cargo_toml" "$tauri_conf"
echo

echo "=== 5. malformed tag shape: refused ==="
check "'v1.0' (not full semver) is refused" 1 \
    "$GATE" push refs/tags/v1.0 deadbeef true "$cargo_toml" "$tauri_conf"
check "'1.0.0' (no v prefix) is refused" 1 \
    "$GATE" push refs/tags/1.0.0 deadbeef true "$cargo_toml" "$tauri_conf"
check "'v1.0.0-' (trailing dash, empty pre-release) is refused" 1 \
    "$GATE" push refs/tags/v1.0.0- deadbeef true "$cargo_toml" "$tauri_conf"
check "'v1.0.0+' (bare trailing plus, empty build metadata) is refused" 1 \
    "$GATE" push refs/tags/v1.0.0+ deadbeef true "$cargo_toml" "$tauri_conf"
echo

echo "=== 5b. build metadata and no pre-release: accepted, not flagged prerelease ==="
# Cargo.toml and tauri.conf.json carry the build metadata too, matching
# the tag literally: the gate compares strings, it does not normalise
# build metadata away, so all three have to spell the version identically.
cat > "$tauri_conf.build" <<'EOF'
{ "productName": "dz-pos", "version": "1.0.0+build.5" }
EOF
cat > "$cargo_toml.build" <<'EOF'
[workspace.package]
version = "1.0.0+build.5"
EOF
out="$("$GATE" push refs/tags/v1.0.0+build.5 deadbeef true "$cargo_toml.build" "$tauri_conf.build" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^mode=release$" && echo "$out" | grep -q "^prerelease=false$"; then
    echo "ok   - v1.0.0+build.5 releases and is not treated as a prerelease"
    pass=$((pass+1))
else
    echo "FAIL - v1.0.0+build.5 ($rc): $out"
    fail=$((fail+1))
fi
echo

echo "=== 6. tag version disagrees with Cargo.toml / tauri.conf.json: refused ==="
check "v9.9.9 on main but files say 0.1.0 is refused" 1 \
    "$GATE" push refs/tags/v9.9.9 deadbeef true "$cargo_toml" "$tauri_conf"
echo

echo "=== 6b. a second 'version = \"...\"' line in Cargo.toml: refused, not silently first-matched ==="
cat > "$cargo_toml.dup" <<'EOF'
[workspace.package]
version = "0.1.0"
edition = "2021"

[dependencies.somecrate]
version = "9.9.9"
EOF
check "two version lines in Cargo.toml is refused" 1 \
    "$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml.dup" "$tauri_conf"
echo

echo "=== 7. prerelease tag shape is accepted and flagged ==="
cat > "$tauri_conf.pre" <<'EOF'
{ "productName": "dz-pos", "version": "1.0.0-rc.1" }
EOF
cat > "$cargo_toml.pre" <<'EOF'
[workspace.package]
version = "1.0.0-rc.1"
EOF
out="$("$GATE" push refs/tags/v1.0.0-rc.1 deadbeef true "$cargo_toml.pre" "$tauri_conf.pre" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^prerelease=true$"; then
    echo "ok   - prerelease tag releases and sets prerelease=true"
    pass=$((pass+1))
else
    echo "FAIL - prerelease tag ($rc): $out"
    fail=$((fail+1))
fi
echo

echo "=== 8. GITHUB_OUTPUT is written when set ==="
gh_out="$tmp/gh_output"
: > "$gh_out"
GITHUB_OUTPUT="$gh_out" "$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml" "$tauri_conf" > /dev/null
if grep -q "^mode=release$" "$gh_out" && grep -q "^version=0.1.0$" "$gh_out"; then
    echo "ok   - GITHUB_OUTPUT carries mode and version"
    pass=$((pass+1))
else
    echo "FAIL - GITHUB_OUTPUT contents: $(cat "$gh_out")"
    fail=$((fail+1))
fi
echo

echo "=== 9. updater manifest: never published without the signing key, never from a dry run ==="
# A release with the key absent (the 7th argument left out, same as every
# case above this one already ran the gate without it): ships, but says
# skip. This is the refusal the brief asks for -- not the gate failing the
# whole run, but the one output that decides whether release.yml is allowed
# to sign and attach an update manifest at all. A workflow that published
# one anyway despite reading "skip" would be the bug this output exists to
# make visible in a diff, not one this script can stop by itself.
out="$("$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml" "$tauri_conf" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^updater=skip$"; then
    echo "ok   - no signing key means the release still ships, but with no updater manifest"
    pass=$((pass+1))
else
    echo "FAIL - release with no signing key ($rc): $out"
    fail=$((fail+1))
fi

out="$("$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml" "$tauri_conf" false 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^updater=skip$"; then
    echo "ok   - an explicit 'false' for the signing key also skips the manifest"
    pass=$((pass+1))
else
    echo "FAIL - release with signing key explicitly false ($rc): $out"
    fail=$((fail+1))
fi

out="$("$GATE" push refs/tags/v0.1.0 deadbeef true "$cargo_toml" "$tauri_conf" true 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^updater=publish$"; then
    echo "ok   - the signing key present publishes the manifest"
    pass=$((pass+1))
else
    echo "FAIL - release with signing key present ($rc): $out"
    fail=$((fail+1))
fi

out="$("$GATE" workflow_dispatch "" deadbeef "" "$cargo_toml" "$tauri_conf" true 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && echo "$out" | grep -q "^updater=skip$"; then
    echo "ok   - a dry run skips the manifest even if the signing key is present"
    pass=$((pass+1))
else
    echo "FAIL - dry run with signing key present ($rc): $out"
    fail=$((fail+1))
fi

echo
echo "==================================="
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
