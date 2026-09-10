// Contract: the error envelope is the one shape every refusal crosses in, so
// the client can turn any failure into a code the UI translates. The figures
// are optional and never null, and each is an exact integer: a credit limit
// JSON.parse had to round would be shown to a cashier as the amount they are
// over by.

import { describe, expect, test } from "vitest";

import type { ApiErrorDto } from "../generated/ApiErrorDto";
import { apiErrorPayloadSchema, apiErrorSchema } from "./error";

const refusal: ApiErrorDto = {
  error: {
    code: "credit_limit",
    message: "the sale would take the customer past the limit",
    balance_after_centimes: 1_250_000,
    credit_limit_centimes: 1_000_000,
  },
};

describe("apiErrorPayloadSchema", () => {
  test("takes a code and a message with no figures at all", () => {
    expect(apiErrorPayloadSchema.safeParse({ code: "not_found", message: "gone" }).success).toBe(
      true,
    );
  });

  test("refuses a payload with no message", () => {
    expect(apiErrorPayloadSchema.safeParse({ code: "not_found" }).success).toBe(false);
  });

  test("refuses an amount JSON.parse had to round", () => {
    const rounded = { code: "credit_limit", message: "over", credit_limit_centimes: 1.5 };
    expect(apiErrorPayloadSchema.safeParse(rounded).success).toBe(false);
  });
});

describe("apiErrorSchema", () => {
  test("takes the envelope the API promises, figures and all", () => {
    expect(apiErrorSchema.parse(refusal)).toEqual(refusal);
  });

  test("refuses a payload sent without the envelope around it", () => {
    expect(apiErrorSchema.safeParse(refusal.error).success).toBe(false);
    expect(apiErrorSchema.safeParse(null).success).toBe(false);
  });
});
