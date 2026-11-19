import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { crc32, decodeEntryName, entryStream, readZipDirectory, ZipError } from '../../src/files/zip-read.ts';
import { makeZip, unicodePathExtra } from '../helpers/zipmake.ts';

const blobOf = (b: Buffer): Blob => new Blob([new Uint8Array(b)]);
// Shift-JIS bytes of 'クリスマス' and the Delphi notation of U+3099 (what the reference zip's writer produced)
const SJIS_KURISUMASU = Buffer.from([0x83, 0x4e, 0x83, 0x8a, 0x83, 0x58, 0x83, 0x7d, 0x83, 0x58]);
const rawCp = Buffer.concat([Buffer.from('v - '), SJIS_KURISUMASU, Buffer.from('#$3099'), Buffer.from([0x81, 0x5b]), Buffer.from('.wav')]);
const wanted = 'v - クリスマス\u3099ー.wav';

async function readAll(blob: Blob, i: number): Promise<Buffer> {
  const dir = await readZipDirectory(blob);
  const chunks: Uint8Array[] = [];
  const r = (await entryStream(blob, dir[i]!)).getReader();
  for (;;) { const { done, value } = await r.read(); if (done) break; chunks.push(value); }
  return Buffer.concat(chunks);
}

describe('zip entry names', () => {
  it('uses UTF-8 when the flag is set', () => {
    expect(decodeEntryName(Buffer.from('日本語.wav'), 0x800, new Uint8Array(0))).toEqual({ name: '日本語.wav', source: 'utf8-flag' });
  });
  it('uses the Info-ZIP Unicode Path field only when its CRC is that of the header name', () => {
    const extra = unicodePathExtra(rawCp, wanted);
    expect(decodeEntryName(rawCp, 0, extra)).toEqual({ name: wanted, source: 'unicode-extra' });
    const wrong = unicodePathExtra(Buffer.from('other'), 'ignored.wav');
    expect(decodeEntryName(rawCp, 0, wrong).source).toBe('cp932');
  });
  it("falls back to Windows-31J and turns the writer's #$XXXX notation back into the UTF-16 unit", () => {
    expect(decodeEntryName(rawCp, 0, new Uint8Array(0))).toEqual({ name: wanted, source: 'cp932' });
    expect(decodeEntryName(Buffer.from('plain.wav'), 0, new Uint8Array(0))).toEqual({ name: 'plain.wav', source: 'ascii' });
  });
  it('crc32 matches the standard value', () => { expect(crc32(Buffer.from('123456789'))).toBe(0xcbf43926); });
});

describe('readZipDirectory / entryStream', () => {
  const data = Buffer.from('hello hello hello hello'.repeat(50));
  it('reads stored and deflated entries and their names', async () => {
    const zip = makeZip([
      { name: Buffer.from('a.txt'), data, method: 8 },
      { name: Buffer.from('dir/'), data: Buffer.alloc(0), method: 0 },
      { name: rawCp, data: Buffer.from('xyz'), method: 0, extra: unicodePathExtra(rawCp, wanted) },
    ]);
    const blob = blobOf(zip);
    const dir = await readZipDirectory(blob);
    expect(dir.map((e) => [e.name, e.isDir, e.method, e.size])).toEqual([['a.txt', false, 8, data.length], ['dir/', true, 0, 0], [wanted, false, 0, 3]]);
    expect((await readAll(blob, 0)).equals(data)).toBe(true);
    expect((await readAll(blob, 2)).toString()).toBe('xyz');
  });
  it('rejects what is not a zip', async () => {
    await expect(readZipDirectory(blobOf(Buffer.from('not a zip at all, long enough to be looked at')))).rejects.toThrow(ZipError);
    await expect(readZipDirectory(blobOf(Buffer.alloc(5)))).rejects.toThrow(ZipError);
  });
  it('refuses an unsupported method and an encrypted entry when the data is asked for', async () => {
    const zip = makeZip([{ name: Buffer.from('x'), data: Buffer.from('abc'), method: 0 }]);
    const dir = await readZipDirectory(blobOf(zip));
    await expect(entryStream(blobOf(zip), { ...dir[0]!, method: 12 })).rejects.toThrow(/not supported/);
    await expect(entryStream(blobOf(zip), { ...dir[0]!, encrypted: true })).rejects.toThrow(/encrypted/);
  });
  it('reads the zip with a comment after the end record', async () => {
    const zip = makeZip([{ name: Buffer.from('c.txt'), data: Buffer.from('c'), method: 0 }]);
    const withComment = Buffer.concat([zip.subarray(0, zip.length - 2), Buffer.from([4, 0]), Buffer.from('note')]);
    expect((await readZipDirectory(blobOf(withComment))).map((e) => e.name)).toEqual(['c.txt']);
  });
});

const REF = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
describe.skipIf(!fs.existsSync(REF))('the reference project zip (~/ダウンロード/serina1-serum2.zip, decisions 0178/0193)', () => {
  it('has 49 entries; both Japanese names come from the Unicode field (the exact names the FLP holds); the CP932 + #$3099 fallback is LOSSY for them (measured: it differs)', async () => {
    const buf = fs.readFileSync(REF);
    const dir = await readZipDirectory(blobOf(buf));
    expect(dir.length).toBe(49);
    const jp = dir.filter((e) => /[^\x00-\x7f]/.test(e.name));
    expect(jp.length).toBe(2);
    expect(jp.every((e) => e.nameSource === 'unicode-extra')).toBe(true);
    expect(jp[0]!.name.startsWith('[fadr.com] Vocals - クリスマスセリナ')).toBe(true);
    // the FLP refers to every audio file by its name in UTF-16LE: the names read here must all be in it (an oracle that does not use this code's decoding)
    const flp = dir.find((x) => x.name.endsWith('.flp'))!;
    const flpBytes = await readAll(blobOf(buf), flp.index);
    for (const x of dir) if (!x.isDir && !x.name.endsWith('.flp')) expect(flpBytes.includes(Buffer.from(x.name, 'utf16le')), x.name).toBe(true);
    // the same header bytes without the extra field -> the fallback: the writer of this zip lost characters in its own code-page conversion (メモリアルロヒ, 質 ... are missing, U+3099 became '#$3099'), so the fallback cannot give the real name
    const ev = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
    const e = buf.lastIndexOf(Buffer.from([0x50, 0x4b, 5, 6]));
    let p = ev.getUint32(e + 16, true);
    let checked = 0;
    for (let i = 0; i < 49; i++) {
      const fnl = ev.getUint16(p + 28, true), exl = ev.getUint16(p + 30, true), cml = ev.getUint16(p + 32, true);
      if (exl > 0) {
        const got = decodeEntryName(buf.subarray(p + 46, p + 46 + fnl), ev.getUint16(p + 8, true), new Uint8Array(0));
        expect(got.source).toBe('cp932');
        expect(got.name).not.toBe(dir[i]!.name);
        expect(got.name).not.toContain('#$');
        expect(got.name.startsWith('[fadr.com] Vocals - クリスマスセリナ')).toBe(true);
        checked++;
      }
      p += 46 + fnl + exl + cml;
    }
    expect(checked).toBe(2);
  });
});
