// The phone's own settings: who is signed in, which till it is paired to,
// and unpairing.

import type { Messages } from "../message";

export const settings = {
  settings_title: "Ce téléphone",
  settings_signed_in_as: "Connecté en tant que",
  settings_nobody: "personne",
  settings_till_computer: "Ordinateur de caisse",
  settings_paired_as: "Appairé sous le nom",
  settings_not_paired: "non appairé",
  settings_queue_waiting: "Ventes en attente d'envoi",
  settings_unpair_would_lose:
    "Désappairer maintenant laisserait ces ventes non envoyées. Revenez d'abord sur le Wi-Fi du magasin et envoyez-les depuis l'écran de caisse.",
  settings_forget_phone: "Oublier ce téléphone",
} as const satisfies Messages;
