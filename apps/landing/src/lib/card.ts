// The Open Graph / Twitter card: the coin, the wordmark and the tagline on
// Comptoir's own surface, generated from the design package the way the
// product shots are (see src/lib/shots.ts and its own comment). Committed as
// public/og/card.png; `pnpm run card` (DZPOS_CARD_WRITE=1 vitest run
// tests/card.test.ts) reruns it, and running this file plainly, the way
// `pnpm test` and the gates do, instead asserts the committed file still
// matches what buildCard() produces right now, so a branding or copy change
// nobody reran the script for fails the gates instead of shipping a stale
// card.
//
// The wordmark and tagline are French: fr is the default, unprefixed locale
// (astro.config.ts), and English reads the same Latin word. Arabic is not
// rendered onto this card: opentype.js walks a font's cmap and draws each
// glyph at its own advance width, with no Arabic shaping (no contextual
// forms, no ligatures), so an Arabic render through this path would not be
// the correct letterforms, only individual isolated glyphs run together. A
// card that gets the Arabic wrong is worse than one route's card carrying
// French; see the L5 report for the alternative this rejects.
//
// Text is drawn from real glyph outlines (opentype.js reading the vendored
// IBM Plex Sans .woff files), never <text> left for sharp's SVG renderer to
// resolve: that renderer asks this machine's fontconfig for "IBM Plex Sans"
// and, absent it (this box has none installed; see
// packages/design/assets/README.md's note about the wordmark SVGs having the
// same problem), silently substitutes whatever sans-serif fontconfig offers
// instead. That substitute differs by machine, so the same source would
// render different pixels in CI than on the box that committed them, and the
// byte-compare test below would fail on a machine that changed nothing.
// Glyph outlines traced from the font file itself have no such dependency.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

import sharp from "sharp";
import * as opentype from "opentype.js";

import { theme } from "@dzpos/design";

import { fr } from "../i18n/fr";

const require = createRequire(import.meta.url);

export const CARD_WIDTH = 1200;
export const CARD_HEIGHT = 630;
export const CARD_FILE = "card.png";

const PADDING = 96;
const COIN_SIZE = 200;

// A plain copy rather than a slice of buffer.buffer: Node types that view as
// ArrayBuffer | SharedArrayBuffer (a Buffer can, in principle, back onto
// either), and opentype.parse wants a concrete ArrayBuffer. A fresh
// Uint8Array always backs a plain ArrayBuffer, so this needs no cast.
const toArrayBuffer = (buffer: Buffer): ArrayBuffer => {
  const copy = new Uint8Array(buffer.byteLength);
  copy.set(buffer);
  return copy.buffer;
};

type Weight = 400 | 600 | 700;

const loadFont = (weight: Weight): opentype.Font => {
  const path = require.resolve(`@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-${weight}-normal.woff`);
  return opentype.parse(toArrayBuffer(readFileSync(path)));
};

/**
 * Throws if any character in `text` has no glyph in `font` (glyph index 0,
 * `.notdef`), so a copy edit that introduces a character IBM Plex Sans does
 * not carry (an em dash, a curly quote) fails loudly here instead of
 * rendering a blank box on the committed card.
 */
const assertGlyphsAvailable = (font: opentype.Font, text: string): void => {
  for (const char of text) {
    if (char === " ") continue;
    if (font.charToGlyph(char).index === 0) {
      throw new Error(`IBM Plex Sans has no glyph for ${JSON.stringify(char)} (from ${JSON.stringify(text)})`);
    }
  }
};

const glyphPath = (
  font: opentype.Font,
  text: string,
  x: number,
  y: number,
  fontSize: number,
  fill: string,
): string => {
  assertGlyphsAvailable(font, text);
  return `<path d="${font.getPath(text, x, y, fontSize).toPathData(2)}" fill="${fill}"/>`;
};

/**
 * Greedy word wrap using the font's own advance widths, so a line never
 * overruns `maxWidth` regardless of what a substitute face's metrics would
 * say (see the module comment on why no substitute face is ever involved
 * here, but the wrap is written to not depend on that anyway).
 */
