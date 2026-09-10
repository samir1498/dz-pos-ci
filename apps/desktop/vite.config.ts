import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const host = process.env.TAURI_DEV_HOST || "127.0.0.1";

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
    tanstackRouter({ target: "react", autoCodeSplitting: false }),
    tailwindcss(),
    react(),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
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
    chunkSizeWarningLimit: 1000,
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});