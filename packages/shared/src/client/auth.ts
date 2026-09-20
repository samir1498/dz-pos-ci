// Signing in, signing up the first shop's owner, and the small reads a
// signed-in session asks for: who is signed in, how long it survives idle,
// and the names a sign-in screen may offer. `client.ts` keeps `setSession`
// and `logout`: both touch the session token the client itself holds, and
// nothing here does.

import { z } from "zod";

import type { ClaimFirstOwnerDto } from "../generated/ClaimFirstOwnerDto";
import type { LoginDto } from "../generated/LoginDto";
import type { MeDto } from "../generated/MeDto";
import type { SessionDto } from "../generated/SessionDto";
import type { SessionIdleDto } from "../generated/SessionIdleDto";
import type { StaffDto } from "../generated/StaffDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { meSchema, sessionIdleSchema, sessionSchema } from "../schemas/session";
import { staffSchema } from "../schemas/staff";

export function authClient({ send }: Transport) {
  return {
    /** Signs in with a user id and a PIN, or a name and a password.
     *
     * The token is returned and deliberately not remembered: a browser got
     * the same session as an httpOnly cookie, and holding the token in a
     * variable JavaScript can read would hand back exactly what httpOnly was
     * for. The desktop, whose webview cannot set a cookie, calls
     * `setSession(answer.token)` after this (T4). */
    async login(body: LoginDto): Promise<SessionDto> {
      return narrow(
        await send("/auth/login", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        }),
        sessionSchema,
        "sign-in answer",
      );
    },

    /** The one door into a shop nobody has ever signed into: a name and a
     * password. The server finds the shop's own owner, writes those, then
     * signs them in the same way `login` does. The till PIN is set later
     * from the users screen. Refuses once any credential anywhere in the
     * shop already exists. */
    async claimFirstOwner(body: ClaimFirstOwnerDto): Promise<SessionDto> {
      return narrow(
        await send("/auth/first-setup", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        }),
        sessionSchema,
        "sign-in answer",
      );
    },

    /** Who is signed in. Raises `session_required` when nobody is, which is
     * what the desktop revalidates on focus against. */
    async me(): Promise<MeDto> {
      return narrow(await send("/auth/me"), meSchema, "session answer");
    },

    /** How long a session survives with nothing happening on it. */
    async sessionIdle(): Promise<SessionIdleDto> {
      return narrow(await send("/auth/idle"), sessionIdleSchema, "idle answer");
    },

    /** The names a sign-in screen offers before anyone is signed in
     * (`GET /auth/staff`): a cashier taps one and types only a PIN. Inside
     * the device gate and outside the session one, like `login`, so it
     * answers a signed-out desktop on loopback and a paired phone on the
     * LAN, and nobody else. */
    async listStaff(): Promise<StaffDto[]> {
      return narrow(await send("/auth/staff"), z.array(staffSchema), "staff list");
    },
  };
}
