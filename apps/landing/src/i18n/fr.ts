import type { Dict } from "./types";

export const fr: Dict = {
  lang: "fr",
  dir: "ltr",
  title: "dz-pos",
  brandWord: "Dinar",
  brandSuffix: "POS",
  tagline: "La caisse, la facture, le stock et les fournisseurs d'un commerce algérien, sans abonnement au comptoir.",

  hero: {
    heading: "La caisse qui imprime la facture et tient le stock à jour.",
    body: "Pas d'abonnement mensuel, pas besoin d'internet au comptoir : tout tourne sur l'ordinateur du magasin.",
    cta: "Nous contacter",
  },

  pieces: {
    heading: "Ce qu'il fait",
    sell: {
      title: "Vendre",
      body: "Le vendeur ajoute les articles, encaisse en espèces, en carte ou à crédit, et le ticket sort du comptoir en quelques secondes.",
    },
    invoice: {
      title: "Facturer",
      body: "D'un geste, le ticket devient une facture numérotée quand le client la réclame, avec son numéro et les mentions légales déjà en place.",
    },
    stock: {
      title: "Stock",
      body: "Chaque vente et chaque livraison met à jour le stock, et l'écran d'accueil prévient dès qu'un article commence à manquer.",
    },
    know: {
      title: "Savoir",
      body: "Le tableau de bord répond en un coup d'œil sur les ventes du jour, ce qui reste dû par les clients et ce qui est dû aux fournisseurs.",
    },
  },

  fiscal: {
    heading: "Ce que demande le fisc, déjà en place",
    intro: "Ce qu'un commerçant algérien demande en premier à sa caisse, avant tout le reste :",
    // docs/features.md §3, fiscal rules table, row "Numbering": "one
    // uninterrupted chronological series per document kind and per year...
    // numbers never reused".
    numbering: "Chaque facture porte un numéro dans la série de l'année, sans trou et sans doublon.",
    // docs/features.md §3, fiscal rules table, row "Droit de timbre": "cash
    // only... the whole amount at its band's rate".
    stamp: "Un paiement en espèces porte le droit de timbre, calculé automatiquement selon le montant.",
    // docs/features.md §3, fiscal rules table, row "TVA rates": "19 %
    // standard, 9 % reduced, 0 % exempt; rate per product".
    tva: "La TVA est calculée par taux, ligne par ligne : 19 %, 9 % ou 0 % selon le produit.",
    // docs/features.md §3, fiscal rules table, row "Amount in words":
    // "French, Arabic and English generators, dinars and centimes".
    words: "Le montant à payer est aussi écrit en toutes lettres, en français, en arabe et en anglais.",
  },

  languages: {
    heading: "Trois langues, pas une option cachée",
    // docs/features.md, Scope: "Arabic (RTL), French, English on every
    // screen and every printed document, independently selectable (UI
    // language ≠ print language)".
    body: "L'écran, le ticket et la facture existent en français, en arabe et en anglais, et chacun choisit librement dans quelle langue il travaille et dans quelle langue il imprime.",
  },

  audience: {
    heading: "Pour qui",
    body: "Pour un commerce en Algérie qui tient encore sa caisse à la main ou sur un tableur, et qui veut garder ses factures, son stock et ses fournisseurs au même endroit.",
  },

  pricing: {
    heading: "Le prix",
    body: "Le prix n'est pas encore fixé. Laissez vos coordonnées, nous vous recontactons dès qu'il l'est.",
    formNameLabel: "Nom",
    formPhoneLabel: "Téléphone",
    formWilayaLabel: "Wilaya",
    formSubmit: "Être recontacté",
  },
};
