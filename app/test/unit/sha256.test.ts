import { createHash, randomBytes } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { Sha256, sha256Hex, sha256HexSync } from '../../src/bundle/sha256.ts';

const ref = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');

describe('Sha256 (incremental)', () => {
  it('matches the published test vectors', () => {
    expect(sha256HexSync(new Uint8Array(0))).toBe('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
    expect(sha256HexSync(new TextEncoder().encode('abc'))).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    expect(sha256HexSync(new TextEncoder().encode('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq'))).toBe('248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1');
  });

  it('agrees with node:crypto for every length around the block boundaries', () => {
    for (let n = 0; n <= 200; n++) {
      const b = new Uint8Array(randomBytes(n));
      expect(sha256HexSync(b), `length ${n}`).toBe(ref(b));
    }
  });

  it('gives the same answer for any chunking of the input', () => {
    const b = new Uint8Array(randomBytes(100_003));
    const want = ref(b);
    for (const step of [1, 3, 63, 64, 65, 1000, 4096, 99_999]) {
      const s = new Sha256();
      for (let i = 0; i < b.length; i += step) s.update(b.subarray(i, i + step));
      expect(s.hex(), `step ${step}`).toBe(want);
    }
  });

  it('rejects update after digest', () => {
    const s = new Sha256();
    s.digest();
    expect(() => s.update(new Uint8Array(1))).toThrow();
  });

  it('WebCrypto one-shot agrees', async () => {
    const b = new Uint8Array(randomBytes(70_000));
    expect(await sha256Hex(b)).toBe(ref(b));
  });
});
