// What a refusal from the shop's core says to the person holding the phone.
// One sentence each, and none of them names a status code except the one
// that has nothing better to say.

import type { Messages } from "../message";

export const errors = {
  error_sign_in_again: "انتهت الجلسة، سجّل الدخول من جديد.",
  error_pair_again: "لم يعد هذا الهاتف مقترنا. اطلب رمز QR جديدا من مسؤول.",
  error_wrong_secret: "هذا ليس الرمز السري ولا كلمة المرور الصحيحة.",
  error_not_allowed: "غير مسموح لك بالقيام بهذا.",
  error_refused: "مرفوض ({status}).",
  error_no_answer: "لا جواب من جهاز الصندوق. تحقّق من Wi-Fi وأعد المحاولة.",
} as const satisfies Messages;
