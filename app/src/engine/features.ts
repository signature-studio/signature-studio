// What the browser must have for the real engine (engine.kind emscripten-fruity-v1), checked BEFORE the big downloads start, so that a missing feature is reported by name.
//  - JSPI: WebAssembly.Suspending / WebAssembly.promising. The engine's cooperative guest threads, its idle wait and its window loop switch stacks with them (decisions 0122, 0124).
//  - WebAssembly.compileStreaming: the engine wasm is compiled while it downloads (the 250-670 MB module is never held twice).
//  - WebAssembly exceptions (the legacy encoding emcc 6.0.12 emits for -sSUPPORT_LONGJMP=wasm): the module does not validate without them.
//  - Worker + OffscreenCanvas: the engine runs in a module Worker and shows the desktop on an OffscreenCanvas there (decisions 0223 and desk2: the only display method of the real engine; the page draws nothing).
// The function takes the global object as a parameter so that the unit tests can describe a browser that lacks one of them.

export interface Feature { name: string; ok: boolean; detail: string }
export interface FeatureReport { ok: boolean; features: Feature[]; missing: Feature[] }

export interface FeatureEnv {
  WebAssembly?: Partial<Record<'Suspending' | 'promising' | 'compileStreaming' | 'validate' | 'Tag', unknown>>;
  Worker?: unknown;
  OffscreenCanvas?: unknown;
}

// (module (func try catch_all end)) in the legacy exception encoding: magic, version, type section (func () -> ()), function section, code section with `try (0x06) 0x40 catch_all (0x19) end end`
const LEGACY_EH_PROBE = new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 3, 2, 1, 0, 10, 8, 1, 6, 0, 0x06, 0x40, 0x19, 0x0b, 0x0b]);

function has(v: unknown): boolean { return typeof v === 'function'; }

export function detectFeatures(env: FeatureEnv = globalThis as unknown as FeatureEnv): FeatureReport {
  const wa = env.WebAssembly;
  const jspi = !!wa && has(wa.Suspending) && has(wa.promising);
  const missingJspi = [!wa || !has(wa.Suspending) ? 'WebAssembly.Suspending' : '', !wa || !has(wa.promising) ? 'WebAssembly.promising' : ''].filter(Boolean).join(' and ');
  let eh = false;
  let ehDetail = 'WebAssembly.validate is not available';
  if (wa && has(wa.validate)) {
    try { eh = (wa.validate as (b: Uint8Array) => boolean)(LEGACY_EH_PROBE); ehDetail = eh ? 'a module with try/catch_all validates' : 'a module with try/catch_all (legacy exception handling) does not validate'; } catch (e) { ehDetail = `validate threw: ${String(e)}`; }
  }
  const features: Feature[] = [
    { name: 'JSPI (WebAssembly JavaScript Promise Integration)', ok: jspi, detail: jspi ? 'WebAssembly.Suspending and WebAssembly.promising exist' : `${missingJspi} ${missingJspi.includes(' and ') ? 'are' : 'is'} missing; the engine switches its guest threads and waits on stack switching and cannot run without it` },
    { name: 'WebAssembly.compileStreaming', ok: !!wa && has(wa.compileStreaming), detail: wa && has(wa.compileStreaming) ? 'present' : 'missing; the engine wasm is compiled while it downloads' },
    { name: 'WebAssembly exception handling (legacy encoding)', ok: eh, detail: ehDetail },
    { name: 'module Worker', ok: has(env.Worker), detail: has(env.Worker) ? 'present' : 'Worker is missing' },
    { name: 'OffscreenCanvas', ok: has(env.OffscreenCanvas), detail: has(env.OffscreenCanvas) ? 'present' : 'missing; the engine is shown on an OffscreenCanvas in its Worker and there is no other way to show it' },
  ];
  const missing = features.filter((f) => !f.ok);
  return { ok: missing.length === 0, features, missing };
}

/** the features without which the real engine cannot run at all: every feature above (since desk2 the page has no display method that needs no OffscreenCanvas) */
export function blockingMissing(report: FeatureReport): Feature[] {
  return report.missing;
}

export function describeMissing(missing: readonly Feature[]): { what: string; where: string } {
  return {
    what: `This browser lacks what the FL engine needs: ${missing.map((f) => `${f.name} - ${f.detail}`).join('; ')}. The app was measured with Chrome 156 (V8 15.6) only.`,
    where: 'feature check (before any download)',
  };
}
