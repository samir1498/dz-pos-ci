// Handing a file to the person at the till.
//
// There is no save dialog to reach: the Tauri shell wires no dialog or fs
// plugin (`src-tauri/capabilities/default.json` grants `core:default` and
// nothing else), so a file leaves through the webview the way it does in a
// browser, as an object URL behind an anchor the code clicks. Adding the
// plugin would widen what the window may touch on the shop's disk, which is
// a permission decision and not this task's to take. See the report.
//
// The name comes from the server (`content-disposition`, exposed by the CORS
// layer), so the workbook a shop finds in its downloads folder is named the
// same whether it was saved from the desktop or from a browser preview.

/** Hands `blob` to the browser as a download called `filename`.
 *
 * The object URL is revoked on the next tick rather than immediately: some
 * webviews start the download asynchronously and a URL revoked in the same
 * frame gives an empty file. */
export function saveBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
