import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { afterAll, describe, expect, it } from 'vitest';
import { validateManifest, type FruityOptions } from '../../src/bundle/manifest.ts';
import { parsePack } from '../../src/bundle/pack.ts';

const SCRIPT = path.resolve(import.meta.dirname, '../../scripts/pack-assets.mjs');
const tmp = fs.mkdtempSync(path.join(process.env.TMPDIR ?? os.tmpdir(), 'packfruity-'));
afterAll(() => fs.rmSync(tmp, { recursive: true, force: true }));
const sha = (b: Uint8Array) => crypto.createHash('sha256').update(b).digest('hex');

function seedPack(files: Record<string, Buffer>): Buffer {
  const entries: object[] = [];
  let off = 0;
  const blobs: Buffer[] = [];
  for (const [p, d] of Object.entries(files)) { entries.push({ path: p, size: d.length, offset: off, sha256: sha(d) }); blobs.push(d); off += d.length; }
  const header = Buffer.from(JSON.stringify({ version: 1, root: 'seed', count: entries.length, data_bytes: off, files: entries }));
  const head = Buffer.alloc(12);
  head.write('FLSEEDP1', 0, 'latin1');
  head.writeUInt32LE(header.length, 8);
  return Buffer.concat([head, header, ...blobs]);
}

const u32 = (n: number): number[] => { const o: number[] = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; };
const str = (t: string): number[] => [...u32(t.length), ...Buffer.from(t)];
/** a module that exports the functions `names` (decision 0223: the engine wasm must export the desktop compositor) */
function wasmExporting(names: string[]): Buffer {
  const exp = [...u32(names.length), ...names.flatMap((n) => [...str(n), 0, 0])];
  const body = [0, 0x0b];
  return Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 3, 2, 1, 0, 7, ...u32(exp.length), ...exp, 10, ...u32(body.length + 2), 1, body.length, ...body]);
}
const wasm = wasmExporting(['fle_desk_compose', 'fle_desk_resync']);
const glue = Buffer.from('export default function createModule(o) { return Promise.resolve(o); }\n');
const seed = seedPack({ 'seed/a.tsv': Buffer.from('a\tb\n'), 'seed/dir/b.bin': Buffer.from([1, 2, 3]) });
fs.mkdirSync(path.join(tmp, 'dll'));
fs.writeFileSync(path.join(tmp, 'dll/FLEngine_x64.dll'), Buffer.from('MZ-engine'));
fs.writeFileSync(path.join(tmp, 'dll/m1.2000000.dll'), Buffer.from('MZ-module'));
fs.writeFileSync(path.join(tmp, 'engine.wasm'), wasm);
fs.writeFileSync(path.join(tmp, 'glue.js'), glue);
fs.writeFileSync(path.join(tmp, 'seed.bin'), seed);

const run = (extra: string[], out = 'out') => spawnSync(process.execPath, [SCRIPT, '--host-files', 'emscripten-6.0.12', '--out', path.join(tmp, out), '--prefix', 'fl-x', '--name', 'fl-engine-x', '--version', '1', '--engine-wasm', path.join(tmp, 'engine.wasm'),
  '--seed-pack', path.join(tmp, 'seed.bin'), '--dll-dir', path.join(tmp, 'dll'), '--brotli-quality', '5', ...extra], { encoding: 'utf8' });

