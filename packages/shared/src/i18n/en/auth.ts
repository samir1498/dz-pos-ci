// Signing in on a paired phone, and signing out again.

import type { Messages } from "../message";

export const auth = {
  auth_title: "Who is signing in?",
  auth_staff_loading: "Reading the staff list…",
  auth_staff_unreadable: "Could not read the staff list.",
  auth_nobody_yet: "Nobody can sign in on this shop yet.",
  auth_by_password: "password",
  auth_pin_label: "PIN",
  auth_password_label: "Password",
  auth_password_placeholder: "••••••••",
  auth_sign_in: "Sign in",
  auth_not_you: "Not you",
  auth_sign_out: "Sign out",
  auth_refused: "That did not sign you in.",
} as const satisfies Messages;
