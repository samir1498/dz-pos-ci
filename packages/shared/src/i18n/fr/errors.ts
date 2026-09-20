// What a refusal from the shop's core says to the person holding the phone.
// One sentence each, and none of them names a status code except the one
// that has nothing better to say.

import type { Messages } from "../message";

export const errors = {
  error_sign_in_again: "Session expirée, reconnectez-vous.",
  error_pair_again: "Ce téléphone n'est plus appairé. Demandez un nouveau QR à un responsable.",
  error_wrong_secret: "Ce n'est pas le bon code PIN ni le bon mot de passe.",
  error_not_allowed: "Vous n'êtes pas autorisé à faire cela.",
  error_refused: "Refusé ({status}).",
  error_no_answer: "Pas de réponse de l'ordinateur de caisse. Vérifiez le Wi-Fi et réessayez.",
} as const satisfies Messages;
