// The till screen: what is for sale, the basket, and paying.

import type { Messages } from "../message";

export const till = {
  till_queue_waiting: {
    one: "{count} sale waiting to be sent",
    other: "{count} sales waiting to be sent",
  },
  till_queue_other_signin: "{count} under another sign-in",
  till_queue_send_now: "Send now",
  till_change_due: "Change: {amount} {currency}",
  till_products_unreadable: "Could not read what is for sale. Check the Wi-Fi, then pull to refresh.",
  till_products_empty: "Nothing for sale yet, add products on the till computer.",
  till_add_product: "Add {name}",

  till_basket_empty: "Basket empty",
  till_basket_count: {
    one: "{count} item",
    other: "{count} items",
  },
  till_basket_no_total: "—",
  till_basket_clear: "Clear",
  till_no_settings:
    "No total yet, the shop settings have not loaded, so the phone cannot tell whether a price tag includes the TVA.",
  till_tendered_exact: "Exact",
  till_pay_cash: "Pay cash",
  till_tendered_short: "Less than the amount to pay.",
  till_tendered_label: "Tendered",
  till_tendered_placeholder: "Amount handed over",

  till_settings: "Settings",
} as const satisfies Messages;
