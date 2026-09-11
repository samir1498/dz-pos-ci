import type { Dict } from "./types";

export const en: Dict = {
  lang: "en",
  dir: "ltr",
  title: "Dinar POS",
  brandWord: "Dinar",
  brandSuffix: "POS",
  tagline: "The till, the invoice, the stock and the suppliers for an Algerian shop, with nothing to subscribe to at the counter.",

  hero: {
    heading: "The till that prints the invoice and keeps the stock straight.",
    body: "No monthly subscription, no internet needed at the counter: everything runs on the shop's own computer.",
    cta: "Get in touch",
  },

  pieces: {
    heading: "What it does",
    sell: {
      title: "Sell",
      body: "The cashier adds the items, takes cash, card or credit, and the ticket is out in seconds.",
    },
    invoice: {
      title: "Invoice",
      body: "The customer asks for an invoice? One button at the moment of the sale, and it comes out numbered in the year's series, with the required details and the customer's name.",
    },
    stock: {
      title: "Stock",
      body: "Every sale and every delivery updates the stock, and the home screen flags an item the moment it starts running low.",
    },
    know: {
      title: "Know",
      body: "The dashboard answers at a glance: today's sales, what customers still owe, and what is owed to suppliers.",
    },
  },

  fiscal: {
    heading: "What the tax office asks for, already built in",
    intro: "What an Algerian shop owner asks a till first, before anything else:",
    // docs/features.md §3, fiscal rules table, row "Numbering": "one
    // uninterrupted chronological series per document kind and per year...
    // numbers never reused".
    numbering: "Every invoice carries a number in that year's series, with no gap and no repeat.",
    // docs/features.md §3, fiscal rules table, row "Droit de timbre": "cash
    // only... the whole amount at its band's rate".
    stamp: "A cash payment carries the stamp duty, worked out automatically from the amount.",
    // docs/features.md §3, fiscal rules table, row "TVA rates": "19 %
    // standard, 9 % reduced, 0 % exempt; rate per product".
    tva: "TVA is calculated per rate, line by line: 19 %, 9 % or 0 % depending on the product.",
    // docs/features.md §3, fiscal rules table, row "Amount in words":
    // "French, Arabic and English generators, dinars and centimes".
    words: "The amount due is also spelled out in words, in French, Arabic and English.",
    // docs/features.md §3, fiscal rules table, row "Régime fiscal": a dated
    // shop setting, `ifu` or `réel`; under IFU no document names a tax, and
    // every document keeps the regime it was issued under.
    regime: "Flat-rate or standard regime: under IFU no document names a tax, and every document keeps the regime it was issued under.",
  },

  languages: {
    heading: "Three languages, not an afterthought",
    // docs/features.md, Scope: "Arabic (RTL), French, English on every
    // screen and every printed document, independently selectable (UI
    // language ≠ print language)".
    body: "The screen, the ticket and the invoice exist in French, Arabic and English, and each is chosen freely: the language to work in and the language to print in.",
  },

  screens: {
    heading: "The app, in pictures",
  },

  audience: {
    heading: "Who it's for",
    body: "For a shop in Algeria still keeping its books by hand or in a spreadsheet, wanting its invoices, its stock and its suppliers in one place.",
  },

  pricing: {
    heading: "Price",
    body: "The price is not set yet, and until it is this form stays closed. It will open here, on this page.",
    formNameLabel: "Name",
    formPhoneLabel: "Phone",
    formWilayaLabel: "Wilaya",
    formSubmit: "Ask to be contacted",
    formDisabledNote: "Closed until the price is set.",
  },
};
