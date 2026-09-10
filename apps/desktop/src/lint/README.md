# The bare-element lint

`eslint.config.js` carries one rule of ours: no `<input>`, `<button>`,
`<select>`, `<textarea>` or `<table>` in JSX outside `components/ui/` and
the kit. A bare element wears the browser's colour and height and the
platform's focus ring, and it looks like nothing in a diff, which is why it
is a rule rather than a review note. Tests are out of scope.

`allowlist.json` names every file that still carries one, with the screen
it belongs to, so the list reads as work left rather than as permission.
`allowlist.test.ts` fails on an entry whose file is gone, and on an entry
whose file has nothing left to fix: rewrite a file on the kit and the gates
make you delete its line in the same commit, which is what makes the count
reach zero. Eighteen entries when the kit landed on 2026-09-10; two by the
evening of the same day (the theme switch, and the fields helper).

`just lint` runs it; `just gates` runs it second, after `fmt`, because it
needs no cargo.
