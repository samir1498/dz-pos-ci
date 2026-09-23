// The clinic's waiting queue (C4). One day at a time, in arrival order.

import { z } from "zod";

import type { QueueEntryDto } from "../generated/QueueEntryDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { queueEntrySchema } from "../schemas/queue";

export function queueClient({ send }: Transport) {
  return {
    /** Today's queue on the shop's clock, in arrival order. */
    async listQueue(): Promise<QueueEntryDto[]> {
      return narrow(await send("/queue"), z.array(queueEntrySchema), "queue");
    },

    async addToQueue(patientId: string): Promise<QueueEntryDto> {
      const body = await send("/queue", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ patient_id: patientId }),
      });
      return narrow(body, queueEntrySchema, "queue entry");
    },

    /** The earliest arrival not yet called. Refused with 409 when the line
     * is empty. */
    async callNextInQueue(): Promise<QueueEntryDto> {
      const body = await send("/queue/next", { method: "POST" });
      return narrow(body, queueEntrySchema, "queue entry");
    },

    /** One entry called in out of order. */
    async callInQueue(id: string): Promise<QueueEntryDto> {
      const body = await send(`/queue/${id}/call`, { method: "POST" });
      return narrow(body, queueEntrySchema, "queue entry");
    },

    async markSeenInQueue(id: string): Promise<QueueEntryDto> {
      const body = await send(`/queue/${id}/seen`, { method: "POST" });
      return narrow(body, queueEntrySchema, "queue entry");
    },

    async markLeftInQueue(id: string): Promise<QueueEntryDto> {
      const body = await send(`/queue/${id}/left`, { method: "POST" });
      return narrow(body, queueEntrySchema, "queue entry");
    },
  };
}
