import { describe, expect, it } from 'vitest';
import { MANIFEST_SCHEMA, FRUITY_OPTIONS_SCHEMA, validateManifest } from '../../src/bundle/manifest.ts';
import { validate, isSameOriginPath, isRelativePackPath } from '../../src/bundle/schema.ts';

const sha = 'a'.repeat(64);
const asset = (url: string) => ({ url, size: 10, sha256: sha, compression: 'identity' });
const good = () => ({
  schema: 1,
  bundle: { name: 'fl-engine-Oz', version: 'abc123' },
  engine: {
    kind: 'emscripten-fruity-v1', wasm: asset('/assets/fl/engine.abc.wasm'),
    args: ['/data/FLEngine_x64.dll', '/data/a.dll:/data/b.dll'], env: { FLE_CALL_FRUITY: '1' },
    options: {
      glue: asset('/assets/fl/engine-glue.abc.js'), root: '/data', hostFilesAbi: 'emscripten-6.0.12',
      hints: { firstPixelSeconds: 49, stopSeconds: 55, variant: 'Oz' },
      stopNotes: [{ contains: 'RaiseException', note: 'a still picture' }],
    },
  },
  seedPack: { ...asset('/assets/fl/seed.abc.flpack'), format: 'flseedp1' },
  dllPack: { ...asset('/assets/fl/dll.abc.flpack'), format: 'flpack1' },
});

describe('manifest schema 1', () => {
  it('accepts a complete manifest', () => {
    const r = validateManifest(good());
    expect(r.ok, JSON.stringify(r)).toBe(true);
  });

  it('accepts a manifest without hints and stop notes', () => {
    const m = good() as any;
    delete m.engine.options.hints;
    delete m.engine.options.stopNotes;
    expect(validateManifest(m).ok).toBe(true);
  });

  const bad = (label: string, mutate: (m: any) => void, path: string, text?: string) =>
    it(`rejects: ${label}`, () => {
      const m = good();
      mutate(m);
      const r = validateManifest(m);
      expect(r.ok).toBe(false);
      if (!r.ok) {
        expect(r.errors.map((e) => e.path)).toContain(path);
        if (text) expect(r.errors.map((e) => e.message).join('\n')).toContain(text);
      }
    });

  bad('a missing engine wasm', (m) => { delete m.engine.wasm; }, '/engine/wasm', 'required');
  bad('a SHA-256 that is not 64 lower-case hex digits', (m) => { m.engine.wasm.sha256 = 'ABC'; }, '/engine/wasm/sha256');
  bad('a negative size', (m) => { m.engine.wasm.size = -1; }, '/engine/wasm/size');
  bad('a fractional size', (m) => { m.engine.wasm.size = 1.5; }, '/engine/wasm/size', 'integer');
  bad('an unknown compression', (m) => { m.engine.wasm.compression = 'zstd'; }, '/engine/wasm/compression', 'one of');
  bad('an absolute URL to another origin', (m) => { m.engine.wasm.url = 'https://cdn.example.com/engine.wasm'; }, '/engine/wasm/url', 'this origin');
  bad('a protocol-relative URL', (m) => { m.seedPack.url = '//cdn.example.com/seed'; }, '/seedPack/url', 'this origin');
  bad('a javascript: URL', (m) => { m.seedPack.url = 'javascript:alert(1)'; }, '/seedPack/url');
  bad('an unknown top-level property', (m) => { m.extra = 1; }, '/extra', 'unknown');
  bad('the wrong schema number', (m) => { m.schema = 2; }, '/schema');
  bad('a bundle name with a slash', (m) => { m.bundle.name = 'a/b'; }, '/bundle/name');
  bad('a pack with the wrong format', (m) => { m.seedPack.format = 'zip'; }, '/seedPack/format');
  bad('args that are not strings', (m) => { m.engine.args = [1]; }, '/engine/args/0');
  bad('a kind that is not the engine', (m) => { m.engine.kind = 'unknown-engine-kind-v0'; }, '/engine/kind', 'one of');
  bad('a field of the removed desktop marker', (m) => { m.engine.desk = true; }, '/engine/desk', 'unknown');
  bad('the removed expect block', (m) => { m.expect = { surfaces: [] }; }, '/expect', 'unknown');

  it('rejects things that are not objects', () => {
    for (const v of [null, 1, 'x', [], undefined]) expect(validateManifest(v).ok, JSON.stringify(v)).toBe(false);
  });

  it('reports every error, not only the first', () => {
    const m = good() as any;
    m.engine.wasm.sha256 = 'x';
    m.engine.wasm.size = -5;
    m.bundle.version = '';
    const r = validateManifest(m);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.errors.length).toBeGreaterThanOrEqual(3);
  });

  it('the schemas themselves use only known keywords', () => {
    expect(validate({ type: 'object', bogus: 1 } as any, {}).map((e) => e.message).join()).toContain('unknown keyword bogus');
    const own = [...validate(MANIFEST_SCHEMA, good()), ...validate(FRUITY_OPTIONS_SCHEMA, good().engine.options)];
    expect(own.filter((e) => e.message.startsWith('schema error'))).toEqual([]);
  });

  it('path helpers', () => {
    expect(isSameOriginPath('/assets/a.wasm')).toBe(true);
    expect(isSameOriginPath('a/b.wasm')).toBe(true);
    for (const s of ['', 'http://x/y', 'data:text/plain,x', '//x/y', 'a\\b']) expect(isSameOriginPath(s), s).toBe(false);
    expect(isRelativePackPath('a/b.txt')).toBe(true);
    for (const s of ['', '/a', 'a//b', './a', 'a/../b', 'a\\b']) expect(isRelativePackPath(s), s).toBe(false);
  });
});

