// Signing in on a paired phone, and signing out again.

import type { Messages } from "../message";

export const auth = {
  auth_title: "Qui se connecte ?",
  auth_staff_loading: "Lecture de la liste du personnel…",
  auth_staff_unreadable: "Impossible de lire la liste du personnel.",
  auth_nobody_yet: "Personne ne peut encore se connecter dans ce magasin.",
  auth_by_password: "mot de passe",
  auth_pin_label: "Code PIN",
  auth_password_label: "Mot de passe",
  auth_password_placeholder: "••••••••",
  auth_sign_in: "Se connecter",
  auth_not_you: "Ce n'est pas vous",
  auth_sign_out: "Se déconnecter",
  auth_refused: "La connexion n'a pas abouti.",
} as const satisfies Messages;
