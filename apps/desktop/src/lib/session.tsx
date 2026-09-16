// Who is signed in on this window, held in memory and nowhere else,
// except in dev (`vite --host`), where the last session is mirrored to
// `localStorage` (`DEV_STORAGE_KEY`) so a refresh keeps you signed in.
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
// The role never reaches a screen as a comparison. The topbar's `UserMenu`
// reads it as a single value — a lookup into a label, `ROLE_LABEL[me.role]`,
// the same shape `ThemeSwitcher` uses for a theme name — never a branch on
// which role it is; `role.test.ts` holds every file under `src/` to that,
// the way `theme.test.ts` holds them to never branching on a theme. `GET
// /auth/me` and `POST /auth/login` both hand back `permissions` too
// (architecture.md rule 2), and `useHasPermission` below is the one place a
// screen asks it: a boolean composes into a filtered column list or a
// hidden block, which a `Can` wrapper component does not (M4 T5).
//
// `useHasPermission` reads this context and not a second fetch of `GET
// /auth/me`: `apps/desktop/src/lib/me.ts` used to keep its own react-query
// cache under the key `["me"]`, fetched the same endpoint a second time, and
// had exactly one caller (`AppShell`'s sidebar). This file already resolves
// `/auth/me` once per window and every other screen that needs "who is
// signed in" reads it, so the second fetch was deleted rather than kept
// alongside a fourth way to ask (M4 T5).
//
// Locking is a client-side idea and does not touch the server session: the
// idle timer here only raises a flag `__root.tsx` reads to show the lock
// overlay on top of the shell, which is what keeps a cart on the till alive
// while the screen is covered (`routes/till.tsx` holds the cart in the
// route's own state, and a route that stayed mounted keeps it).

import { ApiError } from "@dzpos/shared";
import type { LoginDto, MeDto, PermissionDto } from "@dzpos/shared";
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

type Status = "checking" | "needs-setup" | "signed-out" | "signed-in";

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
  readonly claimFirstOwner: (name: string, password: string) => Promise<void>;
  readonly signOut: () => Promise<void>;
  readonly lockNow: () => void;
}

const SessionContext = createContext<Ctx | null>(null);

// In dev (`vite --host`) a refresh wiping the in-memory session is just
// friction — prod (Tauri) has no refresh and must start signed out for the
// overnight-PIN-pad rule, so this is dev-only.
const DEV_PERSIST = import.meta.env.DEV;
const DEV_STORAGE_KEY = "dzpos:dev-session";

function loadDevSession(): { token: string; me: MeDto; idle: number; method: AuthMethod } | null {
  if (!DEV_PERSIST || typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(DEV_STORAGE_KEY);
    if (raw === null) return null;
    const parsed = JSON.parse(raw) as { token?: unknown; me?: unknown; idle?: unknown; method?: unknown };
    if (typeof parsed.token !== "string" || parsed.me === null || typeof parsed.me !== "object") return null;
    return parsed as never;
  } catch {
    return null;
  }
}

function saveDevSession(token: string, me: MeDto, idle: number, method: AuthMethod) {
  if (!DEV_PERSIST || typeof window === "undefined") return;
  try {
    window.localStorage.setItem(DEV_STORAGE_KEY, JSON.stringify({ token, me, idle, method }));
  } catch {}
}

function clearDevSession() {
  if (!DEV_PERSIST || typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(DEV_STORAGE_KEY);
  } catch {}
}

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
    clearDevSession();
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
    // Dev-only: resume the last session so a refresh does not drop you at
    // the sign-in screen. The `/auth/me` check below still revalidates it,
    // and a refusal there forgets it (storage included).
    const dev = loadDevSession();
    if (dev !== null) {
      api.setSession(dev.token);
      setMe(dev.me);
      setMethod(dev.method);
      setIdleMinutes(dev.idle);
      setStatus("signed-in");
    }
    void (async () => {
      try {
        const health = await api.health();
        if (cancelled) return;
        if (health.needs_first_setup) {
          setStatus("needs-setup");
          return;
        }
      } catch {
        // A failed probe is not a sign-in; /auth/me still decides.
      }
      try {
        const answer = await api.me();
        const idle = await api.sessionIdle();
        if (cancelled) return;
        setMe(answer);
        setIdleMinutes(idle.idle_minutes);
        setStatus("signed-in");
      } catch {
        if (cancelled) return;
        // A refused restore is not a session: drop the stored copy too, or
        // every refresh would flash signed-in before falling back out.
        if (dev !== null) forgetSession();
        else setStatus("signed-out");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [forgetSession]);

  const establish = useCallback(
    (session: SessionAnswer, how: AuthMethod) => {
      api.setSession(session.token);
      saveDevSession(session.token, session.me, session.idle_minutes, how);
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

  const claimFirstOwner = useCallback(
    async (name: string, password: string) => {
      const session = await api.claimFirstOwner({ name, password });
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
    // The manager is a module-level singleton and outlives this component.
    // Nothing unmounts the provider in the app, but an override left behind
    // by one that did would decide, for the whole process, whether every
    // later query refetches on focus.
    return () => focusManager.setFocused(undefined);
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
      claimFirstOwner,
      signOut,
      lockNow,
    }),
    [
      status,
      me,
      method,
      idleMinutes,
      locked,
      signInWithPin,
      signInWithPassword,
      claimFirstOwner,
      signOut,
      lockNow,
    ],
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession(): Ctx {
  const ctx = useContext(SessionContext);
  if (ctx === null) throw new Error("useSession outside SessionProvider");
  return ctx;
}

/** Whether `me` holds `permission`. `false` for every refusal `me` can be
 *  (nothing signed in yet, or signed out), not only "the role does not
 *  hold it": a screen that cannot even tell who is signed in offers
 *  nothing that needs asking. A plain function and not a hook, so the one
 *  place that filters a list of items against several permissions
 *  (`AppShell`'s `NAV`) can call it once per item without breaking the
 *  rule that a hook is called the same number of times on every render;
 *  `useHasPermission` below is the hook shape for everywhere else, and
 *  both read `.permissions` in this one place, which is what
 *  `role.test.ts` holds every other file under `src/` to never doing. */
export function hasPermission(me: MeDto | null, permission: PermissionDto): boolean {
  return me?.permissions.includes(permission) ?? false;
}

/** `hasPermission`, read off the signed-in session. `__root.tsx` mounts the
 *  shell only once `status === "signed-in"`, so every screen that reaches
 *  this hook already has `me`; the null case is for the hook itself, not a
 *  frame a mounted screen would see. */
export function useHasPermission(permission: PermissionDto): boolean {
  const { me } = useSession();
  return hasPermission(me, permission);
}
