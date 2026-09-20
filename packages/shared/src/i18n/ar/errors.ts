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
  error_validation: "جهاز الصندوق رفض هذا. تحقّق ممّا أدخلته.",
  error_not_found: "لم يعد هذا موجودا. اطلب من مسؤول تحديث الصندوق.",
  error_duplicate_barcode: "هناك منتج آخر يحمل هذا الباركود.",
  error_sale_already_rung: "هذه البيعة تمت من قبل. لا تعدها، تحقق من حاسوب الصندوق.",
  error_exhausted: "نفدت أرقام هذه الوثيقة. أبلغ مسؤولا.",
  error_credit_limit: "هذه البيعة تتجاوز حدّ الدين المسموح به للزبون.",
  error_party_ids: "ينقص هذا الزبون معرّف تطلبه الفاتورة.",
  error_locked_out: "محاولات خاطئة كثيرة. انتظر قليلا ثم أعد المحاولة.",
  error_till_problem: "حدث خلل في جهاز الصندوق. أبلغ مسؤولا.",
} as const satisfies Messages;
