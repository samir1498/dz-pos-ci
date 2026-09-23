import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

import { resolveModules, routeIgnorePatternSource } from "./modules.config.mjs";

const host = process.env.TAURI_DEV_HOST || "127.0.0.1";

// Which trades this build carries (C6 of
// `the-first-clinic-module-patients-queue-appointments`), `VITE_DINAR_MODULES`
// being a comma list such as "retail,clinic"; unset or unknown falls back to
// "retail" alone, so a build nobody configured is the shop everyone already
// has. `modules.config.mjs` holds the one list of which route file belongs
// to which trade; `scripts/build.mjs`'s tsc pass reads the same list so the
// two agree on what this specific build actually compiles.
const builtModules = resolveModules(process.env.VITE_DINAR_MODULES);
const routeFileIgnorePattern = routeIgnorePatternSource(builtModules) ?? undefined;

export default defineConfig({
  plugins: [
    // No automatic code splitting. It exists to keep a first paint over a
    // network small, and this app has no network in front of it: Tauri loads
    // one bundle off the disk beside the binary, so a split screen is a
    // second local file read and nothing saved. Splitting also warned on
    // every build about the screens the route files export, which the
    // component tests render directly (`customers.test.tsx` and the three
    // beside it); the choice was between exporting the screens and keeping a
    // warning nobody was going to act on, and the exports are what the tests
    // need.
    //
    // `routeFileIgnorePattern`: a module not in this build's list never
    // reaches the generated route tree, so its screen and everything it
    // imports (its `-folder`) never reach the bundle either -- excluded at
    // compile time, not hidden behind a runtime check
    // (`just check-clinic-bundle` proves it by grepping the built assets).
    tanstackRouter({ target: "react", autoCodeSplitting: false, routeFileIgnorePattern }),
    tailwindcss(),
    react(),
  ],
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "esnext",
    // The 500 kB default warns about a first paint over a network, which is
    // the same thing the splitting above was turned off for: the bundle is
    // read off the disk beside the binary. Raised rather than silenced, so it
    // still says something when the bundle grows by half again.
    // Raised from 750 to 1000 when the shadcn kit landed (D3): the bundle
    // went to about 875 kB minified, 255 kB gzipped, and the components and
    // their Radix primitives are the growth. Raised, not removed, for the
    // same reason as before: it should say something again the next time the
    // bundle grows by half.
    // Raised again from 1000 to 1400 when the dashboard's chart landed:
    // recharts and its d3 scales are about 410 kB minified, 120 kB gzipped,
    // and they are read off the disk beside the binary like the rest of it.
    // The chart is one screen's, so this is the place a split would pay for
    // itself if the desktop ever loaded over a network.
    chunkSizeWarningLimit: 1400,
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});