// The phone's own settings: who is signed in, which till it is paired to,
// and unpairing.

import type { Messages } from "../message";

export const settings = {
  settings_title: "هذا الهاتف",
  settings_signed_in_as: "مسجّل الدخول باسم",
  settings_nobody: "لا أحد",
  settings_till_computer: "جهاز الصندوق",
  settings_paired_as: "مقترن باسم",
  settings_not_paired: "غير مقترن",
  settings_queue_waiting: "مبيعات في انتظار الإرسال",
  settings_unpair_would_lose:
    "إلغاء الاقتران الآن يترك تلك المبيعات غير مرسلة. عد أولا إلى Wi-Fi المحل وأرسلها من شاشة الصندوق.",
  settings_forget_phone: "نسيان هذا الهاتف",
} as const satisfies Messages;
