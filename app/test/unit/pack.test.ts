import { describe, expect, it } from 'vitest';
import { buildPack, parsePack, PackError } from '../../src/bundle/pack.ts';

const enc = (s: string) => new TextEncoder().encode(s);

describe('flpack1', () => {
  const files = [
    { path: 'a.txt', data: enc('hello') },
    { path: 'dir/b.bin', data: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]) },
    { path: 'empty', data: new Uint8Array(0) },
    { path: 'dir/c.txt', data: enc('x'.repeat(100)) },
  ];

  it('round trips and keeps every file at an 8-byte aligned offset', () => {
    const pack = buildPack(files);
    const out = parsePack(pack, { verifyFiles: true });
    expect([...out.keys()]).toEqual(files.map((f) => f.path));
    for (const f of files) expect(out.get(f.path)?.data).toEqual(f.data);
    for (const f of files) expect(((out.get(f.path)?.data.byteOffset ?? 1) % 8)).toBe(0);
  });

  it('the first byte of the second file is where the index says (an off-by-one in the offset is visible)', () => {
    const out = parsePack(buildPack(files));
    expect(out.get('dir/b.bin')?.data[0]).toBe(1);
    expect(out.get('dir/b.bin')?.data[8]).toBe(9);
    expect(out.get('a.txt')?.data.length).toBe(5);
  });

  it('refuses bad input with a specific message', () => {
    const pack = buildPack(files);
    const t = (f: (b: Uint8Array) => Uint8Array, text: string) => expect(() => parsePack(f(Uint8Array.from(pack)))).toThrow(text);
    t((b) => b.subarray(0, 8), 'too short');
    t((b) => { b[0] = 0x58; return b; }, 'bad magic');
    t((b) => { b[4] = 9; return b; }, 'unsupported pack version');
    t((b) => b.subarray(0, 40), 'index runs past the end');
    t((b) => b.subarray(0, b.length - 50), 'runs past the end');
    expect(() => parsePack(pack.subarray(0, 3))).toThrow(PackError);
  });

  it('verifies the content hash of each file when asked', () => {
    const pack = Uint8Array.from(buildPack(files));
    const at = parsePack(pack).get('a.txt')!.data.byteOffset - pack.byteOffset;
    pack[at] ^= 0xff;
    expect(() => parsePack(pack, { verifyFiles: true })).toThrow('does not match the index');
    expect(() => parsePack(pack)).not.toThrow();
  });

  it('rejects duplicate and unsafe paths when building', () => {
    expect(() => buildPack([{ path: 'a', data: enc('1') }, { path: 'a', data: enc('2') }])).toThrow('duplicate');
    expect(() => buildPack([{ path: '../a', data: enc('1') }])).toThrow('bad path');
  });
});

// ---- flseedp1: the seed pack of decision 0134 (lift/harness/web/final0129/pack_seed.py) ----
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { detectPackFormat } from '../../src/bundle/pack.ts';

// the layout written the way pack_seed.py writes it: 'FLSEEDP1' | u32 header length | JSON header | data region (offsets from the start of the data region, no padding)
function seedPack1(files: { path: string; data: Uint8Array }[]): Uint8Array {
  let off = 0;
  const entries = files.map((f) => { const e = { path: f.path, size: f.data.length, offset: off, sha256: createHash('sha256').update(f.data).digest('hex') }; off += f.data.length; return e; });
  const header = new TextEncoder().encode(JSON.stringify({ version: 1, root: 'seed', count: files.length, data_bytes: off, files: entries }));
  const out = new Uint8Array(12 + header.length + off);
  out.set(new TextEncoder().encode('FLSEEDP1'), 0);
  new DataView(out.buffer).setUint32(8, header.length, true);
  out.set(header, 12);
  let at = 12 + header.length;
  for (const f of files) { out.set(f.data, at); at += f.data.length; }
  return out;
}

