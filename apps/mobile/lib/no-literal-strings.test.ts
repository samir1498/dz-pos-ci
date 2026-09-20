// The rule that stops the next screen reintroducing an English sentence.
//
// Every literal on the phone was replaced by a dictionary key on
// 2026-09-20. Nothing stopped the next screen from typing one straight back
// in, and a missing translation is not a compile error: `<Text>Pay
// cash</Text>` builds, ships, and is English on an Arabic counter.
//
// It lives under `lib/` because that is what `vitest.config.ts` collects, and
// it reads the screens rather than rendering them, so it costs no test
// renderer and no jsdom.
//
// Parsed with the TypeScript compiler rather than matched with a regular
// expression. A regex over source text cannot tell `<Text>Pay cash</Text>`
// from `useState<string | null>(null)` or from `a > b && c < d`: the first
// attempt at this file reported nine screens, and every one of the hits was
// a generic or a comparison. The parser knows which `>` opened a tag.
//
// It asks whether a literal is drawn, rather than matching a list of node
// kinds. The list version caught text between tags and `title="Pay cash"`
// and nothing else, so `{count === 0 ? "Basket empty" : ...}` walked
// straight past it, and that ternary was the second most common shape in
// the diff this file was written to protect. The tests lens proved it on
// 2026-09-20 with five sentences lifted out of these very screens.
//
// One hole is left and it needs more than a parser to close: a sentence
// held in a `const` above the component and drawn as `{LABEL}` is a name
// here, not a string. Nothing in `apps/mobile` does that today.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

const APP = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Props and object keys that end up in front of a person: a button's
 *  words, a field's label, the placeholder in an empty box, the line a
 *  field shows when the server said no, and what a screen reader reads.
 *
 *  `problem` is here because `components/ui/Field.tsx` and `PinBoxes.tsx`
 *  both take one and render it straight into a `Text`. The tests lens
 *  named it on 2026-09-20 as the most natural place to write the next
 *  literal; no call site passes one today.
 *
 *  Every other string prop is a token name, a keyboard type or a test id. */
const SHOWN = new Set([
  "title",
  "label",
  "placeholder",
  "problem",
  "accessibilityLabel",
  "aria-label",
]);

/** Two letters in a row. One letter is an initial or a unit, and neither is
 *  a sentence; `·`, `—` and a bare number are layout, not copy. */
const HAS_WORDS = /\p{L}{2}/u;

function literalsIn(file: string): string[] {
  const source = ts.createSourceFile(
    file,
    readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const found: string[] = [];

  const visit = (node: ts.Node): void => {
    // Text between tags is drawn by definition; nothing to walk.
    if (ts.isJsxText(node) && HAS_WORDS.test(node.text)) {
      found.push(node.text.trim());
    }
    if (ts.isStringLiteral(node) && HAS_WORDS.test(node.text) && drawn(node, source)) {
      found.push(`"${node.text}"`);
    }
    // Only the written parts of a template are read; what the holes
    // interpolate is the screen's business. This is how the words got back
    // in last time, as `{`Change: ${amount} DA`}`.
    if (ts.isTemplateExpression(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      const written = ts.isNoSubstitutionTemplateLiteral(node)
        ? node.text
        : node.head.text + node.templateSpans.map((span) => span.literal.text).join("");
      if (HAS_WORDS.test(written) && drawn(node, source)) found.push(`\`${written}\``);
    }
    ts.forEachChild(node, visit);
  };

  visit(source);
  return found;
}

/** Whether a literal is drawn, or only passed to something that draws
 *  nothing.
 *
 *  Decided by walking up to the nearest `{...}` in JSX and refusing what
 *  is on the way:
 *
 *  - an argument to a call, because `t("till_pay_cash")` and
 *    `router.replace("/pair")` are both strings nobody reads;
 *  - the body of a function, because an `onPress` runs later and draws
 *    nothing at the point it is written;
 *  - a property of an object, unless the property is one of the shown
 *    ones, which is what keeps `style={{ flexDirection: "row" }}` out
 *    while leaving `screenOptions={{ title: "Sales" }}` in;
 *  - the index in a lookup, because `theme.colors["on-primary"]` names a
 *    token and reads out nothing.
 *
 *  Which `{...}` it is decides. One between tags is drawn whatever is
 *  inside it; one on a prop is drawn only if the prop is shown. Without
 *  that split `tone={problem === null ? "tertiary" : "danger"}` in
 *  `components/ui/Field.tsx` reads as two sentences, and four of the kit's
 *  own files failed on their token names.
 *
 *  A bare attribute stops the walk on its own name too, so
 *  `title="Pay cash"` is caught and `variant="secondary"` is not. */
function drawn(node: ts.Node, source: ts.SourceFile): boolean {
  let child: ts.Node = node;
  // A shown key inside an object wins over the prop that object sits on.
  // `screenOptions={{ title: "Dinar till" }}` is a heading, and without
  // this the outer `screenOptions` is not a shown prop and the walk
  // refused it. That was the one of five shapes the rewrite still missed.
  let viaShownKey = false;
  for (let parent = node.parent; parent !== undefined; parent = parent.parent) {
    if (ts.isJsxExpression(parent)) {
      if (viaShownKey) return true;
      const holder = parent.parent;
      return holder !== undefined && ts.isJsxAttribute(holder)
        ? SHOWN.has(holder.name.getText(source))
        : true;
    }
    if (ts.isJsxAttribute(parent)) return viaShownKey || SHOWN.has(parent.name.getText(source));
    if (ts.isCallExpression(parent) || ts.isNewExpression(parent)) {
      if (parent.arguments?.some((argument) => argument === child)) return false;
    }
    if (ts.isElementAccessExpression(parent) && parent.argumentExpression === child) return false;
    if (ts.isPropertyAssignment(parent) && parent.initializer === child) {
      if (SHOWN.has(parent.name.getText(source).replace(/["']/g, ""))) viaShownKey = true;
      else return false;
    }
    if (ts.isFunctionLike(parent)) return false;
    child = parent;
  }
  return false;
}

const SCREENS = globSync(["app/**/*.tsx", "features/**/*.tsx", "components/**/*.tsx"], {
  cwd: APP,
}).map((relative) => join(APP, relative));

describe("no screen says anything in its own words", () => {
  /** The file list is asserted before the files are, because a glob that
   *  matches nothing passes every check it is given, and a moved folder is
   *  exactly how that happens quietly. */
  it("is reading the screens it thinks it is", () => {
    // One named file per glob, not one count over all three. A count only
    // fails when the largest folder goes: dropping `features/**` left
    // thirteen files and `components/**` left nine, and both were above
    // any threshold worth writing. The tests lens called the number
    // theatre on 2026-09-20 and it was right.
    const under = (folder: string) => SCREENS.filter((file) => file.includes(`/${folder}/`));
    expect(under("app").map((file) => file.endsWith("pair.tsx"))).toContain(true);
    expect(under("features").map((file) => file.endsWith("PayPanel.tsx"))).toContain(true);
    expect(under("components").map((file) => file.endsWith("Field.tsx"))).toContain(true);
  });

  it.each(SCREENS.map((file) => [file.slice(APP.length + 1), file]))("%s", (_name, file) => {
    expect(literalsIn(file)).toEqual([]);
  });
});
