// French, checked against English's key set rather than carrying one of its
// own. The file order and the spread are English's, so a key added there and
// forgotten here fails the gates instead of falling back on a screen.

import { auth } from "./auth";
import { common } from "./common";
import { errors } from "./errors";
import { pairing } from "./pairing";
import { settings } from "./settings";
import { till } from "./till";

export const fr = { ...common, ...till, ...pairing, ...auth, ...settings, ...errors };

/** The domains, unmerged, so a test can say the merge lost nothing. A key
 *  spelled twice across two files would be swallowed silently by the spread
 *  above, and the one that lost is the one nobody notices. */
export const frDomains = { common, till, pairing, auth, settings, errors };
