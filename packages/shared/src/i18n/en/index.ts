// English, the key set the other two are checked against.
//
// Not because the app is English, it is not: that is already the rule on the
// desktop (`apps/desktop/src/i18n/index.tsx`, where `isKey` is `v in en`),
// and the desktop moves onto this package later. Two sources of truth for
// the key list is the failure this shape exists to avoid.

import { auth } from "./auth";
import { common } from "./common";
import { errors } from "./errors";
import { pairing } from "./pairing";
import { settings } from "./settings";
import { till } from "./till";

export const en = { ...common, ...till, ...pairing, ...auth, ...settings, ...errors };

/** The domains, unmerged, so a test can say the merge lost nothing. A key
 *  spelled twice across two files would be swallowed silently by the spread
 *  above, and the one that lost is the one nobody notices. */
export const enDomains = { common, till, pairing, auth, settings, errors };
