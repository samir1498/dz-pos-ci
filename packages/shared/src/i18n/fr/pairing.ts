// Pairing a phone to the till computer, by camera or by typed code.

import type { Messages } from "../message";

export const pairing = {
  pairing_title: "Appairer ce téléphone",
  pairing_hint:
    "Demandez à un responsable d'afficher le code sur l'ordinateur de caisse, puis pointez la caméra dessus.",
  pairing_default_device_name: "Téléphone",
  pairing_code_refused: "Ce code n'a pas marché. Demandez un nouveau QR, ils durent une minute.",
  pairing_not_a_code: "Ce n'est pas un code d'appairage Dinar.",
  pairing_camera_hint:
    "La caméra lit le code sur l'écran de l'ordinateur de caisse. Rien n'est photographié ni gardé.",
  pairing_use_camera: "Utiliser la caméra",
  pairing_camera_blocked:
    "La caméra est désactivée pour cette application dans les paramètres du téléphone. Réactivez-la là-bas, ou tapez le code à la place.",
  pairing_code_label: "Code du QR",
  pairing_code_placeholder: {
    one: "{count} caractère",
    many: "{count} caractères",
    other: "{count} caractères",
  },
  pairing_device_name_label: "Nom de ce téléphone",
  pairing_submit: "Appairer",
  pairing_in_progress: "Appairage…",
  pairing_switch_to_camera: "Utiliser la caméra à la place",
  pairing_switch_to_code: "Taper le code à la place",
  pairing_lost: "Ce téléphone a été désappairé ici. Demandez un nouveau QR à un responsable.",
} as const satisfies Messages;
