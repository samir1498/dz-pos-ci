// Pairing a phone to the till computer, by camera or by typed code.

import type { Messages } from "../message";

export const pairing = {
  pairing_title: "اقتران هذا الهاتف",
  pairing_hint: "اطلب من مسؤول أن يعرض الرمز على جهاز الصندوق، ثم وجّه الكاميرا نحوه.",
  pairing_default_device_name: "هاتف",
  pairing_code_refused: "هذا الرمز لم ينفع. اطلب رمز QR جديدا، فهو لا يدوم إلا دقيقة.",
  pairing_not_a_code: "هذا ليس رمز اقتران خاصا بـ Dinar.",
  pairing_camera_hint: "تقرأ الكاميرا الرمز من شاشة جهاز الصندوق. لا يُصوَّر شيء ولا يُحفظ.",
  pairing_use_camera: "استعمال الكاميرا",
  pairing_camera_blocked:
    "الكاميرا مطفأة لهذا التطبيق في إعدادات الهاتف. أعد تشغيلها هناك، أو اكتب الرمز بدل ذلك.",
  pairing_code_label: "الرمز من QR",
  pairing_code_placeholder: {
    zero: "{count} حرف",
    one: "حرف واحد",
    two: "حرفان",
    few: "{count} أحرف",
    many: "{count} حرفا",
    other: "{count} حرف",
  },
  pairing_device_name_label: "اسم هذا الهاتف",
  pairing_submit: "اقتران",
  pairing_in_progress: "جاري الاقتران…",
  pairing_switch_to_camera: "استعمال الكاميرا بدل ذلك",
  pairing_switch_to_code: "كتابة الرمز بدل ذلك",
  pairing_lost: "أُلغي اقتران هذا الهاتف هنا. اطلب رمز QR جديدا من مسؤول.",
} as const satisfies Messages;
