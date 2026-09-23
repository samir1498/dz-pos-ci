// The clinic's waiting queue (C4). One day at a time, in the desk's order
// (C6b).

import { z } from "zod";

import type { QueueEntryDto } from "../generated/QueueEntryDto";
import type { QueueOrderDto } from "../generated/QueueOrderDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { queueEntrySchema } from "../schemas/queue";

export function queueClient({ send }: Transport) {
  return {
    /** Today's queue on the shop's clock, in the desk's order. */
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

    /** The first of the desk's order not yet called. Refused with 409 when
     * the line is empty. */
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

    /** Puts today's queue in the order the desk dragged it into (C6b): the
     * entry ids, first place first. Answers the whole day in its new
     * order. */
    async reorderQueue(input: QueueOrderDto): Promise<QueueEntryDto[]> {
      const body = await send("/queue/order", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, z.array(queueEntrySchema), "queue");
    },

    /** Marks a booked patient arrived (C6b): today's queue gets an entry
     * naming the appointment. Asked twice, it answers the same entry. */
    async checkInAppointment(appointmentId: string): Promise<QueueEntryDto> {
      const body = await send(`/appointments/${appointmentId}/arrive`, { method: "POST" });
      return narrow(body, queueEntrySchema, "queue entry");
    },
  };
}
