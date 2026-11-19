import { afterEach, describe, expect, it } from 'vitest';
import { EngineClient } from '../../src/engine/client.ts';
import { readFileSync } from 'node:fs';
import { diagnoseWorkerStart, isPlainWorkerStartEvent, isWorkerScriptName, type WorkerDiagHost } from '../../src/engine/worker-diag.ts';
import type { Manifest } from '../../src/bundle/manifest.ts';

const host = (over: Partial<WorkerDiagHost> = {}): WorkerDiagHost => {
  let t = 100;
  return {
    fetch: async () => ({ status: 200, headers: { get: (n: string) => (n === 'content-type' ? 'text/javascript' : null) }, arrayBuffer: async () => new ArrayBuffer(1234) }),
    resourceEntries: () => [{ name: 'http://h/static/worker-AbC123.js', startTime: 5, duration: 0, responseStatus: 0, transferSize: 0, ignored: 'x' }, { name: 'http://h/static/index-1.js', startTime: 1, duration: 3 }],
    now: () => (t += 7),
    env: () => ({ onLine: true, visibility: 'visible' }),
    ...over,
  };
};

describe('worker start diagnostics (decision 0189)', () => {
  it('only a plain event (no message) is diagnosed', () => {
    expect(isPlainWorkerStartEvent({ message: '', filename: '' })).toBe(true);
    expect(isPlainWorkerStartEvent({})).toBe(true);
    expect(isPlainWorkerStartEvent({ message: 'Uncaught Error: boom', filename: 'x.js' })).toBe(false);
  });
  it('records the event, the resource timing of the first request and a re-fetch of the same URL', async () => {
    const d = await diagnoseWorkerStart({ type: 'error', message: '', filename: '', isTrusted: true }, host(), 90) as Record<string, any>;
    expect(d.event).toEqual({ type: 'error', message: '', filename: '', isTrusted: true });
    expect(d.resourceTiming).toEqual([{ name: 'http://h/static/worker-AbC123.js', startTime: 5, duration: 0, responseStatus: 0, transferSize: 0 }]);
    expect(d.refetch).toEqual([expect.objectContaining({ url: 'http://h/static/worker-AbC123.js', ok: true, status: 200, contentType: 'text/javascript', bytes: 1234 })]);
    expect(d.env).toEqual({ onLine: true, visibility: 'visible' });
  });
  it('a failing re-fetch is recorded, not swallowed', async () => {
    const d = await diagnoseWorkerStart({}, host({ fetch: async () => { throw new TypeError('Failed to fetch'); } }), 0) as Record<string, any>;
    expect(d.refetch).toEqual([expect.objectContaining({ ok: false, name: 'TypeError', message: 'Failed to fetch' })]);
  });
  it('without a resource-timing entry of the Worker script there is nothing to re-fetch, and that is said', async () => {
    const d = await diagnoseWorkerStart({}, host({ resourceEntries: () => [{ name: 'http://h/static/index-1.js' }] }), 0) as Record<string, any>;
    expect(d.resourceTiming).toEqual([]);
    expect(String(d.refetch)).toContain('no resource-timing entry');
  });
  it('the names that count as the Worker script (built file and dev server)', () => {
    expect(isWorkerScriptName('http://h/static/worker-DrJewceX.js')).toBe(true);
    expect(isWorkerScriptName('http://h/src/engine/worker.ts?worker_file&type=module')).toBe(true);
    expect(isWorkerScriptName('http://h/static/index-1.js')).toBe(false);
    expect(isWorkerScriptName('http://h/static/fle-out.worklet-B_LXwIiH.js')).toBe(false);
  });
  it('GUARD: client.ts creates the Worker with `new Worker(new URL(\'./worker.ts\', import.meta.url), …)` as ONE expression (Vite bundles the Worker only in that form; a URL kept in a variable is emitted as an asset `worker-*.ts` and the Worker never starts)', () => {
    const src = readFileSync(new URL('../../src/engine/client.ts', import.meta.url), 'utf8');
    expect(src).toMatch(/^\s*const w = new Worker\(new URL\('\.\/worker\.ts', import\.meta\.url\), /m);   // the code line itself (the NOTE comment above it quotes the same text)
  });
});

describe('EngineClient: a plain Worker event is reported as before and then diagnosed', () => {
  const g = globalThis as unknown as { Worker?: unknown; fetch?: unknown; __flWorkerStartFailure?: unknown };
  const saved = { Worker: g.Worker, fetch: g.fetch };
  afterEach(() => { g.Worker = saved.Worker; g.fetch = saved.fetch; delete g.__flWorkerStartFailure; });
  const manifest = { engine: { kind: 'emscripten-fruity-v1', env: {} } } as unknown as Manifest;
  const assets = { module: {} as WebAssembly.Module, seed: new Uint8Array(0), glue: new Uint8Array(0), dll: new Uint8Array(0) };
  const run = async (ev: { message: string; filename: string; lineno: number }) => {
    let w: { onerror: ((e: unknown) => void) | null } | null = null;
    class FakeWorker { onmessage: unknown = null; onerror: ((e: unknown) => void) | null = null; onmessageerror: unknown = null; constructor() { w = this; } postMessage(): void { /* nothing */ } terminate(): void { /* nothing */ } }
    g.Worker = FakeWorker; g.fetch = async () => ({ status: 404, headers: { get: () => null }, arrayBuffer: async () => new ArrayBuffer(0) });
    const errors: string[] = [], logs: string[] = [];
    const events = { deskCanvas: () => ({ transferControlToOffscreen: () => ({}) }) as unknown as HTMLCanvasElement, log: (_l: string, t: string) => { logs.push(t); }, exit: () => undefined, error: (p: string, m: string) => { errors.push(`${p}: ${m}`); } };
    void new EngineClient(events).init(manifest, assets);
    w!.onerror!(ev);
    await new Promise((r) => setTimeout(r, 20));
    return { errors, logs };
  };
  it('plain event: the error is reported exactly as before, and one diagnostics line is logged', async () => {
    const { errors, logs } = await run({ message: '', filename: '', lineno: 0 });
    expect(errors).toEqual(['worker: the engine Worker raised an error (:0)']);
    expect(logs.filter((l) => l.startsWith('worker start failure diagnostics: '))).toHaveLength(1);
    expect(g.__flWorkerStartFailure).toBeTruthy();
  });
  it('an event with a message is not diagnosed', async () => {
    const { errors, logs } = await run({ message: 'Uncaught boom', filename: 'worker.js', lineno: 3 });
    expect(errors).toEqual(['worker: Uncaught boom (worker.js:3)']);
    expect(logs).toEqual([]);
  });
});
