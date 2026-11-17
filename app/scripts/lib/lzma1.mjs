// LZMA1 decoder for the streams of an NSIS installer: a raw stream whose 5 property bytes (lc, lp, pb, dictionary size) come first. It decodes to the end marker, or to `limit` bytes.
// The dictionary is the output itself (every byte is kept), so no distance can reach before the start.
// Same output as Python's lzma with FORMAT_RAW and FILTER_LZMA1 (the decoder of tools/extract_nsis2.py): checked file by file in evidence/R6-37.md.
const TOP = 0x1000000;

class RangeDecoder {
  constructor(src) {
    this.src = src;
    this.p = 0;
    this.range = 0xffffffff;
    this.code = 0;
    if (this.byte() !== 0) throw new Error('LZMA: the first byte of the range coder is not 0');
    for (let i = 0; i < 4; i++) this.code = this.code * 256 + this.byte();
  }

  byte() {
    if (this.p >= this.src.length) throw new Error('LZMA: unexpected end of input');
    return this.src[this.p++];
  }

  bit(probs, i) {
    const p = probs[i];
    const bound = (this.range >>> 11) * p;
    let b;
    if (this.code < bound) {
      this.range = bound;
      probs[i] = p + ((2048 - p) >>> 5);
      b = 0;
    } else {
      this.range -= bound;
      this.code -= bound;
      probs[i] = p - (p >>> 5);
      b = 1;
    }
    if (this.range < TOP) {
      this.range *= 256;
      this.code = this.code * 256 + this.byte();
    }
    return b;
  }

  direct(n) {
    let res = 0;
    for (let i = 0; i < n; i++) {
      this.range = Math.floor(this.range / 2);
      let b = 0;
      if (this.code >= this.range) {
        this.code -= this.range;
        b = 1;
      }
      res = res * 2 + b;
      if (this.range < TOP) {
        this.range *= 256;
        this.code = this.code * 256 + this.byte();
      }
    }
    return res;
  }

  tree(probs, base, n) {
    let m = 1;
    for (let i = 0; i < n; i++) m = (m << 1) | this.bit(probs, base + m);
    return m - (1 << n);
  }

  reverseTree(probs, base, n) {
    let m = 1, sym = 0;
    for (let i = 0; i < n; i++) {
      const b = this.bit(probs, base + m);
      m = (m << 1) | b;
      sym |= b << i;
    }
    return sym;
  }
}

const probs = (n) => new Uint16Array(n).fill(1024);

function lenCoder() {
  return { choice: probs(2), low: probs(128), mid: probs(128), high: probs(256) };
}

function decodeLen(rc, L, posState) {
  if (rc.bit(L.choice, 0) === 0) return rc.tree(L.low, posState << 3, 3);
  if (rc.bit(L.choice, 1) === 0) return 8 + rc.tree(L.mid, posState << 3, 3);
  return 16 + rc.tree(L.high, 0, 8);
}

function decodeDist(rc, M, len) {
  const slot = rc.tree(M.slot, (len < 3 ? len : 3) << 6, 6);
  if (slot < 4) return slot;
  const nd = (slot >>> 1) - 1;
  let dist = (2 | (slot & 1)) * 2 ** nd;
  if (slot < 14) dist += rc.reverseTree(M.pos, dist - slot, nd);
  else dist += rc.direct(nd - 4) * 16 + rc.reverseTree(M.align, 0, 4);
  return dist;
}

/** the decoded bytes of the LZMA1 stream `src` (without its property bytes), props = its 5 property bytes */
export function lzmaDecode(props, src, limit = Infinity) {
  let d = props[0];
  if (d >= 225) throw new Error('LZMA: bad property byte');
  const lc = d % 9;
  d = Math.floor(d / 9);
  const lp = d % 5;
  const pb = Math.floor(d / 5);
  const rc = new RangeDecoder(src);
  const lit = probs(0x300 << (lc + lp));
  const M = { slot: probs(256), pos: probs(115), align: probs(16) };
  const isMatch = probs(192), isRep = probs(12), isRepG0 = probs(12), isRepG1 = probs(12), isRepG2 = probs(12), isRep0Long = probs(192);
  const lenC = lenCoder(), repLenC = lenCoder();
  const pbMask = (1 << pb) - 1, lpMask = (1 << lp) - 1;
  let out = Buffer.alloc(1 << 16), n = 0;
  const put = (b) => {
    if (n === out.length) {
      const g = Buffer.alloc(out.length * 2);
      out.copy(g);
      out = g;
    }
    out[n++] = b;
  };
  let state = 0, rep0 = 0, rep1 = 0, rep2 = 0, rep3 = 0;
  while (n < limit) {
    const posState = n & pbMask;
    if (rc.bit(isMatch, (state << 4) + posState) === 0) {
      const prev = n > 0 ? out[n - 1] : 0;
      const base = 0x300 * (((n & lpMask) << lc) + (prev >> (8 - lc)));
      let sym = 1;
      if (state >= 7) {
        let mb = out[n - rep0 - 1];
        do {
          const mbit = (mb >> 7) & 1;
          mb <<= 1;
          const b = rc.bit(lit, base + ((1 + mbit) << 8) + sym);
          sym = (sym << 1) | b;
          if (mbit !== b) break;
        } while (sym < 0x100);
      }
      while (sym < 0x100) sym = (sym << 1) | rc.bit(lit, base + sym);
      put(sym & 0xff);
      state = state < 4 ? 0 : state < 10 ? state - 3 : state - 6;
      continue;
    }
    let len;
    if (rc.bit(isRep, state) === 1) {
      if (n === 0) throw new Error('LZMA: a repeat at position 0');
      if (rc.bit(isRepG0, state) === 0) {
        if (rc.bit(isRep0Long, (state << 4) + posState) === 0) {
          state = state < 7 ? 9 : 11;
          put(out[n - rep0 - 1]);
          continue;
        }
      } else {
        let dist;
        if (rc.bit(isRepG1, state) === 0) {
          dist = rep1;
        } else {
          if (rc.bit(isRepG2, state) === 0) {
            dist = rep2;
          } else {
            dist = rep3;
            rep3 = rep2;
          }
          rep2 = rep1;
        }
        rep1 = rep0;
        rep0 = dist;
      }
      len = decodeLen(rc, repLenC, posState);
      state = state < 7 ? 8 : 11;
    } else {
      rep3 = rep2;
      rep2 = rep1;
      rep1 = rep0;
      len = decodeLen(rc, lenC, posState);
      state = state < 7 ? 7 : 10;
      const dist = decodeDist(rc, M, len);
      if (dist === 0xffffffff) break;
      rep0 = dist;
    }
    len += 2;
    for (let k = 0; k < len && n < limit; k++) put(out[n - rep0 - 1]);
  }
  return out.subarray(0, n);
}