describe('flseedp1 (decision 0134 seed pack)', () => {
  const files = [{ path: 'seed/vfs-seed-idle1.tsv', data: enc('# list\n') }, { path: 'seed/Data/System/Empty.flp', data: new Uint8Array([9, 8, 7, 6, 5]) }, { path: 'seed/Shared/x y.txt', data: enc('spaces in names are fine') }];

  it('is recognised by its magic and read, with every file at the right place', () => {
    const pack = seedPack1(files);
    expect(detectPackFormat(pack)).toBe('flseedp1');
    expect(detectPackFormat(buildPack(files.map((f) => ({ ...f, path: f.path.replace('seed/', '') }))))).toBe('flpack1');
    const out = parsePack(pack, { verifyFiles: true, format: 'flseedp1' });
    expect([...out.keys()]).toEqual(files.map((f) => f.path));
    for (const f of files) expect(out.get(f.path)?.data).toEqual(f.data);
  });

  it('refuses a pack whose format is not the one the manifest names', () => {
    expect(() => parsePack(seedPack1(files), { format: 'flpack1' })).toThrow('the manifest says flpack1 but the bytes are flseedp1');
    expect(() => parsePack(buildPack([{ path: 'a', data: enc('x') }]), { format: 'flseedp1' })).toThrow('but the bytes are flpack1');
  });

  it('refuses a bad version, a short header, a file past the end and a changed byte', () => {
    const good = seedPack1(files);
    const patched = (f: (b: Uint8Array) => void) => { const b = Uint8Array.from(good); f(b); return b; };
    expect(() => parsePack(good.subarray(0, 40))).toThrow('header runs past the end');
    expect(() => parsePack(good.subarray(0, good.length - 10))).toThrow('runs past the end');
    const dataStart = good.length - files.reduce((n, f) => n + f.data.length, 0);
    expect(() => parsePack(patched((b) => { b[dataStart + 8] ^= 1; }), { verifyFiles: true })).toThrow('does not match the index');
    const hdr = new TextDecoder().decode(good.subarray(12, dataStart)).replace('"version":1', '"version":2');
    const v2 = new Uint8Array(12 + hdr.length + (good.length - dataStart));
    v2.set(good.subarray(0, 8)); new DataView(v2.buffer).setUint32(8, hdr.length, true); v2.set(new TextEncoder().encode(hdr), 12); v2.set(good.subarray(dataStart), 12 + hdr.length);
    expect(() => parsePack(v2)).toThrow('unsupported flseedp1 version 2');
  });

  // The real producer. pack_seed.py exists from decision 0134 on (it is not in the HEAD this app was written against), so the test is skipped where it is absent.
  const script = process.env.PACK_SEED_PY ?? path.resolve(import.meta.dirname, '../../../lift/harness/web/final0129/pack_seed.py');
  const havePy = fs.existsSync(script) && spawnSync('python3', ['--version']).status === 0;
  it.skipIf(!havePy)('reads a pack that pack_seed.py itself wrote', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'app0-seedpack-'));
    try {
      const sha = (b: Buffer) => createHash('sha256').update(b).digest('hex');
      const a = Buffer.from('file a\n'), b = Buffer.from([0, 1, 2, 3, 250, 251]), x = Buffer.from('registry seed\n');
      fs.mkdirSync(path.join(dir, 'root/Data'), { recursive: true });
      fs.writeFileSync(path.join(dir, 'root/Data/a.txt'), a);
      fs.writeFileSync(path.join(dir, 'root/Data/b.bin'), b);
      fs.writeFileSync(path.join(dir, 'reg.tsv'), x);
      fs.writeFileSync(path.join(dir, 'list.tsv'), `/Data/a.txt\tData/a.txt\t${sha(a)}\n/Data/b.bin\tData/b.bin\t${sha(b)}\n/dir\t-\t-\n`);
      const r = spawnSync('python3', [script, '--root', path.join(dir, 'root'), '--tsv', path.join(dir, 'list.tsv'), '--extra', path.join(dir, 'reg.tsv'), '--out', path.join(dir, 'out.pack')], { encoding: 'utf8', env: { ...process.env, PYTHONDONTWRITEBYTECODE: '1' } });
      expect(r.status, r.stderr).toBe(0);
      const pack = new Uint8Array(fs.readFileSync(path.join(dir, 'out.pack')));
      const out = parsePack(pack, { verifyFiles: true, format: 'flseedp1' });
      expect(Buffer.from(out.get('seed/Data/a.txt')!.data).equals(a)).toBe(true);
      expect(Buffer.from(out.get('seed/Data/b.bin')!.data).equals(b)).toBe(true);
      expect(Buffer.from(out.get('seed/reg.tsv')!.data).equals(x)).toBe(true);
      expect(out.has('seed/list.tsv')).toBe(true);
    } finally { fs.rmSync(dir, { recursive: true, force: true }); }
  });
});
