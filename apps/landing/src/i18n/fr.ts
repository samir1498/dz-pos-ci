import type { Dict } from "./types";

export const fr: Dict = {
  lang: "fr",
  dir: "ltr",
  title: "Dinar POS",
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
      body: "Le client demande une facture ? Un bouton au moment de la vente, et elle sort numérotée dans la série de l'année, avec les mentions obligatoires et le nom du client.",
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

  staff: {
    heading: "Un compte pour chaque personne du magasin",
    intro:
      "Un magasin qui a plusieurs personnes au comptoir n'a pas besoin qu'elles voient toutes la même chose. Chacune ouvre sa propre session, et ce qu'elle peut faire dépend de son rôle : propriétaire, gérant ou caissier.",
    // docs/features.md §5: "A cashier signs in at the till with a PIN and
    // everyone else with a name and a password"; the PIN itself is four to
    // six digits (same section, "PIN and password").
    signIn: "Le caissier ouvre sa session au comptoir avec un code de quatre à six chiffres ; les autres, avec un nom et un mot de passe.",
    // crates/core/src/services/permissions.rs `can`: Sell is the only
    // permission a Cashier holds; SeeCostAndMargin (closes the purchase and
    // supplier routes outright) and CommitMoney (expenses) and SeeReports
    // (the cash figures, checked against crates/api/src/gates.rs's rows for
    // /dashboard, /expenses and /cash) are Owner | Manager only.
    cashierScope: "Le caissier vend, encaisse et imprime le ticket. Rien de plus : ni le coût d'un article, ni la marge faite dessus, ni les fournisseurs, ni les dépenses, ni la caisse du jour.",
    // crates/core/src/services/permissions.rs: `discount_needs_permission`
    // lets a discount at or below the shop's threshold through with no
    // permission at all, and only a discount strictly above it asks for
    // DiscountAboveThreshold (Owner | Manager); ChangePriceAtTheTill, a
    // different line price entirely, and OverrideCreditBlock are both
    // Owner | Manager only with no such threshold.
    cashierLimits: "Il peut donner une remise jusqu'au seuil fixé par le magasin ; au-delà, il faut le propriétaire ou le gérant. Il ne peut jamais taper un autre prix sur la ligne, ni dépasser la limite de crédit d'un client.",
    // crates/core/src/services/permissions.rs `can`: every Permission
    // variant is Owner | Manager except ManageUsers and SeeAuditLog, which
    // are Owner only.
    manager: "Le gérant fait tout ce que fait le propriétaire, sauf gérer le personnel et lire le journal d'audit.",
    // crates/core/src/services/audit.rs: ACTION_PRICE_OVERRIDE,
    // ACTION_CANCEL, ACTION_CREDIT_OVERRIDE record who did it;
    // ACTION_PERMISSION_REFUSED (permissions.rs::record_refusal) records a
    // refusal at the one seam every gated request passes through.
    log: "Un prix changé, une facture annulée, une limite de crédit dépassée : tout est noté dans le journal avec le nom de la personne. Une tentative refusée aussi, pour que le propriétaire voie qu'elle a eu lieu.",
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
    // docs/features.md §3, fiscal rules table, row "Régime fiscal": a dated
    // shop setting, `ifu` or `réel`; under IFU no document names a tax, and
    // every document keeps the regime it was issued under.
    regime: "Au forfait comme au réel : sous l'IFU aucun document ne mentionne de TVA, et chaque document garde le régime sous lequel il a été émis.",
  },

  languages: {
    heading: "Trois langues, pas une option cachée",
    // docs/features.md, Scope: "Arabic (RTL), French, English on every
    // screen and every printed document, independently selectable (UI
    // language ≠ print language)".
    body: "L'écran, le ticket et la facture existent en français, en arabe et en anglais, et chacun choisit librement dans quelle langue il travaille et dans quelle langue il imprime.",
  },

  screens: {
    heading: "L'application en images",
  },

  audience: {
    heading: "Pour qui",
    body: "Pour un commerce en Algérie qui tient encore sa caisse à la main ou sur un tableur, et qui veut garder ses factures, son stock et ses fournisseurs au même endroit.",
  },

  pricing: {
    heading: "Le prix",
    body: "Le prix n'est pas encore fixé, et tant qu'il ne l'est pas ce formulaire reste fermé. Il ouvrira ici même, sur cette page.",
    formNameLabel: "Nom",
    formPhoneLabel: "Téléphone",
    formWilayaLabel: "Wilaya",
    formSubmit: "Être recontacté",
    formDisabledNote: "Fermé jusqu'à ce que le prix soit fixé.",
  },

  // lib/site.ts DOWNLOAD_FILES: one stable filename per OS, so these
  // labels never carry a version.
  download: {
    heading: "Télécharger",
    body: "Dinar s'installe sur le poste de la caisse, en français, en anglais et en arabe. Choisissez le fichier de votre système.",
    windowsLabel: "Windows",
    windowsNote: "Installeur .exe",
    linuxLabel: "Linux",
    linuxNote: "AppImage, à rendre exécutable",
    recommended: "Pour votre système",
    unsignedNote:
      "Premières versions non signées : Windows affichera un avertissement SmartScreen. Cela disparaîtra avec le certificat.",
    allLink: "Toutes les versions",
  },
};
