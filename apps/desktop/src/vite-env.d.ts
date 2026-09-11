/// <reference types="vite/client" />

// Declared so `import.meta.env.VITE_API_URL` is a string, not `any`; an
// `any` here would hide a wrong base URL until runtime.
interface ImportMetaEnv {
  readonly VITE_API_URL?: string;
  readonly VITE_API_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

// Injected by the Tauri process before any app script runs; absent in a
// plain browser. The launch token does not travel this way; see api.ts.
declare const __DZPOS_API_URL__: string | undefined;
