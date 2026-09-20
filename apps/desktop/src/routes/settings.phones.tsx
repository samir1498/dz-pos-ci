// The phones paired to this till.
//
// Minting a code, listing the phones and revoking one are all `EditSettings`
// in `crates/api/src/gates/`, enforced by the session middleware on the
// route itself. The rail hides this room from a role that does not hold it;
// the server refuses it regardless.

import { createFileRoute } from "@tanstack/react-router";

import { PairedPhonesPanel } from "@/components/PairedPhonesPanel";

export const Route = createFileRoute("/settings/phones")({ component: PairedPhonesPanel });
