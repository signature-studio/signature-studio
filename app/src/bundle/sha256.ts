// Incremental SHA-256 (FIPS 180-4). WebCrypto's digest() takes the whole input at once; the engine wasm is hundreds of MB and is streamed into
// WebAssembly.compileStreaming, so the hash is computed on the chunks as they pass. Checked against node:crypto in test/unit/sha256.test.ts.

const K = new Uint32Array([
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
]);

export class Sha256 {
  private readonly h = new Uint32Array([0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]);
  private readonly block = new Uint8Array(64);
  private readonly w = new Uint32Array(64);
  private blockLen = 0;
  private total = 0;
  private done = false;

  update(data: Uint8Array): this {
    if (this.done) throw new Error('Sha256: update after digest');
    let i = 0;
    const n = data.length;
    this.total += n;
    if (this.blockLen > 0) {
      const take = Math.min(64 - this.blockLen, n);
      this.block.set(data.subarray(0, take), this.blockLen);
      this.blockLen += take;
      i = take;
      if (this.blockLen === 64) { this.compress(this.block, 0); this.blockLen = 0; }
    }
    for (; i + 64 <= n; i += 64) this.compress(data, i);
    if (i < n) { this.block.set(data.subarray(i), 0); this.blockLen = n - i; }
    return this;
  }

  digest(): Uint8Array {
    if (this.done) throw new Error('Sha256: digest twice');
    this.done = true;
    const bits = this.total * 8;
    const pad = new Uint8Array(this.blockLen < 56 ? 64 : 128);
    pad.set(this.block.subarray(0, this.blockLen), 0);
    pad[this.blockLen] = 0x80;
    const dv = new DataView(pad.buffer);
    dv.setUint32(pad.length - 8, Math.floor(bits / 0x100000000), false);
    dv.setUint32(pad.length - 4, bits >>> 0, false);
    for (let o = 0; o < pad.length; o += 64) this.compress(pad, o);
    const out = new Uint8Array(32);
    const ov = new DataView(out.buffer);
    for (let k = 0; k < 8; k++) ov.setUint32(k * 4, this.h[k] ?? 0, false);
    return out;
  }

  hex(): string { return toHex(this.digest()); }

  private compress(b: Uint8Array, o: number): void {
    const w = this.w;
    for (let t = 0; t < 16; t++, o += 4) w[t] = ((b[o] ?? 0) << 24) | ((b[o + 1] ?? 0) << 16) | ((b[o + 2] ?? 0) << 8) | (b[o + 3] ?? 0);
    for (let t = 16; t < 64; t++) {
      const x = w[t - 15] as number, y = w[t - 2] as number;
      const s0 = ((x >>> 7) | (x << 25)) ^ ((x >>> 18) | (x << 14)) ^ (x >>> 3);
      const s1 = ((y >>> 17) | (y << 15)) ^ ((y >>> 19) | (y << 13)) ^ (y >>> 10);
      w[t] = ((w[t - 16] as number) + s0 + (w[t - 7] as number) + s1) | 0;
    }
    const h = this.h;
    let a = h[0] as number, bb = h[1] as number, c = h[2] as number, d = h[3] as number, e = h[4] as number, f = h[5] as number, g = h[6] as number, hh = h[7] as number;
    for (let t = 0; t < 64; t++) {
      const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
      const ch = (e & f) ^ (~e & g);
      const t1 = (hh + S1 + ch + (K[t] as number) + (w[t] as number)) | 0;
      const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
      const maj = (a & bb) ^ (a & c) ^ (bb & c);
      const t2 = (S0 + maj) | 0;
      hh = g; g = f; f = e; e = (d + t1) | 0; d = c; c = bb; bb = a; a = (t1 + t2) | 0;
    }
    h[0] = (h[0] as number) + a; h[1] = (h[1] as number) + bb; h[2] = (h[2] as number) + c; h[3] = (h[3] as number) + d;
    h[4] = (h[4] as number) + e; h[5] = (h[5] as number) + f; h[6] = (h[6] as number) + g; h[7] = (h[7] as number) + hh;
  }
}

export function toHex(u8: Uint8Array): string {
  let s = '';
  for (const b of u8) s += b.toString(16).padStart(2, '0');
  return s;
}

export function sha256HexSync(data: Uint8Array): string {
  return new Sha256().update(data).hex();
}

// One-shot hash through WebCrypto (used for small inputs and for canvas read-backs, where the browser's own implementation is the independent check).
export async function sha256Hex(data: Uint8Array): Promise<string> {
  const copy = new Uint8Array(data.byteLength);
  copy.set(data);
  return toHex(new Uint8Array(await crypto.subtle.digest('SHA-256', copy)));
}
