// The clinic's patient file (C6 of
// `the-first-clinic-module-patients-queue-appointments`). Thin on purpose:
// this file exists only so the router can find it, and a build that leaves
// `clinic` out of `VITE_DINAR_MODULES` never compiles it in
// (`vite.config.ts`'s `routeFileIgnorePattern`). The screen itself is
// `-patients/PatientsScreen.tsx`, imported by a test directly and never
// through this file, so the tsc pass a build without `clinic` runs never
// needs to know "/patients" exists (`scripts/build.mjs`).

import { createFileRoute } from "@tanstack/react-router";

import { PatientsScreen } from "./-patients/PatientsScreen";

export const Route = createFileRoute("/patients")({ component: PatientsScreen });
