// The OG/Twitter card is generated, then committed (same reason as L1's
// shots: a visitor, and a scraper fetching the page for a link preview,
// never waits on a build to see it). `pnpm run card`
// (DZPOS_CARD_WRITE=1 vitest run tests/card.test.ts) writes public/og/card.png
// fresh; running this file plainly, the way `pnpm test` and the gates do,
// instead asserts the committed file still matches what buildCard()
// produces right now, so a branding or copy change nobody reran the script
// for fails here rather than shipping a stale card.
//
// The same shape as tests/shots.test.ts.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildCard, CARD_FILE, CARD_HEIGHT, CARD_WIDTH } from "../src/lib/card";

const PUBLIC_OG = fileURLToPath(new URL("../public/og/", import.meta.url));
const CARD_PATH = `${PUBLIC_OG}${CARD_FILE}`;

const built = await buildCard();

if (process.env.DZPOS_CARD_WRITE === "1") {
  mkdirSync(PUBLIC_OG, { recursive: true });
  writeFileSync(CARD_PATH, built.bytes);
}

describe("the committed OG card", () => {
  it(`renders at ${CARD_WIDTH}x${CARD_HEIGHT}, the size og:image:width/height declare`, () => {
    expect(built.width).toBe(CARD_WIDTH);
    expect(built.height).toBe(CARD_HEIGHT);
  });

  it("is the file buildCard() produces right now", () => {
    const committed = readFileSync(CARD_PATH);
    expect(committed.equals(built.bytes), `${CARD_FILE} is stale; run pnpm run card`).toBe(true);
  });
});
