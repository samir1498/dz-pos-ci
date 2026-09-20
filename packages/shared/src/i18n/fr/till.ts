// The till screen: what is for sale, the basket, and paying.

import type { Messages } from "../message";

export const till = {
  till_queue_waiting: {
    one: "{count} vente en attente d'envoi",
    many: "{count} ventes en attente d'envoi",
    other: "{count} ventes en attente d'envoi",
  },
  till_queue_other_signin: "{count} sous une autre connexion",
  till_queue_send_now: "Envoyer maintenant",
  till_change_due: "Rendu : {amount} {currency}",
  till_products_unreadable:
    "Impossible de lire ce qui est en vente. Vérifiez le Wi-Fi, puis tirez pour rafraîchir.",
  till_products_empty: "Rien en vente pour l'instant, ajoutez des produits sur l'ordinateur de caisse.",
  till_add_product: "Ajouter {name}",

  till_basket_empty: "Panier vide",
  till_basket_count: {
    one: "{count} article",
    many: "{count} articles",
    other: "{count} articles",
  },
  till_basket_no_total: "—",
  till_basket_clear: "Vider",
  till_no_settings:
    "Pas encore de total, les paramètres du magasin ne sont pas chargés, le téléphone ne peut donc pas savoir si un prix affiché comprend la TVA.",
  till_tendered_exact: "Exact",
  till_pay_cash: "Encaisser en espèces",
  till_tendered_short: "Moins que le montant à payer.",
  till_tendered_label: "Montant reçu",
  till_tendered_placeholder: "Montant remis",

  till_settings: "Paramètres",
} as const satisfies Messages;
