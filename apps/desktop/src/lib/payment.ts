// The two ways a payment can go out: cash or card. A customer pays the shop
// this way, the shop pays a supplier this way, and the shop pays a supplier
// for a purchase order this way too, so the pair lived three times
// (`routes/-customers/parts.tsx`, `routes/suppliers.tsx`, and
// `routes/purchases_.new.tsx`) until the suppliers screen split into a
// folder and went looking for the other two. One home for all three.

import type { PaymentMethodDto } from "@dzpos/shared";

import type { Key } from "@/i18n";

export const PAYMENT_METHODS: readonly PaymentMethodDto[] = ["cash", "card"];

export const PAYMENT_METHOD_KEY: Readonly<Record<PaymentMethodDto, Key>> = {
  cash: "payment_cash",
  card: "payment_card",
};
