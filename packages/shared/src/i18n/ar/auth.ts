// Signing in on a paired phone, and signing out again.

import type { Messages } from "../message";

export const auth = {
  auth_title: "من يسجّل الدخول؟",
  auth_staff_loading: "قراءة قائمة الموظفين…",
  auth_staff_unreadable: "تعذّرت قراءة قائمة الموظفين.",
  auth_nobody_yet: "لا أحد يستطيع تسجيل الدخول في هذا المحل بعد.",
  auth_by_password: "كلمة المرور",
  auth_pin_label: "الرمز السري",
  auth_password_label: "كلمة المرور",
  auth_password_placeholder: "••••••••",
  auth_sign_in: "تسجيل الدخول",
  auth_not_you: "لست أنت",
  auth_sign_out: "تسجيل الخروج",
  auth_refused: "لم يتم تسجيل دخولك.",
} as const satisfies Messages;
