// The paired phones a shop trusts (M6 T2, M6 T4).
//
// The token hash never leaves the server, which is why there is no field for
// it here: the screen revokes a row by its id, and the only thing it can
// show a person is the name whoever paired the phone typed.

import { z } from "zod";

import type { DeviceTokenDto } from "../generated/DeviceTokenDto";
import type { PairedDeviceDto } from "../generated/PairedDeviceDto";
import type { PairingQrDto } from "../generated/PairingQrDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const pairingQrSchema = z.object({
  pairing_token: z.string(),
  expires_in_seconds: z.number(),
}) satisfies z.ZodType<PairingQrDto>;
type _PairingQr = Assert<Matches<PairingQrDto, typeof pairingQrSchema>>;

export const deviceTokenSchema = z.object({
  device_token: z.string(),
}) satisfies z.ZodType<DeviceTokenDto>;
type _DeviceToken = Assert<Matches<DeviceTokenDto, typeof deviceTokenSchema>>;

export const pairedDeviceSchema = z.object({
  id: exactInteger,
  name: z.string(),
  created_at: z.string(),
  // Null while the phone is still trusted. A row is never deleted: a shop
  // has to be able to say that this phone was trusted between these two
  // moments, which a missing row cannot say.
  revoked_at: z.string().nullable(),
  created_by: exactInteger,
}) satisfies z.ZodType<PairedDeviceDto>;
type _PairedDevice = Assert<Matches<PairedDeviceDto, typeof pairedDeviceSchema>>;
