// The till screen: what is for sale, the basket, and paying.

import type { Messages } from "../message";

export const till = {
  till_queue_waiting: {
    zero: "لا عمليات بيع في انتظار الإرسال",
    one: "عملية بيع واحدة في انتظار الإرسال",
    two: "عمليتا بيع في انتظار الإرسال",
    few: "{count} عمليات بيع في انتظار الإرسال",
    many: "{count} عملية بيع في انتظار الإرسال",
    other: "{count} عملية بيع في انتظار الإرسال",
  },
  till_queue_other_signin: "{count} تحت تسجيل دخول آخر",
  till_queue_send_now: "أرسل الآن",
  till_change_due: "الباقي: {amount} {currency}",
  till_products_unreadable: "تعذّرت قراءة ما هو معروض للبيع. تحقّق من Wi-Fi ثم اسحب للتحديث.",
  till_products_empty: "لا شيء للبيع بعد، أضف منتجات على جهاز الصندوق.",
  till_add_product: "أضف {name}",

  till_basket_empty: "السلة فارغة",
  till_basket_count: {
    zero: "لا أصناف",
    one: "صنف واحد",
    two: "صنفان",
    few: "{count} أصناف",
    many: "{count} صنفًا",
    other: "{count} صنف",
  },
  till_basket_no_total: "—",
  till_basket_clear: "تفريغ",
  till_no_settings:
    "لا يوجد مجموع بعد، إعدادات المحل لم تُحمَّل، فلا يستطيع الهاتف أن يعرف هل السعر المعروض يشمل الرسم على القيمة المضافة.",
  till_tendered_exact: "بالضبط",
  till_pay_cash: "الدفع نقدا",
  till_tendered_short: "أقل من المبلغ المطلوب.",
  till_tendered_label: "المبلغ المدفوع",
  till_tendered_placeholder: "المبلغ المسلَّم",

  till_settings: "الإعدادات",
} as const satisfies Messages;
