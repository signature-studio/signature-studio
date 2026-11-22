import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { afterAll, describe, expect, it } from 'vitest';

// Issue #1315: pack-seed.mjs --lazy leaves a host file of the seed list out of the pack; the file is still checked against the list, and named in the side manifest.
const SCRIPT = path.resolve(import.meta.dirname, '../../scripts/pack-seed.mjs');
const tmp = fs.mkdtempSync(path.join(process.env.TMPDIR ?? os.tmpdir(), 'packseedlazy-'));
afterAll(() => fs.rmSync(tmp, { recursive: true, force: true }));
const sha = (b: Buffer) => crypto.createHash('sha256').update(b).digest('hex');

const root = path.join(tmp, 'host');
const alpha = Buffer.from('alpha-bytes'), beta = Buffer.from('beta-bytes-2');
fs.mkdirSync(path.join(root, 'lift', 'idle'), { recursive: true });
fs.writeFileSync(path.join(root, 'lift', 'idle', 'alpha.bin'), alpha);
fs.writeFileSync(path.join(root, 'lift', 'idle', 'beta.bin'), beta);
const list = (betaSha: string) => `# a list\nC:\\app\\alpha.bin\tlift/idle/alpha.bin\t${sha(alpha)}\nC:\\app\\beta.bin\tlift/idle/beta.bin\t${betaSha}\n`;
const tsv = path.join(tmp, 'list.tsv');
fs.writeFileSync(tsv, list(sha(beta)));
const badTsv = path.join(tmp, 'bad-list.tsv');
fs.writeFileSync(badTsv, list(sha(alpha)));

const pack = (out: string, extra: string[] = [], listFile = tsv) =>
  spawnSync(process.execPath, [SCRIPT, '--root', root, '--tsv', listFile, '--out', out, ...extra], { encoding: 'utf8' });
/** the guest paths of the files in a FLSEEDP1 pack */
const packed = (file: string): string[] => {
  const buf = fs.readFileSync(file);
  const n = buf.readUInt32LE(8);
  return (JSON.parse(buf.subarray(12, 12 + n).toString('utf8')) as { files: { path: string }[] }).files.map((f) => f.path);
};

describe('pack-seed.mjs --lazy (Issue #1315)', () => {
  it('packs every host file of the list when no --lazy is given', () => {
    const out = path.join(tmp, 'all.bin');
    const r = pack(out);
    expect(r.status, r.stderr).toBe(0);
    expect(packed(out)).toEqual(['seed/list.tsv', 'seed/lift/idle/alpha.bin', 'seed/lift/idle/beta.bin']);
  });

  it('leaves the --lazy file out of the pack and names it in the side manifest, with its size and SHA-256', () => {
    const out = path.join(tmp, 'lazy.bin');
    const r = pack(out, ['--lazy', 'lift/idle/beta.bin']);
    expect(r.status, r.stderr).toBe(0);
    expect(packed(out)).toEqual(['seed/list.tsv', 'seed/lift/idle/alpha.bin']);
    const man = JSON.parse(fs.readFileSync(`${out}.manifest.json`, 'utf8'));
    expect(man.left_out).toEqual([{ path: 'seed/lift/idle/beta.bin', host: 'lift/idle/beta.bin', size: beta.length, sha256: sha(beta) }]);
    expect(man.files_checked_against_list_sha256).toBe(2);
  });

  it('refuses a --lazy file that the list does not name', () => {
    const r = pack(path.join(tmp, 'none.bin'), ['--lazy', 'lift/idle/gamma.bin']);
    expect(r.status).not.toBe(0);
    expect(r.stderr).toContain('--lazy lift/idle/gamma.bin: not a host file of');
  });

  it('refuses a --lazy file whose bytes differ from the list', () => {
    const r = pack(path.join(tmp, 'mismatch.bin'), ['--lazy', 'lift/idle/beta.bin'], badTsv);
    expect(r.status).not.toBe(0);
    expect(r.stderr).toContain('SHA-256 mismatch');
  });
});
