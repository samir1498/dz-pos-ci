// One rule, and the list of files that are still allowed to break it.
//
// A screen that writes `<button>` or `<input>` gets the browser's default
// control: a colour the themes never heard of, a height off the scale, a
// focus ring that is the platform's rather than ours, and in Arabic a text
// alignment nobody chose. That is what the whole kit exists to stop, and it
// is the one thing review kept missing because a bare element looks like
// nothing in a diff.
//
// So the five elements the kit replaces are refused in JSX. `components/ui`
// is where they are supposed to be, since that is what a shadcn component
// is made of, and the kit files on top of it are allowed the ones they wrap.
//
// The allowlist is the point of the exercise. Every screen written before the
// kit is named in src/lint/allowlist.json with the screen it is, so the list
// reads as a to-do rather than as a permission, and the wave that rewrites
// the screens deletes one line per screen. When the file's `screens` array is
// empty the migration is done, and `tests/lint/allowlist.test.ts` refuses an
// entry whose file no longer has anything to fix, so the list cannot outlive
// the work.
//
// Tests are out of scope: a `<button>` in a fixture is not a control the shop
// ever sees, and forcing kit components into test scaffolding would make the
// tests harder to read for nothing.
//
// Deliberately just this one rule for now. The import rules in
// context/processes/frontend-conventions are written against a `features/`
// tree the app does not have yet and would fail on every screen today; they
// land with the screen wave.

import { readFileSync } from "node:fs";

import tseslint from "typescript-eslint";

const allowlist = JSON.parse(
  readFileSync(new URL("./src/lint/allowlist.json", import.meta.url), "utf8"),
);

/** What each element should have been. The message is the fix, not a scolding. */
const INSTEAD = {
  input: "Input from @/components/ui/input, or MoneyInput for an amount",
  button: "Button from @/components/ui/button, or PayButton for the payment",
  select: "Select from @/components/ui/select",
  textarea: "Textarea from @/components/ui/textarea",
  table: "DataTable from @/components/DataTable",
};

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "src/routeTree.gen.ts",
      // shadcn's own files: the elements are what they are built out of.
      "src/components/ui/**",
      // Fixtures and harnesses, not controls the shop sees.
      "**/*.test.ts",
      "**/*.test.tsx",
      ...allowlist.screens.map((entry) => entry.file),
    ],
  },
  {
    files: ["src/**/*.tsx"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
        ecmaFeatures: { jsx: true },
      },
    },
    rules: {
      "no-restricted-syntax": [
        "error",
        ...allowlist.elements.map((element) => ({
          selector: `JSXOpeningElement[name.name="${element}"]`,
          message: `<${element}> outside the kit. Use ${INSTEAD[element]}; a bare element wears the browser's colours and heights, not the theme's.`,
        })),
      ],
    },
  },
);
