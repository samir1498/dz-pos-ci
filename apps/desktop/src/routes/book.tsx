// The clinic's appointment book (C6 of
// `the-first-clinic-module-patients-queue-appointments`). Thin on purpose,
// the same reason `patients.tsx` and `queue.tsx` beside it are: this file
// exists only so the router can find it, and a build that leaves `clinic`
// out of `VITE_DINAR_MODULES` never compiles it in
// (`vite.config.ts`'s `routeFileIgnorePattern`). The screen itself is
// `-book/BookScreen.tsx`, imported by a test directly and never through
// this file.

import { createFileRoute } from "@tanstack/react-router";

import { BookScreen } from "./-book/BookScreen";

export const Route = createFileRoute("/book")({ component: BookScreen });
