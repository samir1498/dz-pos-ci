// The dashboard and the chart behind it.

import type { DashboardDto } from "../generated/DashboardDto";
import type { DashboardSeriesDto } from "../generated/DashboardSeriesDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { dashboardSchema, dashboardSeriesSchema } from "../schemas/dashboard";

export function dashboardClient({ send }: Transport) {
  return {
    /** The whole dashboard for one day and the month it falls in. The day is
     * the shop's today when the caller names none: the server reads the same
     * clock the services date documents with, so a screen that sent nothing
     * and one that sent what `/clock` gave it get the same answer. */
    async dashboard(day?: string): Promise<DashboardDto> {
      const suffix = day === undefined ? "" : `?${new URLSearchParams({ day }).toString()}`;
      return narrow(await send(`/dashboard${suffix}`), dashboardSchema, "dashboard");
    },

    /** The chart behind the dashboard: the last `days` days ending on `day`,
     * each on its own and folded into weeks. Both default the way the screen
     * reads them, the shop's today and thirty days, and the server refuses a
     * window of nothing or of more than a year. */
    async dashboardSeries(window?: { day?: string; days?: number }): Promise<DashboardSeriesDto> {
      const query = new URLSearchParams();
      if (window?.day !== undefined) query.set("day", window.day);
      if (window?.days !== undefined) query.set("days", String(window.days));
      const suffix = query.toString() === "" ? "" : `?${query.toString()}`;
      return narrow(
        await send(`/dashboard/series${suffix}`),
        dashboardSeriesSchema,
        "dashboard series",
      );
    },
  };
}
