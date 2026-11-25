import zlib from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { encodePngRgba, pngDataUrl } from '../../src/ui/png.ts';

/** the pixels of a PNG the way a reader takes them: the IDAT chunks inflated, the filter byte of each row (0 is required here) removed */
function readRgba(png: Uint8Array): { width: number; height: number; rgba: number[] } {
  const b = Buffer.from(png);
  const width = b.readUInt32BE(16), height = b.readUInt32BE(20);
  let p = 8;
  const idat: Buffer[] = [];
  while (p < b.length) {
    const len = b.readUInt32BE(p), type = b.toString('ascii', p + 4, p + 8);
    if (type === 'IDAT') idat.push(b.subarray(p + 8, p + 8 + len));
    p += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const rgba: number[] = [];
  for (let y = 0; y < height; y++) {
    expect(raw[y * (width * 4 + 1)]).toBe(0);
    for (let i = 0; i < width * 4; i++) rgba.push(raw[y * (width * 4 + 1) + 1 + i] ?? -1);
  }
  return { width, height, rgba };
}

describe('the PNG of a cursor picture', () => {
  it('writes the same pixels back, row by row, with the signature and the IHDR of 8-bit RGBA', () => {
    const px = Uint8Array.from([1, 2, 3, 255, 0, 0, 0, 0, 9, 8, 7, 128, 255, 255, 255, 255, 4, 5, 6, 7, 10, 11, 12, 13]);
    const png = encodePngRgba(3, 2, px);
    expect(Array.from(png.subarray(0, 8))).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
    expect(Buffer.from(png).toString('ascii', 12, 16)).toBe('IHDR');
    expect(png[24]).toBe(8);
    expect(png[25]).toBe(6);
    expect(readRgba(png)).toEqual({ width: 3, height: 2, rgba: Array.from(px) });
  });
  it('a picture longer than one stored block (65535 bytes) is read back whole', () => {
    const w = 128, h = 200;
    const px = new Uint8Array(w * h * 4);
    for (let i = 0; i < px.length; i++) px[i] = (i * 37) & 0xff;
    expect(readRgba(encodePngRgba(w, h, px)).rgba).toEqual(Array.from(px));
  });
  it('a size that does not match the pixels is refused', () => {
    expect(() => encodePngRgba(2, 2, new Uint8Array(15))).toThrow(/15 bytes/);
  });
  it('the data URL is a PNG in base64', () => {
    const url = pngDataUrl(1, 1, Uint8Array.from([0, 0, 0, 0]));
    expect(url.startsWith('data:image/png;base64,')).toBe(true);
    const bytes = Buffer.from(url.slice('data:image/png;base64,'.length), 'base64');
    expect(readRgba(new Uint8Array(bytes))).toEqual({ width: 1, height: 1, rgba: [0, 0, 0, 0] });
  });
});
