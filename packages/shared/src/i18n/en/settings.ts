// The phone's own settings: who is signed in, which till it is paired to,
// and unpairing.

import type { Messages } from "../message";

export const settings = {
  settings_title: "This phone",
  settings_signed_in_as: "Signed in as",
  settings_nobody: "nobody",
  settings_till_computer: "Till computer",
  settings_paired_as: "Paired as",
  settings_not_paired: "not paired",
  settings_queue_waiting: "Sales waiting to be sent",
  settings_unpair_would_lose:
    "Unpairing now would leave those sales unsent. Get back on the shop Wi-Fi and send them from the till screen first.",
  settings_forget_phone: "Forget this phone",
} as const satisfies Messages;
