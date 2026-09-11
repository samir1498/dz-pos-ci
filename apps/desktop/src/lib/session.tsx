// Who is signed in on this window, held in memory and nowhere else.
//
// A launch of the desktop app starts signed out every time: nothing here is
// written to `localStorage` or any other store, so a machine left on
// overnight shows the PIN pad again in the morning rather than resuming
// whoever last touched it. The one case that resumes without a screen is a
// browser tab that still carries the httpOnly cookie the server set
// (`crates/api/src/session.rs`); that is the server's memory, not this
// file's, and it is what lets `just e2e` sign in once per run instead of
// once per spec.
//
// The role never reaches a screen as a comparison. Today the only screen
// that reads it at all is the topbar's `UserMenu`, and it reads it as a
// single value — a lookup into a label, `ROLE_LABEL[me.role]`, the same
// shape `ThemeSwitcher` uses for a theme name — never a branch on which
// role it is; `role.test.ts` holds every file under `src/` to that, the
// way `theme.test.ts` holds them to never branching on a theme. `GET
// /auth/me` and `POST /auth/login` both hand back `permissions` too
// (architecture.md rule 2), untouched, for the day a screen needs to gate
// something finer than a label on it; nothing reads that field yet.
//
// Locking is a client-side idea and does not touch the server session: the
// idle timer here only raises a flag `__root.tsx` reads to show the lock
// overlay on top of the shell, which is what keeps a cart on the till alive
// while the screen is covered (`routes/till.tsx` holds the cart in the
// route's own state, and a route that stayed mounted keeps it).

import { ApiError } from "@dzpos/shared";
import type { LoginDto, MeDto } from "@dzpos/shared";
import { focusManager, useQueryClient } from "@tanstack/react-query";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { api } from "@/api";

/** How the person now signed in got in, remembered from the call that
 * succeeded rather than worked out from who they are: nothing stops a shop
 * handing the same person both a PIN and a password, so this is not the
 * role and it is not a permission either. The lock screen reads it to offer
 * the same door back. */
export type AuthMethod = "pin" | "password";

type Status = "checking" | "signed-out" | "signed-in";

interface SessionAnswer {
  readonly me: MeDto;
  readonly token: string;
  readonly idle_minutes: number;
}

interface Ctx {
  readonly status: Status;
  readonly me: MeDto | null;
  readonly method: AuthMethod | null;
  readonly idleMinutes: number | null;
  readonly locked: boolean;
  readonly signInWithPin: (userId: number, pin: string) => Promise<void>;
  readonly signInWithPassword: (name: string, password: string) => Promise<void>;
  readonly signOut: () => Promise<void>;
  readonly lockNow: () => void;
}

const SessionContext = createContext<Ctx | null>(null);

/** What counts as the till being touched, for the idle timer. Not a click
 * that lands on a button (`keydown`/`pointerdown` already cover the pad and
 * the keyboard both), and never an API call succeeding: a screen quietly
 * polling in the background must not be what keeps a session from locking,
 * or the lock would never come down on an idle machine that still refreshes
 * its own dashboard every minute. */
const ACTIVITY_EVENTS = ["pointerdown", "keydown"] as const;

