/** Retry queue for the thin client (M6 T5). The phone is a thin client with a
 * retry queue, never bidirectional sync: the desktop is the one source of
 * truth. A failed `POST /sales` (offline LAN) is enqueued locally and retried
 * when the phone is back on the shop Wi-Fi. `AsyncStorage` is the store; an
 * in-memory fallback keeps tests and web preview working without native.
 */

export type QueuedRequest = {
  id: string;
  method: string;
  url: string;
  body: string | null;
  createdAt: string;
  /** The session that queued it, or null for items from before sessions.
   * A retry under a different signer skips the item rather than ringing
   * one person's sale as another (M7 T4). */
  sessionToken: string | null;
};

/** A fresh client retry key (M7 T4): doubles as the sale's idempotency key
 * when the queued request is a ring, so the first try and every retry
 * promise the same sale. */
export function newIdempotencyKey(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

const STORAGE_KEY = "dzpos:retry-queue";

let memory: QueuedRequest[] | null = null;

async function load(): Promise<QueuedRequest[]> {
  if (memory !== null) return memory;
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const { default: AsyncStorage } = await import(
      "@react-native-async-storage/async-storage"
    );
    const raw = await AsyncStorage.getItem(STORAGE_KEY);
    memory = raw ? (JSON.parse(raw) as QueuedRequest[]) : [];
    return memory ?? [];
  } catch {
    memory = [];
    return [];
  }
}

async function save(queue: QueuedRequest[]): Promise<void> {
  memory = queue;
  try {
    const { default: AsyncStorage } = await import(
      "@react-native-async-storage/async-storage"
    );
    await AsyncStorage.setItem(STORAGE_KEY, JSON.stringify(queue));
  } catch {
    // in-memory fallback already updated
  }
}

export async function enqueue(req: Omit<QueuedRequest, "id" | "createdAt">): Promise<string> {
  const queue = await load();
  const id = newIdempotencyKey();
  queue.push({ ...req, id, createdAt: new Date().toISOString() });
  await save(queue);
  return id;
}

export async function list(): Promise<QueuedRequest[]> {
  return [...(await load())];
}

export async function remove(id: string): Promise<void> {
  const queue = await load();
  await save(queue.filter((r) => r.id !== id));
}

export async function clear(): Promise<void> {
  await save([]);
}

/** Try each queued request with `sender`; on success remove it, on failure keep it.
 * Items queued under another session are skipped, never sent: `current` is
 * the signer's session token now, and a null one signs nothing out. */
export async function retry(
  sender: (req: QueuedRequest) => Promise<boolean>,
  currentSessionToken?: string | null,
): Promise<{ succeeded: number; failed: number; skipped: number }> {
  const queue = await load();
  let succeeded = 0;
  let failed = 0;
  let skipped = 0;
  for (const req of [...queue]) {
    if (req.sessionToken !== null && req.sessionToken !== currentSessionToken) {
      skipped += 1;
      continue;
    }
    try {
      const ok = await sender(req);
      if (ok) {
        await remove(req.id);
        succeeded += 1;
      } else {
        failed += 1;
      }
    } catch {
      failed += 1;
    }
  }
  return { succeeded, failed, skipped };
}
