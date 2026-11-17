// Test data for the file manager e2e (decision 0216, opfs1): generated deterministically into a directory under the work dir (not /tmp: a snap Firefox cannot read /tmp).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { makeZip, unicodePathExtra } from '../helpers/zipmake.ts';

const rnd = (seed, n) => { const b = Buffer.alloc(n); let x = seed >>> 0 || 1; for (let i = 0; i < n; i++) { x ^= x << 13; x >>>= 0; x ^= x >>> 17; x ^= x << 5; x >>>= 0; b[i] = x & 255; } return b; };
export const sha = (b) => crypto.createHash('sha256').update(b).digest('hex');

export function makeTestData(dir) {
  fs.rmSync(dir, { recursive: true, force: true });
  const put = (rel, buf) => { const p = path.join(dir, rel); fs.mkdirSync(path.dirname(p), { recursive: true }); fs.writeFileSync(p, buf); return p; };
  const files = {};
  const add = (rel, buf) => { files[rel] = { path: put(rel, buf), size: buf.length, sha: sha(buf) }; };
  add('loose/readme.txt', Buffer.from('hello\n'));
  add('loose/日本語 サンプル.wav', rnd(1, 40000));
  add('proj/Song.flp', Buffer.concat([Buffer.from('FLhd'), rnd(2, 3000)]));
  add('proj/Samples/kick.wav', rnd(3, 70000));
  add('proj/Samples/deep/鳴る [x].wav', rnd(4, 1 << 20));
  add('proj/Samples/deep/b.bin', rnd(5, 5000));
  // zips with chosen raw names
  const sjis = Buffer.from([0x83, 0x4e, 0x83, 0x8a, 0x83, 0x58, 0x83, 0x7d, 0x83, 0x58]); // クリスマス in Shift-JIS
  const rawCp = Buffer.concat([Buffer.from('s - '), sjis, Buffer.from('.wav')]);
  const data = (n) => rnd(100 + n, 10000 + n * 100);
  const d = [data(1), data(2), data(3), data(4)];
  put('zips/utf8.zip', makeZip([{ name: Buffer.from('P/曲.flp'), data: d[0], flags: 0x800 }, { name: Buffer.from('P/Samples/音.wav'), data: d[1], flags: 0x800, method: 0 }, { name: Buffer.from('P/Empty/'), data: Buffer.alloc(0), method: 0, flags: 0x800 }]));
  put('zips/cp932.zip', makeZip([{ name: rawCp, data: d[2] }, { name: Buffer.from('plain.txt'), data: d[3] }]));
  put('zips/unicode-extra.zip', makeZip([{ name: rawCp, data: d[2], extra: unicodePathExtra(rawCp, 's - クリスマス.wav') }]));
  put('zips/evil.zip', makeZip([
    { name: Buffer.from('../escape.txt'), data: d[0] }, { name: Buffer.from('/abs.txt'), data: d[1] }, { name: Buffer.from('C:\\drive.txt'), data: d[1] },
    { name: Buffer.from('ok/fine.txt'), data: d[2] }, { name: Buffer.from('bad\\sep/win.txt'), data: d[3] }, { name: Buffer.from('we:ird*name?.txt'), data: d[3] }, { name: Buffer.from('a/../../up.txt'), data: d[0] },
  ]));
  put('zips/badcrc.zip', makeZip([{ name: Buffer.from('good.bin'), data: d[0] }, { name: Buffer.from('bad.bin'), data: d[1], crc: 0x12345678 }, { name: Buffer.from('after.bin'), data: d[2] }]));
  put('zips/unsupported.zip', makeZip([{ name: Buffer.from('bz.bin'), data: d[0], method: 12 }, { name: Buffer.from('enc.bin'), data: d[1], flags: 1 }, { name: Buffer.from('ok.bin'), data: d[2] }]));
  const many = []; for (let i = 0; i < 24; i++) many.push({ name: Buffer.from(`many/f${String(i).padStart(2, '0')}.bin`), data: rnd(500 + i, 3 << 20), method: 0 });
  put('zips/many.zip', makeZip(many));
  return { dir, files, zips: fs.readdirSync(path.join(dir, 'zips')).map((n) => path.join(dir, 'zips', n)), expected: { d: d.map(sha) } };
}
