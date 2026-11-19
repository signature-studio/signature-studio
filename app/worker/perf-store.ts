// The beacon's R2 object (decision 0224): one per beacon, the body is one JSONL line {t, remote: "workers", perf}, the key is shared/perf.ts perfObjectKey. Nothing else is kept: not the IP address, the Access identity
// (Cf-Access-Authenticated-User-Email), any cookie or any header. A failed write is logged (the Worker's log) and reported as "not stored": the page does not depend on the beacon.
import { perfLine, perfObjectKey } from '../shared/perf.ts';

/** the part of R2Bucket this file uses (R2Bucket satisfies it; a test can pass a few lines of its own) */
export interface PerfPutBucket {
  put(key: string, value: string, options?: { httpMetadata?: { contentType?: string } }): Promise<unknown>;
}

export async function storeBeacon(bucket: PerfPutBucket | undefined, value: Record<string, unknown>, now: Date = new Date()): Promise<boolean> {
  if (!bucket) return false;
  const key = perfObjectKey(value, now, crypto.randomUUID().replaceAll('-', '').slice(0, 8));
  try {
    await bucket.put(key, perfLine(value, now, 'workers'), { httpMetadata: { contentType: 'application/json' } });
    return true;
  } catch (e) {
    console.error(`the beacon ${key} was not stored: ${String((e as Error)?.message ?? e)}`);
    return false;
  }
}
