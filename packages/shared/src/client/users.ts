// The owner's staff fiches: listing them, creating one, setting a PIN, and
// switching a fiche on or off.

import { z } from "zod";

import type { NewUserDto } from "../generated/NewUserDto";
import type { SetPinDto } from "../generated/SetPinDto";
import type { UserDto } from "../generated/UserDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { userSchema } from "../schemas/user";

export function usersClient({ send }: Transport) {
  return {
    /** The shop's staff, active first then alphabetical: the owner's own
     * read (M4 T8). */
    async listUsers(): Promise<UserDto[]> {
      return narrow(await send("/users"), z.array(userSchema), "user list");
    },

    /** A fiche, name and role. No credential yet: `setUserPin` is what
     * gives it a PIN, the first one or a reset alike. */
    async createUser(input: NewUserDto): Promise<UserDto> {
      const body = await send("/users", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, userSchema, "user");
    },

    /** Gives a fiche its first PIN or resets a forgotten one; the server
     * does not tell the two apart and neither does this. Never answers with
     * the PIN it replaces, because there is not one to show. */
    async setUserPin(id: number, input: SetPinDto): Promise<UserDto> {
      const body = await send(`/users/${id}/pin`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, userSchema, "user");
    },

    /** Switches a fiche off. The last-owner and self refusals are the
     * server's, enforced on the row. */
    async deactivateUser(id: number): Promise<UserDto> {
      const body = await send(`/users/${id}/deactivate`, { method: "POST" });
      return narrow(body, userSchema, "user");
    },

    /** Switches a fiche back on. */
    async reactivateUser(id: number): Promise<UserDto> {
      const body = await send(`/users/${id}/reactivate`, { method: "POST" });
      return narrow(body, userSchema, "user");
    },
  };
}
