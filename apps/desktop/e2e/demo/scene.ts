// What every demo scene shares: the signed-in `test` the suite uses, a
// pause a viewer can read, and the one place a finished recording is put.
//
// Playwright writes each scene's video under test-results/ with a name it
// invents; `keepClip` copies it, once the page is closed and the file is
// whole, to `demo-clips/<scene>.webm` next to the e2e folder (gitignored),
// which is the name `just demo-clips` converts and hands to the Remotion
// project. The scene is the spec's own file name without its number, so
// `02-the-ticket.spec.ts` becomes `the-ticket.webm`.

import { mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import type { Page, TestInfo } from "@playwright/test";

export { expect, test } from "../auth";

const here = fileURLToPath(new URL(".", import.meta.url));
export const CLIPS_DIR = path.join(here, "..", "..", "demo-clips");

/** A viewer's pause, in milliseconds. A test would never wait for nothing;
 * a demo waits so the eye lands on what just happened. */
export const BEAT = 1_200;

export async function beat(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(BEAT * times);
}

/** The scene's clip name from the spec file: `01-ring-a-sale.spec.ts` gives
 * `ring-a-sale`. */
export function sceneName(testInfo: TestInfo): string {
  return path.basename(testInfo.file).replace(/^\d+-/, "").replace(/\.spec\.ts$/, "");
}

/** Close the page (the video is only complete once it is) and put the clip
 * under its scene name. Call it as the last line of a scene. */
export async function keepClip(page: Page, testInfo: TestInfo): Promise<void> {
  const video = page.video();
  await page.close();
  if (video === null) throw new Error("the demo project records video; this run did not");
  mkdirSync(CLIPS_DIR, { recursive: true });
  await video.saveAs(path.join(CLIPS_DIR, `${sceneName(testInfo)}.webm`));
}
