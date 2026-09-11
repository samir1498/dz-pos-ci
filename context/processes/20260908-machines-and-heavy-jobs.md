---
title: 'Machines and heavy jobs'
slug: 'machines-and-heavy-jobs'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'WSL box vs laptop, the shared-box claim rule, the disk gate (check /mnt/c, not /), worktree teardown'
---
# Machines and heavy jobs

Two machines. The `laptop-dev` skill has the loop between them.

| | `fedora-wsl` | `fedora` laptop |
|---|---|---|
| Role | code, tests, CI-equivalent gates | anything with a window: Tauri, Expo, a browser Samir clicks |
| Shared with | several ObserveOne sessions | nobody |
| Heavy jobs | claim the box first | build freely |

## The shared-box rule
WSL crashed on 2026-08-20 with three sessions building at once and took
every background agent with it. So on `fedora-wsl`:
- Heavy: a Tauri `cargo build` or `tauri dev`, a `pnpm build` of the
  desktop app, anything `--coverage`. `ListAgents`, announce it with a
  rough duration, wait for a clear, say when it is done.
- Not heavy, just run it: `cargo test` on `crates/core`, `cargo clippy`,
  `cargo fmt`, vitest, `tsc`.
- One heavy job at a time, foreground, and batch your own.

## The disk gate on `fedora-wsl`

**`df -h /` lies. Check `df -h /mnt/c`.**

The distro's `/` is a dynamically expanding `ext4.vhdx` that sits on the
Windows `C:` drive. On 2026-09-10 `df -h /` reported `845G avail` while the
real host disk had **479 MB** free. The guest cannot see the wall it is
about to hit.

Before any heavy job, and before the first build in a session:

```sh
df -h /mnt/c        # this is the number that matters
```

- Under 20 GB free on `/mnt/c`: clean before you build (`just disk`, then
  `just clean-targets`). Do not start a build "to see how far it gets".
- Under 5 GB: stop and tell Samir. Nothing you delete inside the distro
  reaches Windows without an elevated compact he has to run himself
  (`C:\Users\Anwender\compact-wsl.ps1` on the Windows side).

**Symptoms of a host disk already full**, seen from inside the distro:
`Input/output error` exec'ing ordinary binaries (`rm`, `wc`), `Bus error`
on `git`, segfaults on `df`. This is not a corrupt filesystem and not
your change — it is `C:` at zero. `wsl --shutdown` and restart clears it
enough to delete things. Say so plainly rather than debugging the repo.

**What cleaning actually buys, because the number will not move.** After
`just clean-targets` frees ten gigabytes, `df -h /mnt/c` reads exactly what
it read before. That is correct and not a failed clean: the space comes back
inside the VHDX, and the VHDX does not shrink. What you have bought is
headroom the *next* build reuses instead of growing the file, which is the
whole point. So read both numbers after a clean: `df -h /mnt/c` says how
much the VHDX may still grow, and `df -h /` says how much the next build can
write without growing it at all. Seen on 2026-09-11, when `/mnt/c` sat at
18 GB before and after freeing 12.5 GB and it would have been easy to
conclude the clean had done nothing.

