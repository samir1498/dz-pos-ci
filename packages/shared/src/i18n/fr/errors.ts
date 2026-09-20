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
  error_validation: "L'ordinateur de caisse a refusé cela. Vérifiez ce que vous avez saisi.",
  error_not_found: "Cela n'existe plus. Demandez à un responsable d'actualiser la caisse.",
  error_duplicate_barcode: "Un autre produit a déjà ce code-barres.",
  error_sale_already_rung: "Cette vente est déjà passée. Ne la refaites pas, vérifiez sur l'ordinateur de la caisse.",
  error_exhausted: "Les numéros de ce document sont épuisés. Prévenez un responsable.",
  error_credit_limit: "Cette vente dépasserait la limite de crédit du client.",
  error_party_ids: "Il manque à ce client un identifiant exigé par une facture.",
  error_locked_out: "Trop d'essais incorrects. Patientez un instant et réessayez.",
  error_till_problem: "Un problème est survenu sur l'ordinateur de caisse. Prévenez un responsable.",
} as const satisfies Messages;
