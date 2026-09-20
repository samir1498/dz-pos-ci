// Pairing a phone to a shop (M6 T2): the QR the desktop shows, the phones
// that have claimed one, and revoking one.

import { z } from "zod";

import type { DeviceTokenDto } from "../generated/DeviceTokenDto";
import type { PairedDeviceDto } from "../generated/PairedDeviceDto";
import type { PairingQrDto } from "../generated/PairingQrDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { deviceTokenSchema, pairedDeviceSchema, pairingQrSchema } from "../schemas/pairing";

export function pairingClient({ send }: Transport) {
  return {
    /** The QR the desktop shows (M6 T2): 60s single-use, owner|manager only. */
    async createPairingQr(): Promise<PairingQrDto> {
      return narrow(await send("/pairing/qr", { method: "POST" }), pairingQrSchema, "pairing QR");
    },

    /** The phones this shop has paired, newest first as the server orders
     * them. Same gate as the QR that created them (`EditSettings`), so a
     * cashier's session is refused by the route, not by a hidden button. */
    async listPairedDevices(): Promise<PairedDeviceDto[]> {
      return narrow(await send("/pairing/devices"), z.array(pairedDeviceSchema), "paired phones");
    },

    /** Revoke one paired phone. The phone finds out on its next call, which
     * is a 401 it reads as "pair again" rather than as a dropped Wi-Fi. */
    async revokePairedDevice(id: number): Promise<PairedDeviceDto> {
      return narrow(
        await send(`/pairing/devices/${id}/revoke`, { method: "POST" }),
        pairedDeviceSchema,
        "revoked phone",
      );
    },

    /** The phone trades the QR's pairing token for a device token (M6 T2). */
    async claimPairing(body: { pairing_token: string; device_name: string }): Promise<DeviceTokenDto> {
      return narrow(
        await send("/pairing/claim", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        }),
        deviceTokenSchema,
        "pairing claim",
      );
    },
  };
}