**The VHDX only ever grows.** Sparse mode is refused on this box ("disabled
due to potential data corruption" — never pass `--allow-unsafe`). So every
gigabyte written here is permanent until Samir runs the compact. Writing
less matters more than cleaning up after.

## Sessions and worktrees

One session per machine is the normal case, so branches in the checkout
are enough and no worktree ceremony is needed. But worktrees *do* get
created here — Claude Code's own `EnterWorktree` puts them in
`.claude/worktrees/<name>` (gitignored). Until 2026-09-10 each one carried
its own Rust `target/`, and that is what filled the disk: three worktrees
held 57 GB of `target/` between them (`t7` alone was 36 GB) on top of the
main checkout's 15 GB. Since then every worktree builds into the one shared
`CARGO_TARGET_DIR` (next section), so there is no per-worktree `target/` to
manage.

So:
- **Tearing down a worktree: `just worktree-rm <name>`**, which removes a
  leftover `target/` if one exists, then `git worktree remove` and
  `git worktree prune`. Leaving the directory behind is what produced the
  orphaned `t5`, a `.claude/worktrees/` dir that `git worktree list` did
  not even know about.
- **Never `rm -rf` a worktree directory to save space.** `design` and `t7`
  held 19 and 6 uncommitted files when they were 55 GB of build output.
  Only build output is disposable, never the tree.
- `just clean-targets` removes every stray per-worktree `target/` at once.

## Dev servers
Nothing runs by default. The web UI (`just api`, then `just dev`) can run on
either machine; the native window only on the laptop. Say which servers
you started and stop them when done; use a pid file or `fuser -k
<port>/tcp`, never `pkill -f` in a chained command (it matches the shell
running it).

`just seed` fills `.dev/dev.db` with a catalogue, twelve customers, five
suppliers and thirty days of trading, so a screen has something to show; it
is deterministic, so every machine reads the same figures, and repeatable,
because it deletes the file and fills a fresh one. `just seed-clean` deletes
it and stops there, so the next `just api` opens an empty shop. Neither
recipe takes a path, and both refuse a file the dev API is holding open, so
stop `just api` first. The seeder is dev only and enforced as such in three
places; `docs/architecture.md` (Local development) says where.

## After the 125 GB day: one build folder, one build, one session

Decided with Samir on 2026-09-10 after the VHDX reached 125 GB and the host
disk hit zero five times in an hour.

- Every cargo command in this repo runs through `just`, which exports the
  one shared build folder (`.cargo-target` next to the main checkout's
  `.git`, found through `git rev-parse --git-common-dir`, so the laptop
  clone gets its own without an env var) and four build jobs (one cargo run at a time is the memory rule; several builds at once is what starved the box, not one build on four cores). One shared
  build folder for every worktree: ten worktrees cost one build's disk,
  and cargo's lock on the folder makes it one build at a time, which is
  also the memory rule.
- The shared folder has one catch (found the same afternoon): cargo names
  an artifact of our own crates the same in every worktree and decides
  freshness by mtime, so after a build in worktree A, a bare `cargo` in
  worktree B whose sources are older reuses A's `dzpos-core` without a
  word (clippy in B failed on a type only A's branch had). `just claim`
  keeps `.owner` in the shared folder; when the checkout changes it
  touches that checkout's crate sources, so the three members rebuild and
  the dependencies (identical everywhere) stay cached. Every cargo recipe
  in the justfile depends on it; a bare `cargo` in a worktree comes after
  `just claim`. The compile-and-run recipes (`just clippy`, `just test`,
  `just types`, `just types-check`) also hold `flock` on the folder for
  the whole run, because cargo's own lock only covers compilation: while
  one worktree's `cargo test` ran its binaries, another worktree's build
  replaced the rlib its doc-tests were about to link. One cargo
  invocation at a time across every checkout; the second waits. A gate
  run that overlapped another worktree's build is not a gate run: rerun
  it through `just`.
- **Holding the lock is not enough on its own: export the folder too.**
  `flock "$CARGO_TARGET_DIR/.lock" cargo test ...` typed by hand, without
  `export CARGO_TARGET_DIR=<main checkout>/.cargo-target` first, serialises
  correctly and then builds into a per-worktree `target/` anyway, because
  the variable the lock path names is empty in that shell. That is where
  both of 2026-09-11's stray target folders came from, 1.3 GB and 2.2 GB,
  and the second one was written after the first had been cleaned. The
  `just` recipes export it; a bare `cargo` needs the export as well as the
  `just claim`. `just disk` names a stray folder, so read it after a hand-run
  rather than at the start of the next session.
- **Never background a long check and wait on a process check.** Run
  `just gates`, `just test`, `just clippy` and `just e2e` in the foreground
  and read the output. This cost four agents about an hour each on
  2026-09-11 and it is the single most expensive mistake made in this repo.
  The reason it is so expensive here specifically: because of the `flock`
  rule above, a cargo run that is queued behind another worktree's run looks
  exactly like a cargo run that has hung, and a watcher polling for a
  process cannot tell the two apart. Two of the four were worse than a
  simple wait: one watched a process name that matched a *different*
  agent's run and could never have exited, and one waited on a run that had
  already finished and been cleaned up. A foreground run that takes twenty
  minutes because it is queued is a foreground run that is working.
- A worktree is torn down with `just worktree-rm` the moment its branch
  merges. Moving a worktree to a new task to keep its warm cache (what the
  loop did all morning) is what kept four `target/` folders alive.
- `df -h /mnt/c` before any build, in every heartbeat. Under 20 GB, clean
  before building; under 5 GB, stop and tell Samir.
- One Claude session per conversation. The `claude-dz` service resumes the
  session in tmux after a boot; a second `claude --continue` started by
  hand on the same transcript makes two processes fight over it and each
  resume kills the other's background agents.

