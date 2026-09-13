---
name: laptop-dev
description: Run dev servers, native app windows and builds on Samir's laptop (`fedora`, Tailscale 100.111.55.62) while the code and the Claude session stay on fedora-wsl. Use whenever Samir wants to "run it on the laptop", "see the app", "open the Tauri window", "start expo", "test on my screen", or whenever a task needs a real display, a real phone, or a GUI that WSL over SSH cannot show — dz-pos (Tauri desktop + Expo mobile) is the main case, but it applies to any repo cloned on both machines.
---

# Cross-machine dev: code on fedora-wsl, run on the laptop

Samir codes over SSH into `fedora-wsl` (this box) but sits at the laptop
`fedora`. Anything with a window — Tauri, a browser he wants to click,
Expo on his phone — is far better run on the laptop. Git is the transport:
commit on a branch here, pull there, start the server there, read its log
from here, look at it with `screenshot-pull`.

## The laptop

| | |
|---|---|
| Host / user | `fedora`, `samir@100.111.55.62` (Tailscale, key auth, no passphrase) |
| Repo path | `~/Developer/dz-pos` — same layout as here (`/home/samir/dz-pos`) |
| Shell quirk | node/pnpm come from nvm and only exist in an interactive login shell. Wrap commands: `ssh laptop 'zsh -lic "cd ~/Developer/dz-pos && pnpm …"'` — a bare `ssh laptop pnpm` says "command not found". |
| Display | GNOME Wayland on seat0. From SSH set `DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000` before anything that opens a window. |
| sudo | Needs a password. A bare `sudo` from SSH fails. Package installs go through PackageKit in the graphical session so a password dialog appears on the laptop: `systemd-run --user --wait --pipe --collect -p Slice=session.slice pkcon -y install …`. Set `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus` and `XAUTHORITY` from gnome-shell's environ. |
| Toolchain (2026-09-13) | cargo 1.96, node 22 (nvm), pnpm 11.8, gh + GitHub SSH OK. Clone at `~/Developer/dz-pos` on `6790eec`, `pnpm install` done, webkit/gtk devel packages installed, `dzpos-desktop` cargo cache warm. **Missing:** `claude`. |

`ssh laptop` below means `ssh -o BatchMode=yes -o ConnectTimeout=10 samir@100.111.55.62`.
Check `tailscale status` first if a command hangs; an offline peer waits
for the full timeout.

## The loop

1. Work on a branch here as usual. Commit (WIP commits are fine on a
   branch; squash at merge). `git push -u origin <branch>`.
2. Sync the laptop:
   `ssh laptop 'cd ~/Developer/dz-pos && git fetch -q && git checkout -q <branch> && git pull -q --ff-only && git log --oneline -1'`
   Read the SHA back and compare with `git rev-parse --short HEAD` here —
   that line is the proof the laptop runs what you just wrote.
3. Start the server detached, with a log file, and a pid file so it can be
   stopped without `pkill -f` (which has twice killed unrelated servers on
   this box because the pattern matched the shell running it):
   ```
   ssh laptop 'zsh -lic "cd ~/Developer/dz-pos && mkdir -p .dev && \
     nohup <command> > .dev/<name>.log 2>&1 & echo \$! > .dev/<name>.pid"'
   ```
4. Wait on the log, not on a sleep:
   `ssh laptop 'until grep -qE "ready|Local:|error|Error" ~/Developer/dz-pos/.dev/<name>.log; do sleep 1; done; tail -5 ~/Developer/dz-pos/.dev/<name>.log'`
5. Tell Samir what to look at (a URL, or "the window is on your screen").
   To see it yourself: `screenshot-pull` after he takes one, or drive the
   URL headless from here with Playwright against `http://100.111.55.62:<port>`.
6. Stop: `ssh laptop 'kill $(cat ~/Developer/dz-pos/.dev/<name>.pid)'`, or
   `fuser -k <port>/tcp` when the pid file is stale. Say which servers you
   started and stopped.

## Commands by server

| What | Command (inside `zsh -lic "cd ~/Developer/dz-pos && …"`) | Where it shows |
|---|---|---|
| Web UI only | `just api 4317 .dev/dev.db http://100.111.55.62:5173` in one shell, then `just dev` | `http://100.111.55.62:5173` — from the laptop browser and from this box |
| Native Tauri window | `DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus just tauri` | window on the laptop screen; first build is minutes, cached after. Pid/log under `.dev/tauri.{pid,log}`. |
| Expo (later) | `REACT_NATIVE_PACKAGER_HOSTNAME=100.111.55.62 pnpm --filter mobile start` | Expo Go on the phone over Tailscale |
| Rust / web tests | not on the laptop — run `cargo test` and `pnpm -r test` here; the laptop is for things that need a screen |

webkit/gtk devel packages are installed (2026-09-13). `just tauri` from
`apps/desktop` needs the `tauri` script in that package (`pnpm desktop tauri
dev` is what the justfile runs).

## First-time setup

Done (2026-09-13): clone, `pnpm install`, webkit/gtk devel, seeded
`.dev/dev.db`, and one `dzpos-desktop` debug build. Native window has been
opened. Stop it with `kill $(cat ~/Developer/dz-pos/.dev/tauri.pid)`, never
`pkill -f`.

## Why git and not rsync

rsync is faster for a half-done edit, but it leaves the laptop on code
that exists nowhere else and confuses which SHA a bug was seen on. A commit
on a feature branch costs nothing, and step 2's SHA line settles "is the
laptop running what I think it is" every time.

## Heavy-job rule

Builds on the laptop do not touch the shared WSL box, so they need no claim
from the other sessions. Builds here still do.
