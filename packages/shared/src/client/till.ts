// The till's shifts: opening a drawer with a float, counting it at close, and
// reading either one back (routes/till.rs; plan
// till-shifts-a-float-and-a-count).
//
// Neither call below takes a moment. `NewShiftDto::opening_cash_centimes` and
// `TillCountDto::counted_centimes` are the whole of what each body carries;
// the server stamps `opened_at` and the close's own moment on the way in, the
// way `dto/till.rs`'s own doc explains, and a body naming either is refused
// outright rather than quietly dropped.

import type { NewShiftDto } from "../generated/NewShiftDto";
import type { ShiftDto } from "../generated/ShiftDto";
import type { ShiftReportDto } from "../generated/ShiftReportDto";
import type { TillCountDto } from "../generated/TillCountDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { shiftReportSchema, shiftSchema } from "../schemas/till";

export function tillClient({ send }: Transport) {
  return {
    /** Opens the caller's own drawer with what is in it. The opener is
     * never on the body — it is the caller's own identity, like every
     * other write — and neither is the moment. */
    async openShift(input: NewShiftDto): Promise<ShiftDto> {
      const body = await send("/till/shifts", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, shiftSchema, "shift");
    },

    /** Counts the drawer and closes it. The expected figure is the core's,
     * worked out at the moment of closing and never sent from here: a
     * caller that could supply it could record a clean evening over a
     * short drawer. */
    async closeShift(id: number, input: TillCountDto): Promise<ShiftDto> {
      const body = await send(`/till/shifts/${id}/close`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, shiftSchema, "shift");
    },

    /** The caller's own open drawer with its live figures, or `null` when
     * none is open. Ungated on the server: it answers about the caller and
     * nobody else, which is what lets the close screen show the expected
     * figure before a cashier who holds no reporting permission counts it. */
    async getOpenShift(): Promise<ShiftReportDto | null> {
      return narrow(
        await send("/till/shifts/open"),
        shiftReportSchema.nullable(),
        "open shift",
      );
    },

    /** One shift by id, with the figures a screen puts beside it. Gated on
     * `see_reports` by the server: a cashier's own figures come from
     * `getOpenShift` before the close and from `closeShift`'s own answer
     * after it, never from here. */
    async getShift(id: number): Promise<ShiftReportDto> {
      return narrow(await send(`/till/shifts/${id}`), shiftReportSchema, "shift report");
    },
  };
}
