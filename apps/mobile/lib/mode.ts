/** Which version this phone runs (M7 onboarding). One app, two modes:
 * `till` is just the till (cashier on the floor), `full` is the owner shell.
 * Stored like the session, same AsyncStorage fallback. A cashier is always
 * `till`; an owner/manager picks once after sign-in.
 */

export type Mode = "till" | "full";

const KEY = "dzpos:mode";

let memory: Mode | null | undefined;

async function store() {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { default: AsyncStorage } = await import(
    "@react-native-async-storage/async-storage"
  );
  return AsyncStorage;
}

export async function saveMode(mode: Mode): Promise<void> {
  memory = mode;
  try {
    await (await store()).setItem(KEY, mode);
  } catch {}
}

export async function loadMode(): Promise<Mode | null> {
  if (memory !== undefined) return memory;
  try {
    const raw = await (await store()).getItem(KEY);
    memory = raw === "till" || raw === "full" ? raw : null;
    return memory;
  } catch {
    memory = null;
    return null;
  }
}

export async function clearMode(): Promise<void> {
  memory = null;
  try {
    await (await store()).removeItem(KEY);
  } catch {}
}

export function forgetCachedMode(): void {
  memory = undefined;
}