const wrapLines = (font: opentype.Font, text: string, fontSize: number, maxWidth: number): string[] => {
  const words = text.split(" ");
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const attempt = current === "" ? word : `${current} ${word}`;
    if (current === "" || font.getAdvanceWidth(attempt, fontSize) <= maxWidth) {
      current = attempt;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current !== "") lines.push(current);
  return lines;
};

/**
 * `packages/design/assets/mark.svg`'s own shapes, scaled and read from the
 * theme instead of the asset's literal hex (that file's README explains why
 * it carries literal colour; this generator has no such exemption): one
 * stroke for two scripts, the bowl closing into a Latin D against the coin
 * edge and the foot on the baseline making it an Arabic dal.
 */
const coinMark = (x: number, y: number, size: number): string => {
  const scale = size / 100;
  const brass = theme.colors.money;
  const ink = theme.colors["on-money"];
  return (
    `<g transform="translate(${x} ${y}) scale(${scale})">` +
    `<circle cx="50" cy="50" r="46" fill="${brass}"/>` +
    `<circle cx="50" cy="50" r="40" fill="none" stroke="${ink}" stroke-width="3"/>` +
    `<path d="M40 28 A 24 24 0 0 1 40 72" fill="none" stroke="${ink}" stroke-width="11" stroke-linecap="round"/>` +
    `<path d="M40 72 L 30 72" fill="none" stroke="${ink}" stroke-width="11" stroke-linecap="round"/>` +
    `</g>`
  );
};

export interface CardBuild {
  readonly bytes: Buffer;
  readonly width: number;
  readonly height: number;
}

export const buildCard = async (): Promise<CardBuild> => {
  const bold = loadFont(700);
  const semibold = loadFont(600);
  const regular = loadFont(400);

  const bg = theme.colors.surface.bg;
  const wordColor = theme.colors.text.primary;
  const brassColor = theme.colors.money;
  const taglineColor = theme.colors.text.secondary;

  const coinX = PADDING;
  const coinY = 90;

  const wordX = coinX + COIN_SIZE + 40;
  const wordFontSize = 72;
  const wordBaselineY = coinY + COIN_SIZE / 2 + 26;
  const brandWordPath = glyphPath(bold, fr.brandWord, wordX, wordBaselineY, wordFontSize, wordColor);
  const brandSuffixX = wordX + bold.getAdvanceWidth(`${fr.brandWord} `, wordFontSize);
  const brandSuffixPath = glyphPath(semibold, fr.brandSuffix, brandSuffixX, wordBaselineY, wordFontSize, brassColor);

  const taglineFontSize = 34;
  const taglineLineHeight = 48;
  const taglineStartY = coinY + COIN_SIZE + 90;
  const taglineMaxWidth = CARD_WIDTH - PADDING * 2;
  const taglinePaths = wrapLines(regular, fr.tagline, taglineFontSize, taglineMaxWidth)
    .map((line, i) => glyphPath(regular, line, PADDING, taglineStartY + i * taglineLineHeight, taglineFontSize, taglineColor))
    .join("");

  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${CARD_WIDTH}" height="${CARD_HEIGHT}" ` +
    `viewBox="0 0 ${CARD_WIDTH} ${CARD_HEIGHT}">` +
    `<rect width="${CARD_WIDTH}" height="${CARD_HEIGHT}" fill="${bg}"/>` +
    coinMark(coinX, coinY, COIN_SIZE) +
    brandWordPath +
    brandSuffixPath +
    taglinePaths +
    `</svg>`;

  const bytes = await sharp(Buffer.from(svg)).png().toBuffer();
  const meta = await sharp(bytes).metadata();
  if (meta.width === undefined || meta.height === undefined) {
    throw new Error("card.ts: sharp returned no pixel dimensions for the rendered card");
  }
  return { bytes, width: meta.width, height: meta.height };
};
