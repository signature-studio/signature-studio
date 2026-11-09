import { describe, expect, it } from 'vitest';
import { blockingMissing, describeMissing, detectFeatures, type FeatureEnv } from '../../src/engine/features.ts';

const fn = () => undefined;
const full = (): FeatureEnv => ({ WebAssembly: { Suspending: fn, promising: fn, compileStreaming: fn, validate: () => true, Tag: fn }, Worker: fn, OffscreenCanvas: fn });

describe('detectFeatures', () => {
  it('passes for a browser that has everything', () => {
    const r = detectFeatures(full());
    expect(r.ok).toBe(true);
    expect(r.missing).toEqual([]);
  });

  it('reports a browser without JSPI by the names of what is missing', () => {
    const env = full();
    delete env.WebAssembly!.Suspending;
    delete env.WebAssembly!.promising;
    const r = detectFeatures(env);
    expect(r.ok).toBe(false);
    const j = r.missing.find((f) => f.name.startsWith('JSPI'))!;
    expect(j.detail).toContain('WebAssembly.Suspending and WebAssembly.promising');
    expect(blockingMissing(r)).toHaveLength(1);
  });

  it('names only the one JSPI function that is missing', () => {
    const env = full();
    delete env.WebAssembly!.promising;
    const j = detectFeatures(env).missing[0]!;
    expect(j.detail).toContain('WebAssembly.promising is missing');
    expect(j.detail).not.toContain('Suspending');
  });

  it('reports a browser that cannot validate the legacy exception encoding', () => {
    const env = full();
    env.WebAssembly!.validate = () => false;
    const r = detectFeatures(env);
    expect(r.missing.map((f) => f.name)).toEqual(['WebAssembly exception handling (legacy encoding)']);
  });

  it('survives a validate that throws and a missing WebAssembly object', () => {
    const env = full();
    env.WebAssembly!.validate = () => { throw new Error('boom'); };
    expect(detectFeatures(env).missing[0]!.detail).toContain('validate threw');
    const none = detectFeatures({});
    expect(none.ok).toBe(false);
    expect(none.missing.length).toBeGreaterThanOrEqual(4);
  });

  it('blocks on a missing OffscreenCanvas (desk2: the real engine is shown on one and there is no other display method)', () => {
    const env = full();
    delete env.OffscreenCanvas;
    const r = detectFeatures(env);
    expect(r.ok).toBe(false);
    expect(blockingMissing(r).map((f) => f.name)).toEqual(['OffscreenCanvas']);
    const d = describeMissing(blockingMissing(r));
    expect(d.what).toContain('OffscreenCanvas');
  });

  it('the error says what is missing and where', () => {
    const env = full();
    delete env.WebAssembly!.Suspending;
    const d = describeMissing(blockingMissing(detectFeatures(env)));
    expect(d.what).toContain('JSPI');
    expect(d.what).toContain('WebAssembly.Suspending is missing');
    expect(d.where).toContain('before any download');
  });

  it('really validates the exception-handling probe of this runtime', () => {
    // node 24 has JSPI and legacy exception handling; this is the check of the code against a real WebAssembly.validate
    const r = detectFeatures();
    expect(r.features.find((f) => f.name.startsWith('WebAssembly exception'))!.ok).toBe(true);
  });
});
