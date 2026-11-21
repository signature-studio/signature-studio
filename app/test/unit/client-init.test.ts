import { afterEach, describe, expect, it } from 'vitest';
import { EngineClient } from '../../src/engine/client.ts';
import type { ToRender, ToWorker } from '../../src/engine/protocol.ts';
import type { Manifest } from '../../src/bundle/manifest.ts';

// the page -> Worker init messages (decision 0223, C-2: the render Worker is handed the desktop canvas; the engine Worker the input ring and the link to the render Worker). Both Workers post to `sent` in the order they were created.
const sent: (ToWorker | ToRender)[] = [];
class FakeWorker {
  onmessage: unknown = null;
  onerror: unknown = null;
  onmessageerror: unknown = null;
  readonly url: unknown;
  readonly opts: unknown;
  constructor(url: unknown, opts: unknown) { this.url = url; this.opts = opts; }
  postMessage(m: ToWorker | ToRender): void { sent.push(m); }
  terminate(): void { /* nothing */ }
}
const g = globalThis as unknown as { Worker?: unknown };
const original = g.Worker;
afterEach(() => { g.Worker = original; sent.length = 0; });

const manifest = { engine: { kind: 'emscripten-fruity-v1', env: {} } } as unknown as Manifest;
const assets = { module: {} as WebAssembly.Module, seed: new Uint8Array(0), glue: new Uint8Array(0), dll: new Uint8Array(0) };
const events = { log: () => undefined, exit: () => undefined, error: () => undefined };
const canvasOf = (transfer: () => unknown = () => ({})) => ({ deskCanvas: () => ({ transferControlToOffscreen: transfer }) as unknown as HTMLCanvasElement });

describe('EngineClient.init message', () => {
  it('a full ring does not lose the event: it goes as a message', () => {
    g.Worker = FakeWorker;
    const c = new EngineClient({ ...events, ...canvasOf() });
    void c.init(manifest, assets);
    sent.length = 0;
    const ev = { k: 'mouse', kind: 1, x: 5, y: 6, data: 0 } as const;
    for (let i = 0; i < 1024; i++) c.sendInput(i, 100, ev);
    expect(sent.length).toBe(0);
    c.sendInput(1024, 100, ev);
    expect(sent).toEqual([{ cmd: 'input', seq: 1024, tPage: 100, ev }]);
    expect(c.ringStats.viaMessage).toBe(1);
  });
  it('the render Worker is handed the desktop canvas (transferred); the engine Worker gets the input ring and the link to the render Worker (C-2)', () => {
    g.Worker = FakeWorker;
    const transferred: unknown[] = [];
    void new EngineClient({ ...events, ...canvasOf(() => { const o = { offscreen: true }; transferred.push(o); return o; }) }).init(manifest, assets);
    const r = sent[0] as Extract<ToRender, { cmd: 'init' }>;
    expect(r.cmd).toBe('init');
    expect(r.desk.canvas).toBe(transferred[0]);
    expect(r.desk.ctl instanceof SharedArrayBuffer).toBe(true);
    const a = sent[1] as Extract<ToWorker, { cmd: 'init' }>;
    expect(a.cmd).toBe('init');
    expect(a.desk.ring instanceof SharedArrayBuffer).toBe(true);
    expect(a.desk.ctl).toBe(r.desk.ctl);
    expect(Object.keys(a).sort()).toEqual(['assets', 'cmd', 'desk', 'manifest']);
    expect(Object.keys(a.desk).sort()).toEqual(['ctl', 'port', 'ring']);
  });
  it('an input event goes into the ring and a message wakes the Worker only if it is parked', () => {
    g.Worker = FakeWorker;
    const c = new EngineClient({ ...events, ...canvasOf() });
    void c.init(manifest, assets);
    const init = sent[1] as Extract<ToWorker, { cmd: 'init' }>;
    sent.length = 0;
    const ev = { k: 'mouse', kind: 1, x: 5, y: 6, data: 0 } as const;
    c.sendInput(1, 100, ev);
    expect(sent.length).toBe(0);                       // not parked: nothing but the ring
    new Int32Array(init.desk.ring)[2] = 1;             // the Worker says it is parked
    c.sendInput(2, 101, ev);
    expect(sent).toEqual([{ cmd: 'wake' }]);
    expect(c.ringStats.wakes).toBe(1);
  });
});