export function SessionProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<Status>("checking");
  const [me, setMe] = useState<MeDto | null>(null);
  const [method, setMethod] = useState<AuthMethod | null>(null);
  const [idleMinutes, setIdleMinutes] = useState<number | null>(null);
  const [locked, setLocked] = useState(false);
  const idleTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const forgetSession = useCallback(() => {
    api.setSession(null);
    setMe(null);
    setMethod(null);
    setIdleMinutes(null);
    setLocked(false);
    setStatus("signed-out");
    // Every screen's query answered against the person who just left;
    // holding them would show the next person a stale basket total or,
    // worse, someone else's customer search.
    queryClient.clear();
  }, [queryClient]);

  // The one check a fresh window makes, once. A browser tab still carrying
  // the httpOnly cookie resumes without a screen, the way any cookie-backed
  // site does; the desktop webview carries no cookie and `GET /auth/me`
  // answers `session_required`, which is the sign-in screen appearing on
  // every launch, as the brief asks for.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const answer = await api.me();
        const idle = await api.sessionIdle();
        if (cancelled) return;
        setMe(answer);
        setIdleMinutes(idle.idle_minutes);
        setStatus("signed-in");
      } catch {
        if (!cancelled) setStatus("signed-out");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const establish = useCallback(
    (session: SessionAnswer, how: AuthMethod) => {
      api.setSession(session.token);
      setMe(session.me);
      setMethod(how);
      setIdleMinutes(session.idle_minutes);
      setLocked(false);
      setStatus("signed-in");
      // `clear()`, not `invalidateQueries()`: an invalidated query still
      // renders its last answer while the refetch is in flight, and
      // `staleTime: 30_000` (`main.tsx`) means a query that was already
      // fresh does not even refetch on its own for half a minute. Either
      // way a cashier signing in right after an owner, on the same
      // machine, would be served the owner's cached rows for a while
      // without the server ever being asked — the audit log screen is
      // where that stopped being hypothetical. `clear()` drops the data
      // itself, not just its freshness, so the next read has nothing
      // stale to render and has to ask. Paid for on every unlock too
      // (`establish` is the same call), not only a fresh sign-in: nothing
      // here can tell an unlock apart from a different person sitting
      // down, and a wasted refetch is a far smaller cost than getting
      // that distinction wrong.
      queryClient.clear();
    },
    [queryClient],
  );

  const signInWithPin = useCallback(
    async (userId: number, pin: string) => {
      const session = await api.login({ user_id: userId, pin } satisfies LoginDto);
      establish(session, "pin");
    },
    [establish],
  );

  const signInWithPassword = useCallback(
    async (name: string, password: string) => {
      const session = await api.login({ name, password } satisfies LoginDto);
      establish(session, "password");
    },
    [establish],
  );

  const signOut = useCallback(async () => {
    try {
      await api.logout();
    } finally {
      forgetSession();
    }
  }, [forgetSession]);

  const lockNow = useCallback(() => {
    setLocked(true);
  }, []);

  // Revalidated on focus (the brief's own words): a window left in the
  // background whose session ended elsewhere, another till signing the same
  // person out or the idle time running out on the server, finds out the
  // moment it is looked at again. `GET /auth/me` does not slide the idle
  // time forward (auth.rs), so looking does not itself keep a session
  // alive, and a network hiccup here is left alone rather than read as a
  // sign-out: only the server's own refusal ends the session.
  useEffect(() => {
    if (status !== "signed-in") return;
    function revalidate() {
      api.me().then(
        (answer) => setMe(answer),
        (error: unknown) => {
          if (error instanceof ApiError && error.code === "session_required") forgetSession();
        },
      );
    }
    function onVisibility() {
      if (document.visibilityState === "visible") revalidate();
    }
    window.addEventListener("focus", revalidate);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("focus", revalidate);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [status, forgetSession]);

  // While locked, tell TanStack Query the window is not focused, so no
  // screen under the overlay refetches when the OS hands this window focus
  // back (`refetchOnWindowFocus` is on at its default, `main.tsx` sets
  // nothing else). Every one of those requests carries the session
  // credential and `session::resolve` (`crates/api/src/session.rs`) slides
  // `last_seen_at` on every one that reaches it, so a till left locked with
  // a window that keeps getting focus never timed out on the server even
  // though the glass stayed covered. `setFocused(false)` is a manual
  // override that sticks regardless of which real event fires next (this
  // version of the library drives it off `visibilitychange`, not a `focus`
  // event, and the override wins either way); `setFocused(undefined)` on
  // unlock hands the decision back to the library's own default.
  //
  // Two things this does not touch, deliberately:
  //
  // - The unlock call. `signInWithPin` / `signInWithPassword` below call
  //   `api.login` directly, a plain fetch through `@dzpos/shared`'s client
  //   and never a react-query query or mutation, so `focusManager` saying
  //   the window is blurred has nothing to say about it — a query's
  //   `refetchOnWindowFocus` decides whether a mount auto-refetches, not
  //   whether a call may be made. Same for the revalidate-on-focus check
  //   just above (`api.me()`): it is a plain `window.addEventListener`
  //   handler, not a query, and `GET /auth/me` does not slide the idle
  //   clock either, so it is left running while locked on purpose — a
  //   session ended elsewhere is still worth finding out about from behind
  //   the lock screen. `session.test.tsx` proves the login call still
  //   answers while this effect has told the manager the window is blurred.
  // - The idle timer below. It is a client-side `setTimeout` armed and
  //   cleared by `pointerdown` / `keydown` on `window`, never by query
  //   focus state, so a blurred focus manager cannot stop it noticing a
  //   typed PIN: the keystrokes that answer the lock screen are the same
  //   ones that would have reset the timer had it still been running, and
  //   the effect below already stops arming it the moment `locked` is true.
  useEffect(() => {
    focusManager.setFocused(locked ? false : undefined);
  }, [locked]);

  // The idle timer. Counted from the shop's own figure, the same one the
  // lock screen would show if it asked again, reset by a pointer or a key
  // and by nothing else. It runs only while signed in and not already
  // locked: a locked screen has done its job and a signed-out one has
  // nothing to time.
  useEffect(() => {
    if (status !== "signed-in" || locked || idleMinutes === null) return;
    // Captured in a local: a nested function closes over the binding, not
    // the narrowing, so TypeScript sees `idleMinutes` as still nullable
    // inside `schedule` without this.
    const minutes = idleMinutes;
    function schedule() {
      if (idleTimer.current !== null) clearTimeout(idleTimer.current);
      idleTimer.current = setTimeout(() => setLocked(true), minutes * 60_000);
    }
    schedule();
    for (const name of ACTIVITY_EVENTS) window.addEventListener(name, schedule);
    return () => {
      for (const name of ACTIVITY_EVENTS) window.removeEventListener(name, schedule);
      if (idleTimer.current !== null) clearTimeout(idleTimer.current);
    };
  }, [status, locked, idleMinutes]);

  const value = useMemo<Ctx>(
    () => ({
      status,
      me,
      method,
      idleMinutes,
      locked,
      signInWithPin,
      signInWithPassword,
      signOut,
      lockNow,
    }),
    [status, me, method, idleMinutes, locked, signInWithPin, signInWithPassword, signOut, lockNow],
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession(): Ctx {
  const ctx = useContext(SessionContext);
  if (ctx === null) throw new Error("useSession outside SessionProvider");
  return ctx;
}