describe('pack-assets.mjs', () => {
  const r = run(['--glue', path.join(tmp, 'glue.js'), '--args', '/data/FLEngine_x64.dll /data/m1.2000000.dll', '--env', 'FLE_CALL_FRUITY=1', '--env', 'FLE_VFS_SEED_ROOT=/data/seed',
    '--fruity-hints', 'first=49,stop=55,variant=Oz', '--stop-note', 'some stop=>some note', '--bucket', 'my-bucket']);
  const dir = path.join(tmp, 'out');
  const manifest = () => JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf8'));

  it('succeeds and writes a manifest that the app\'s own validator accepts', () => {
    expect(r.status, r.stderr).toBe(0);
    const v = validateManifest(manifest());
    expect(v.ok, JSON.stringify(v)).toBe(true);
  });

  it('names the glue by SHA-256, with the options of the bundle', () => {
    const m = manifest();
    const o = m.engine.options as FruityOptions;
    expect(m.engine.kind).toBe('emscripten-fruity-v1');
    expect(o.glue).toMatchObject({ size: glue.length, sha256: sha(glue), compression: 'br' });
    expect(o.glue.url).toMatch(/^\/assets\/fl-x\/engine-glue\.[0-9a-f]{12}\.js$/);
    expect(o.root).toBe('/data');
    expect(o.hostFilesAbi).toBe('emscripten-6.0.12');
    expect(m.engine.desk).toBeUndefined();
    expect(o.hints).toEqual({ firstPixelSeconds: 49, stopSeconds: 55, variant: 'Oz' });
    expect(o.stopNotes).toEqual([{ contains: 'some stop', note: 'some note' }]);
    expect(m.engine.args).toEqual(['/data/FLEngine_x64.dll', '/data/m1.2000000.dll']);
    expect(m.engine.env).toEqual({ FLE_CALL_FRUITY: '1', FLE_VFS_SEED_ROOT: '/data/seed' });
  });

  it('needs --host-files (decision 0221: the Emscripten that linked the glue) and writes it as hostFilesAbi', () => {
    const bad = spawnSync(process.execPath, [SCRIPT, '--out', path.join(tmp, 'out-nohf'), '--name', 'x', '--version', '1', '--engine-wasm', path.join(tmp, 'engine.wasm'), '--glue', path.join(tmp, 'glue.js'), '--seed-pack', path.join(tmp, 'seed.bin'), '--dll-dir', path.join(tmp, 'dll'), '--args', '/data/FLEngine_x64.dll'], { encoding: 'utf8' });
    expect(bad.status).not.toBe(0);
    expect(bad.stderr).toContain('--host-files is required');
  });

  it('refuses an engine wasm that does not export the desktop compositor (decision 0223)', () => {
    fs.writeFileSync(path.join(tmp, 'nodesk.wasm'), wasmExporting(['fle_desk_compose']));
    const bad = spawnSync(process.execPath, [SCRIPT, '--host-files', 'emscripten-6.0.12', '--out', path.join(tmp, 'out-nodesk'), '--name', 'x', '--version', '1', '--engine-wasm', path.join(tmp, 'nodesk.wasm'), '--glue', path.join(tmp, 'glue.js'), '--seed-pack', path.join(tmp, 'seed.bin'), '--dll-dir', path.join(tmp, 'dll'), '--args', '/data/FLEngine_x64.dll'], { encoding: 'utf8' });
    expect(bad.status).not.toBe(0);
    expect(bad.stderr).toContain('desktop compositor');
  });

  it('packs the DLL directory as flpack1 (pack path = file name) and takes the FLSEEDP1 seed pack as it is', () => {
    const m = manifest();
    expect(m.seedPack).toMatchObject({ format: 'flseedp1', size: seed.length, sha256: sha(seed) });
    expect(m.dllPack.format).toBe('flpack1');
    const file = fs.readdirSync(dir).find((f) => /^dll\.[0-9a-f]{12}\.flpack$/.test(f))!;
    const files = parsePack(new Uint8Array(fs.readFileSync(path.join(dir, file))), { verifyFiles: true });
    expect([...files.keys()].sort()).toEqual(['FLEngine_x64.dll', 'm1.2000000.dll']);
    expect(Buffer.from(files.get('m1.2000000.dll')!.data).toString()).toBe('MZ-module');
  });

  it('writes brotli variants that decode to the originals and no gzip variants', () => {
    const m = manifest();
    const wasmFile = path.basename(m.engine.wasm.url);
    expect(zlib.brotliDecompressSync(fs.readFileSync(path.join(dir, wasmFile + '.br')))).toEqual(wasm);
    expect(fs.readdirSync(dir).filter((f) => f.endsWith('.gz'))).toEqual([]);
    expect(m.engine.wasm.encodedSize).toBe(fs.statSync(path.join(dir, wasmFile + '.br')).size);
  });

  it('only prints the upload commands: br objects get --content-encoding br, the manifest no-cache, and nothing was run', () => {
    const cmds = fs.readFileSync(path.join(dir, 'upload-commands.txt'), 'utf8');
    expect(cmds).toContain("wrangler r2 object put 'my-bucket/fl-x/manifest.json'");
    const puts = cmds.split('\n').filter((l) => l.startsWith('wrangler r2 object put'));
    expect(puts).toHaveLength(5); // wasm, glue, seed pack, DLL pack, manifest
    expect(puts.filter((l) => l.includes('--content-encoding br'))).toHaveLength(4);
    expect(cmds).toMatch(/engine-glue\.[0-9a-f]{12}\.js' --remote --file '[^']+\.br' --content-type 'text\/javascript' --content-encoding br/);
    expect(puts.find((l) => l.includes('manifest.json'))).toContain('--cache-control no-cache');
    expect(r.stdout).toContain('nothing was uploaded');
  });

  it('refuses to build without the glue', () => {
    const bad = run([], 'out-noglue');
    expect(bad.status).not.toBe(0);
    expect(bad.stderr).toContain('--glue is required');
  });
});
