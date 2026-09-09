// The two Node functions the fixture-reading test uses, declared here
// rather than by taking a dependency on @types/node.
//
// `pnpm add -D @types/node` in this package rewrote 600 lines of
// pnpm-lock.yaml (peer resolutions of vite and rolldown across every
// workspace importer) for two function signatures, and a lockfile churn
// that size in a task branch is a merge conflict waiting for the session.
// Nothing shipped by this package touches Node: only `totals.test.ts`
// does, to read the files under fixtures/money/ that also pin the Rust
// core, so the surface declared here stays exactly those two functions.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
}
