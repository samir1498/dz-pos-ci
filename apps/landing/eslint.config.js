// Astro's own recommended flat config (eslint-plugin-astro) plus
// typescript-eslint's recommended set, the same pair the project's docs use.
// No hand-rolled Astro parsing: that plugin already carries it.
import js from "@eslint/js";
import eslintPluginAstro from "eslint-plugin-astro";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/**", "src/styles/tokens.css", ".astro/**"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...eslintPluginAstro.configs.recommended,
);
