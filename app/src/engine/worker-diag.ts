// Decision 0189 (ship1, PROVISIONAL): a DIAGNOSTIC for the one failure of the engine Worker's start that was seen once (b2d2: a plain `Event` from `new Worker(...)`, `message` and `filename` empty) and never
// again in 690 starts (decision 0176). It only RECORDS what the page can still learn about the Worker script after the failure; it does not retry, re-create or hide anything (the page's state is `error` as before).
// A plain event (no message) is the signature of "the main script could not be fetched or loaded"; an exception thrown inside the Worker carries a message and is not diagnosed here.
// The Worker's URL is NOT kept in a variable by the caller (Vite only bundles `new Worker(new URL('./worker.ts', import.meta.url))` written as one expression): it is found among the page's resource-timing entries.

export interface WorkerDiagHost {
  fetch(url: string, init: { cache: 'no-store' }): Promise<{ status: number; headers: { get(name: string): string | null }; arrayBuffer(): Promise<ArrayBuffer> }>;
  /** performance.getEntriesByType('resource') as plain objects */
  resourceEntries(): Array<Record<string, unknown>>;
  now(): number;
  env(): Record<string, unknown>;
}

export interface WorkerEventLike { type?: string; message?: string; filename?: string; lineno?: number; colno?: number; isTrusted?: boolean }

/** true for the "plain Event" signature: no message (the Worker's main script was not loaded) */
export const isPlainWorkerStartEvent = (ev: WorkerEventLike): boolean => !ev.message;

/** the resource-timing entries that look like the engine Worker's script (the built file `worker-<hash>.js`, or the dev server's `…/worker.ts?worker_file…`) */
export const isWorkerScriptName = (name: string): boolean => /\/worker(-[^/?]*)?\.(js|ts)(\?|$)/.test(name);

const pick = (o: Record<string, unknown>, keys: string[]): Record<string, unknown> => Object.fromEntries(keys.filter((k) => k in o).map((k) => [k, o[k]]));

export async function diagnoseWorkerStart(ev: WorkerEventLike, host: WorkerDiagHost, startedAt: number): Promise<Record<string, unknown>> {
  const entries = host.resourceEntries().filter((e) => typeof e.name === 'string' && isWorkerScriptName(e.name as string));
  const info: Record<string, unknown> = {
    sinceWorkerCreatedMs: Math.round(host.now() - startedAt),
    event: pick(ev as Record<string, unknown>, ['type', 'message', 'filename', 'lineno', 'colno', 'isTrusted']),
    env: host.env(),
    // the first request's own record: no entry = the request never completed (or the URL is not one we recognise); responseStatus 0 / transferSize 0 with an entry = blocked, failed or served opaquely
    resourceTiming: entries.map((e) => pick(e, ['name', 'startTime', 'duration', 'responseStart', 'responseEnd', 'responseStatus', 'transferSize', 'encodedBodySize', 'nextHopProtocol', 'workerStart'])),
  };
  // the same URL now: is the failure permanent (status/exception now) or was it transient (it loads now)?
  const urls = [...new Set(entries.map((e) => e.name as string))].slice(0, 2);
  const refetch: Array<Record<string, unknown>> = [];
  for (const url of urls) {
    const t0 = host.now();
    try {
      const r = await host.fetch(url, { cache: 'no-store' });
      const body = await r.arrayBuffer();
      refetch.push({ url, ok: true, status: r.status, contentType: r.headers.get('content-type'), bytes: body.byteLength, ms: Math.round(host.now() - t0) });
    } catch (e) {
      refetch.push({ url, ok: false, name: (e as Error)?.name ?? 'unknown', message: String((e as Error)?.message ?? e).slice(0, 300), ms: Math.round(host.now() - t0) });
    }
  }
  info.refetch = refetch.length ? refetch : 'no resource-timing entry of the Worker script: nothing to re-fetch';
  return info;
}

export const browserWorkerDiagHost = (): WorkerDiagHost => ({
  fetch: (url, init) => fetch(url, init),
  resourceEntries: () => performance.getEntriesByType('resource').map((e) => JSON.parse(JSON.stringify(e)) as Record<string, unknown>),
  now: () => performance.now(),
  env: () => ({ onLine: typeof navigator !== 'undefined' ? navigator.onLine : null, visibility: (globalThis as { document?: { visibilityState?: string } }).document?.visibilityState ?? null, hardwareConcurrency: typeof navigator !== 'undefined' ? navigator.hardwareConcurrency : null, pageAgeMs: Math.round(performance.now()) }),
});
