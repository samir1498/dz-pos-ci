/** Who this phone is and who is acting on it (M7 T3).
 *
 * Three credentials ride every guarded call, each proving one thing: the
 * launch token (build-time env, the server operator's secret) says the
 * caller may reach the server at all; the device token (from pairing) says
 * which phone; the session token (from sign-in) says which person. This
 * store holds the two the phone learns at runtime. `AsyncStorage` like the
 * retry queue, with the same in-memory fallback when native is absent; a
 * hardware-backed store is tracked open, not done here.
 */

export type Session = {
  deviceToken: string;
  sessionToken: string;
  name: string;
  role: string;
};

const STORAGE_KEY = "dzpos:session";

let memory: Session | null | undefined;

async function store() {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { default: AsyncStorage } = await import(
    "@react-native-async-storage/async-storage"
  );
  return AsyncStorage;
}

export async function saveSession(session: Session): Promise<void> {
  memory = session;
  try {
    await (await store()).setItem(STORAGE_KEY, JSON.stringify(session));
  } catch {
    // in-memory fallback already updated
  }
}

export async function loadSession(): Promise<Session | null> {
  if (memory !== undefined) return memory;
  try {
    const raw = await (await store()).getItem(STORAGE_KEY);
    memory = raw ? (JSON.parse(raw) as Session) : null;
    return memory ?? null;
  } catch {
    memory = null;
    return null;
  }
}

export async function clearSession(): Promise<void> {
  memory = null;
  try {
    await (await store()).removeItem(STORAGE_KEY);
  } catch {
    // in-memory fallback already updated
  }
}

/** Reset the in-memory cache only; tests use this to prove the store round trip. */
export function forgetCachedSession(): void {
  memory = undefined;
}

// ---- the device, kept apart from the person ----
//
// The two credentials die for different reasons and must be forgotten
// separately. A session idles out every 15 minutes
// (`DEFAULT_SESSION_IDLE_MINUTES`); the phone is still the same paired
// phone, so the cashier types their PIN again and carries on. Only a
// revoke kills the device, and only then does someone have to find a
// manager for a fresh QR. Keeping both in one blob meant every idle-out
// demanded a re-pair — found in the M6+M7 review, 2026-09-16.

const DEVICE_KEY = "dzpos:device";

let deviceMemory: string | null | undefined;

export async function saveDevice(deviceToken: string): Promise<void> {
  deviceMemory = deviceToken;
  try {
    await (await store()).setItem(DEVICE_KEY, deviceToken);
  } catch {
    // in-memory fallback already updated
  }
}

export async function loadDevice(): Promise<string | null> {
  if (deviceMemory !== undefined) return deviceMemory;
  try {
    deviceMemory = (await (await store()).getItem(DEVICE_KEY)) ?? null;
    return deviceMemory;
  } catch {
    deviceMemory = null;
    return null;
  }
}

/** Forget the phone itself: only a revoke calls this. */
export async function clearDevice(): Promise<void> {
  deviceMemory = null;
  try {
    await (await store()).removeItem(DEVICE_KEY);
  } catch {
    // in-memory fallback already updated
  }
}

/** Reset the in-memory cache only; tests use this to prove the store round trip. */
export function forgetCachedDevice(): void {
  deviceMemory = undefined;
}
