// Pairing a phone to the till computer, by camera or by typed code.

import type { Messages } from "../message";

export const pairing = {
  pairing_title: "Pair this phone",
  pairing_hint: "Ask a manager to show the code on the till computer, then point the camera at it.",
  pairing_default_device_name: "Phone",
  pairing_code_refused: "That code did not work. Ask for a fresh QR, they last a minute.",
  pairing_not_a_code: "That is not a Dinar pairing code.",
  pairing_camera_hint: "The camera reads the code off the till computer's screen. Nothing is photographed or kept.",
  pairing_use_camera: "Use the camera",
  pairing_camera_blocked:
    "The camera is switched off for this app in the phone's settings. Turn it back on there, or type the code instead.",
  pairing_code_label: "Code from the QR",
  pairing_code_placeholder: {
    one: "{count} character",
    other: "{count} characters",
  },
  pairing_device_name_label: "Name for this phone",
  pairing_submit: "Pair",
  pairing_in_progress: "Pairing…",
  pairing_switch_to_camera: "Use the camera instead",
  pairing_switch_to_code: "Type the code instead",
  pairing_lost: "This phone was unpaired here. Ask a manager for a fresh QR.",
} as const satisfies Messages;
