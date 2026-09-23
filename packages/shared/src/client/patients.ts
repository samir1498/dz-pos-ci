// The clinic's patient file (C3, C3b). Behind the `clinic` feature server
// side and behind the same build flag on the desktop (C6); the client
// itself carries no switch, since a build that never mounts these screens
// never calls it.

import { z } from "zod";

import type { PatientDto } from "../generated/PatientDto";
import type { PatientWriteDto } from "../generated/PatientWriteDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { patientSchema } from "../schemas/patient";

export function patientsClient({ send }: Transport) {
  return {
    /** The shop's patients, active ones first. `search` is a piece of a
     * name or a phone; blank asks for the whole (live) list. `archived`
     * brings the archived files back into the answer beside the live ones,
     * which the archive room reads. */
    async listPatients(search?: string, archived = false): Promise<PatientDto[]> {
      const trimmed = search === undefined ? "" : search.trim();
      const params = new URLSearchParams();
      if (trimmed !== "") params.set("q", trimmed);
      if (archived) params.set("archived", "true");
      const query = params.size === 0 ? "" : `?${params.toString()}`;
      return narrow(await send(`/patients${query}`), z.array(patientSchema), "patient list");
    },

    async getPatient(id: string): Promise<PatientDto> {
      return narrow(await send(`/patients/${id}`), patientSchema, "patient");
    },

    async createPatient(input: PatientWriteDto): Promise<PatientDto> {
      const body = await send("/patients", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, patientSchema, "patient");
    },

    /** The whole file again, not a patch. `notes` always travels: `null`
     * clears it, and a caller without `ViewPatientNotes` may only ever send
     * `null` there (a real value is refused with 403 before anything is
     * written, and the service keeps the stored notes for that caller
     * regardless of what else changed). */
    async updatePatient(id: string, input: PatientWriteDto): Promise<PatientDto> {
      const body = await send(`/patients/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, patientSchema, "patient");
    },

    /** Takes the file out of the search; still readable by id. A second
     * archive is a 409. */
    async archivePatient(id: string): Promise<PatientDto> {
      const body = await send(`/patients/${id}/archive`, { method: "POST" });
      return narrow(body, patientSchema, "patient");
    },
  };
}
