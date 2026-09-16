// Who this phone is and who is acting on it, held above the router so the
// auth gate can read it.
//
// The device and the person are kept apart on purpose. A session idles out
// after fifteen minutes and the phone is still the same paired phone, so the
// cashier types a PIN and carries on; only a revoke kills the device, and
// only then does someone have to find a manager for a fresh QR. They shared
// one storage blob until the M6+M7 review, which would have demanded a QR
// every quarter of an hour.

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import {
  clearDevice,
  clearSession,
  loadDevice,
  loadSession,
  saveDevice,
  saveSession,
  type Session,
} from "../lib/session";

type SessionState = {
  /** The paired phone's token, or null if this phone has never paired. */
  device: string | null;
  /** The signed-in person, or null between sign-ins. */
  session: Session | null;
  /** False until both have been read off disk. The gate renders nothing
   * until then, so nobody sees a flash of the pairing screen. */
  ready: boolean;
  /** Why the person is looking at a sign-in screen they did not ask for. */
  lost: string | null;
  paired: (deviceToken: string) => Promise<void>;
  signedIn: (session: Session) => Promise<void>;
  signOut: () => Promise<void>;
  sessionLost: (message: string) => Promise<void>;
  deviceLost: (message: string) => Promise<void>;
};

const SessionContext = createContext<SessionState | null>(null);

export function useSession(): SessionState {
  const value = useContext(SessionContext);
  if (value === null) throw new Error("useSession outside SessionProvider");
  return value;
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const [device, setDevice] = useState<string | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [ready, setReady] = useState(false);
  const [lost, setLost] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      const [d, s] = await Promise.all([loadDevice(), loadSession()]);
      setDevice(d);
      setSession(s);
      setReady(true);
    })();
  }, []);

  const paired = useCallback(async (deviceToken: string) => {
    await saveDevice(deviceToken);
    setDevice(deviceToken);
    setLost(null);
  }, []);

  const signedIn = useCallback(async (next: Session) => {
    await saveSession(next);
    setDevice(next.deviceToken);
    setSession(next);
    setLost(null);
  }, []);

  const signOut = useCallback(async () => {
    await clearSession();
    setSession(null);
    setLost(null);
  }, []);

  const sessionLost = useCallback(async (message: string) => {
    await clearSession();
    setSession(null);
    setLost(message);
  }, []);

  const deviceLost = useCallback(async (message: string) => {
    await clearSession();
    await clearDevice();
    setSession(null);
    setDevice(null);
    setLost(message);
  }, []);

  const value = useMemo(
    () => ({ device, session, ready, lost, paired, signedIn, signOut, sessionLost, deviceLost }),
    [device, session, ready, lost, paired, signedIn, signOut, sessionLost, deviceLost],
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}
