#!/usr/bin/env node
// The desktop's `build` script (C6 of
// `the-first-clinic-module-patients-queue-appointments`), replacing plain
// `vite build && tsc`.
//
// `vite build` regenerates `src/routeTree.gen.ts` from whichever route
// files `VITE_DINAR_MODULES` leaves in (`vite.config.ts`'s
// `routeFileIgnorePattern`), so a build without `clinic` drops
// "/patients" and "/queue" from the registered route tree along with the
// screens behind them. `createFileRoute("/patients")` in `patients.tsx`
// needs that exact path to be a member of the tree's own
// `FileRoutesByPath` to typecheck at all -- a route file is always
// walkable by plain `tsc` (nothing about `include`/`exclude` in
// `tsconfig.json` knows which trades this build carries) even though the
// router just dropped it from the tree it will run against. So this
// script generates a matching, throwaway tsconfig that excludes precisely
// the same route files this build's `vite build` step excluded, and runs
// tsc against that one instead of the base config.
//
// Nothing else needs the same treatment: every route file's own screen
// lives in a sibling `-folder` (`-patients/PatientsScreen.tsx`), which
// carries no `createFileRoute` call and so has nothing to agree with the
// tree about; tsc keeps checking it in every build, `clinic` in or out.

import { execFileSync } from "node:child_process";
import { existsSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { excludedRouteFiles, resolveModules } from "../modules.config.mjs";

const root = path.resolve(import.meta.dirname, "..");
const scratchConfig = path.join(root, ".tsconfig.build.json");

function run(command, args) {
  execFileSync(command, args, { cwd: root, stdio: "inherit", shell: process.platform === "win32" });
}

function binary(name) {
  const ext = process.platform === "win32" ? ".cmd" : "";
  return path.join(root, "node_modules", ".bin", `${name}${ext}`);
}

const active = resolveModules(process.env.VITE_DINAR_MODULES);
const excluded = excludedRouteFiles(active).map((name) => `src/routes/${name}`);

try {
  run(binary("vite"), ["build"]);

  writeFileSync(
    scratchConfig,
    `${JSON.stringify({ extends: "./tsconfig.json", exclude: excluded }, null, 2)}\n`,
  );
  run(binary("tsc"), ["-p", scratchConfig]);
} finally {
  if (existsSync(scratchConfig)) rmSync(scratchConfig);
}
