// The support bundle (M5 T3).

import type { Download } from "../client-response";
import type { Transport } from "../client";

export function supportClient({ sendFile }: Transport) {
  return {
    /** The support bundle (M5 T3): a zip carrying the session log, the
     * build's own version and migration history, the shape of the schema, a
     * few counts and the machine's OS, language and time zone. No customer,
     * no product, no price and no document is in it; `dzpos_core::services::
     * support_bundle`'s own doc names the whole list and the test that holds
     * it. */
    async supportBundle(): Promise<Download> {
      return sendFile("/support-bundle");
    },
  };
}
