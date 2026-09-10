// The envelope every refusal crosses in.

import { z } from "zod";

import type { ApiErrorDto } from "../generated/ApiErrorDto";
import type { ApiErrorPayloadDto } from "../generated/ApiErrorPayloadDto";
import { exactInteger } from "./common";
import type { Assert, Covers } from "./drift";

/** The figures are optional rather than nullable: the server leaves a field
 *  out instead of sending null, and absent is an answer here. Only a credit
 *  refusal carries the two amounts, only a payment above the debt carries the
 *  outstanding one. */
export const apiErrorPayloadSchema = z.object({
  code: z.string(),
  message: z.string(),
  balance_after_centimes: exactInteger.optional(),
  credit_limit_centimes: exactInteger.optional(),
  field: z.string().optional(),
  outstanding_centimes: exactInteger.optional(),
  party_side: z.string().optional(),
  missing_ids: z.array(z.string()).optional(),
}) satisfies z.ZodType<ApiErrorPayloadDto>;
type _Payload = Assert<Covers<ApiErrorPayloadDto, typeof apiErrorPayloadSchema>>;

export const apiErrorSchema = z.object({
  error: apiErrorPayloadSchema,
}) satisfies z.ZodType<ApiErrorDto>;
type _Error = Assert<Covers<ApiErrorDto, typeof apiErrorSchema>>;
