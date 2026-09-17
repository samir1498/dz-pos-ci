---
name: adhd-toggle
description: Toggle the i-have-adhd plugin on/off. Use when you want to disable the ADHD-friendly output, re-enable it, or uninstall the plugin entirely. Covers always-on flag, plugin entry, and cleanup.
---

# ADHD plugin toggle — install / uninstall / pause

This skill controls the `i-have-adhd` OpenCode plugin (always-on mode).

## Check current state

```bash
ls -la ~/.config/opencode/.i-have-adhd-always 2>&1 | head -1
cat ~/.config/opencode/opencode.json 2>&1 | head -20
ls -la ~/.config/opencode/vendor/i-have-adhd 2>&1 | head -5
```

- `.i-have-adhd-always` exists → always-on is active (rules injected every turn).
- `opencode.json` has `plugin: [...i-have-adhd.mjs]` → plugin loaded.
- `vendor/i-have-adhd` exists → installed.

## Pause for this session only (keep installed)

In any OpenCode session:

```
/i-have-adhd stop adhd mode
# or
/i-have-adhd normal mode
```

Rules skip for the rest of that session. Next session they return if always-on flag still exists. No file changes.

## Disable always-on (keep plugin, manual trigger only)

```bash
rm ~/.config/opencode/.i-have-adhd-always
# verify
ls ~/.config/opencode/.i-have-adhd-always 2>&1 | head -1
```

Plugin stays installed. Turn it on per-session with `/i-have-adhd`.

## Re-enable always-on

```bash
touch ~/.config/opencode/.i-have-adhd-always
```

## Uninstall entirely

```bash
# 1. remove always-on flag
rm -f ~/.config/opencode/.i-have-adhd-always
# 2. remove plugin entry from global config (edit file, delete the i-have-adhd line)
#    keep other plugins (e.g. pc-ctx stays in your project opencode.json)
cat ~/.config/opencode/opencode.json
# edit to remove the "/home/samir/.config/opencode/vendor/i-have-adhd/..." entry,
# or if it's the only entry, delete the file:
# rm ~/.config/opencode/opencode.json
# 3. remove the checkout
rm -rf ~/.config/opencode/vendor/i-have-adhd
# 4. verify
ls ~/.config/opencode/vendor/i-have-adhd 2>&1 | head -1
```

## Re-install (if you removed it and want it back)

```bash
mkdir -p ~/.config/opencode/vendor
git clone https://github.com/ayghri/i-have-adhd ~/.config/opencode/vendor/i-have-adhd
# re-add to global opencode.json
cat > ~/.config/opencode/opencode.json <<'JSON'
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["/home/samir/.config/opencode/vendor/i-have-adhd/.opencode/plugins/i-have-adhd.mjs"]
}
JSON
touch ~/.config/opencode/.i-have-adhd-always
```

Then start a new OpenCode session and run `/i-have-adhd` to verify it appears in the `/` command list.

## Update

```bash
git -C ~/.config/opencode/vendor/i-have-adhd pull
```
