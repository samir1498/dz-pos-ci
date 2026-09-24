// What every "Imprimer" test needs and cannot get from the DOM alone:
// proof that a print dialog was actually asked for, on the sheet the
// cashier was looking at, not only that a preview showed up.
//
// `window.print()` takes no argument a test could inspect, so the stub
// below records what its own document held at the moment it ran. A render
// race that opened the dialog on a blank or half-loaded frame would show
// up here as a missing or wrong string even though the preview looked
// fine a moment later.

import type { Page } from "@playwright/test";

/**
 * Replaces `window.print` in every document this page ever navigates,
 * including the sandboxed receipt frame: `addInitScript` is injected by
 * the browser itself ahead of a document's own script and does not need
 * `allow-scripts` on the frame it lands in, which the receipt frame never
 * carries. Call this before the page navigates anywhere.
 */
export async function stubPrint(page: Page): Promise<void> {
  await page.addInitScript(() => {
    window.print = () => {
      const el = document.documentElement;
      const count = Number(el.dataset.printedCount ?? "0") + 1;
      el.dataset.printedCount = String(count);
      el.dataset.printed = document.body.innerText;
    };
  });
}
