#!/usr/bin/env node
// Wraps the Tauri CLI so every run through this package's own `tauri`
// script (`just tauri`'s `pnpm desktop tauri dev`, and any future
// `pnpm desktop tauri build`) carries the cargo features that match
// `VITE_DINAR_MODULES`, the same source `scripts/build.mjs` reads for the
// frontend half of the same switch (C6 of
// `the-first-clinic-module-patients-queue-appointments`; whole-loop review,
// 2026-09-24, T2: the installed app's Rust binary had no clinic routes
// because nothing ever told cargo to build them, whatever the frontend
// carried).
//
// `pnpm exec tauri` (the release workflow's own call, `.github/workflows
// /release.yml`) resolves the binary straight out of `node_modules/.bin`
// and never reaches this file or this package's `package.json` at all;
// that build stays retail-only until release day actually ships a clinic,
// and is out of this fix's reach on purpose (see the module doc on
// `cargoFeatureArgs`).

import { execFileSync } from "node:child_process";
import path from "node:path";

import { cargoFeatureArgs, resolveModules } from "../modules.config.mjs";

const root = path.resolve(import.meta.dirname, "..");

function binary(name) {
  const ext = process.platform === "win32" ? ".cmd" : "";
  return path.join(root, "node_modules", ".bin", `${name}${ext}`);
}

// Only `dev` and `build` ever touch cargo; `tauri icon`, `tauri info` and
// the rest take the caller's own arguments untouched.
const forwarded = process.argv.slice(2);
const buildsRust = forwarded[0] === "dev" || forwarded[0] === "build";
const active = resolveModules(process.env.VITE_DINAR_MODULES);
const args = buildsRust ? [...forwarded, ...cargoFeatureArgs(active)] : forwarded;

execFileSync(binary("tauri"), args, {
  cwd: root,
  stdio: "inherit",
  shell: process.platform === "win32",
});
