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

/** One item off the store, as a request this build can resend, or null.
 *
 *  Checked rather than asserted, and per item rather than for the list as
 *  a whole: a queue is a cashier's unsent sales, so one entry written by
 *  an older build should cost that entry and not the rest. An entry whose
 *  `url` or `body` came back undefined would be posted as the string
 *  "undefined" to a path that does not exist, and the retry would keep
 *  earning the refusal that this module was written to stop looping on.
 *
 *  A missing `sessionToken` is filled in as null rather than refused.
 *  The field arrived with the Expo SDK 57 rebuild (838e4ee), so a sale
 *  queued before that has no such key at all, and null is what the type
 *  already means by it: queued before sessions existed, so `retry` sends
 *  it under whoever is signed in now instead of skipping it for ever.
 *  Refusing it would drop a rung sale silently, and leaving the key
 *  undefined would fail `retry`'s null check and strand it in `skipped`
 *  with nothing to clear it. */
function queuedFrom(value: unknown): QueuedRequest | null {
  if (typeof value !== "object" || value === null) return null;
  if (!("id" in value) || typeof value.id !== "string") return null;
  if (!("method" in value) || typeof value.method !== "string") return null;
  if (!("url" in value) || typeof value.url !== "string") return null;
  if (!("body" in value) || !(typeof value.body === "string" || value.body === null)) return null;
  if (!("createdAt" in value) || typeof value.createdAt !== "string") return null;
  const signer: unknown = "sessionToken" in value ? value.sessionToken : null;
  if (!(typeof signer === "string" || signer === null)) return null;
  return {
    id: value.id,
    method: value.method,
    url: value.url,
    body: value.body,
    createdAt: value.createdAt,
    sessionToken: signer,
  };
}

/** Drops the warm copy so the next read goes to the store again. For the
 *  tests, the same seam and the same reason as `forgetCachedSession`: a
 *  check that reads back what this process just wrote proves the cache,
 *  not the disk, and would go on passing with the write deleted. */
export function forgetCachedQueue(): void {
  memory = null;
}

const STORAGE_KEY = "dzpos:retry-queue";

/** Where an entry this build cannot read is put instead of being thrown
 *  away. `load` drops what it cannot resend and the next `save` writes
 *  the shorter list back, so without this the bytes of a cashier's
 *  unsent sale are gone from the phone and there is nothing for a
 *  support bundle to find. Nothing reads this key; it is a bin with a
 *  lid, and it is the only trace a drop leaves. */
const UNREADABLE_KEY = "dzpos:retry-queue:unreadable";

let memory: QueuedRequest[] | null = null;

async function load(): Promise<QueuedRequest[]> {
  if (memory !== null) return memory;
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const { default: AsyncStorage } = await import(
      "@react-native-async-storage/async-storage"
    );
    const raw = await AsyncStorage.getItem(STORAGE_KEY);
    const parsed: unknown = raw === null ? null : JSON.parse(raw);
    const rows: unknown[] = Array.isArray(parsed) ? parsed : [];
    const unreadable = rows.filter((row) => queuedFrom(row) === null);
    if (unreadable.length > 0) await quarantine(AsyncStorage, unreadable);
    memory = rows.map(queuedFrom).filter((req): req is QueuedRequest => req !== null);
    return memory;
  } catch {
    memory = [];
    return [];
  }
}

/** Appends, never replaces: a second unreadable entry on a later read
 *  must not erase the first one. */
async function quarantine(
  storage: { getItem: (key: string) => Promise<string | null>; setItem: (key: string, value: string) => Promise<void> },
  rows: unknown[],
): Promise<void> {
  const raw = await storage.getItem(UNREADABLE_KEY);
  const held: unknown = raw === null ? null : JSON.parse(raw);
  await storage.setItem(UNREADABLE_KEY, JSON.stringify([...(Array.isArray(held) ? held : []), ...rows]));
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