describe('manifest schema 1: the options of the engine', () => {
  const badOpt = (label: string, mutate: (m: any) => void, path: string, text?: string) =>
    it(`rejects: ${label}`, () => {
      const m = good();
      mutate(m);
      const r = validateManifest(m);
      expect(r.ok).toBe(false);
      if (!r.ok) {
        expect(r.errors.map((e) => e.path)).toContain(path);
        if (text) expect(r.errors.map((e) => e.message).join('\n')).toContain(text);
      }
    });

  badOpt('no options', (m) => { delete m.engine.options; }, '/engine/options', 'required');
  badOpt('no glue', (m) => { delete m.engine.options.glue; }, '/engine/options/glue', 'required');
  badOpt('a glue on another origin', (m) => { m.engine.options.glue.url = 'https://cdn.example.com/g.js'; }, '/engine/options/glue/url', 'this origin');
  badOpt('a glue with a bad SHA-256', (m) => { m.engine.options.glue.sha256 = 'nope'; }, '/engine/options/glue/sha256');
  badOpt('a file root that is not an absolute path', (m) => { m.engine.options.root = 'data'; }, '/engine/options/root');
  badOpt('a file root with ..', (m) => { m.engine.options.root = '/data/../etc'; }, '/engine/options/root');
  badOpt('no Emscripten version (decision 0221: the file calls are answered by the page for that version)', (m) => { delete m.engine.options.hostFilesAbi; }, '/engine/options/hostFilesAbi', 'required');
  badOpt('a malformed version', (m) => { m.engine.options.hostFilesAbi = '6.0.12'; }, '/engine/options/hostFilesAbi');
  badOpt('the removed hostFiles marker', (m) => { m.engine.options.hostFiles = 'packs-v1'; }, '/engine/options/hostFiles', 'unknown');
  badOpt('the removed mainLid', (m) => { m.engine.options.mainLid = 13; }, '/engine/options/mainLid', 'unknown');
  badOpt('a negative first-pixel hint', (m) => { m.engine.options.hints.firstPixelSeconds = -1; }, '/engine/options/hints/firstPixelSeconds');
  badOpt('a stop note without text', (m) => { m.engine.options.stopNotes[0].note = ''; }, '/engine/options/stopNotes/0/note');
  badOpt('an unknown option', (m) => { m.engine.options.entry = 'main'; }, '/engine/options/entry', 'unknown');
  badOpt('no seed pack', (m) => { m.seedPack = null; }, '/seedPack');
  badOpt('no DLL pack', (m) => { m.dllPack = null; }, '/dllPack');
  badOpt('no engine arguments', (m) => { m.engine.args = []; }, '/engine/args', 'must name the engine DLL');
  badOpt('an engine DLL outside the file root', (m) => { m.engine.args[0] = '/elsewhere/FLEngine_x64.dll'; }, '/engine/args/0', '/data/');
});
