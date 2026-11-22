// LZMA2 decoder for the file data of an Inno Setup installer (CompressMethod cmLZMA2): the chunks that the LZMA SDK's Lzma2Dec reads
// (control byte, sizes, props, state / dictionary resets). The LZMA core is the one of app/scripts/lib/lzma1.mjs, but its state is kept across the chunks.
// The output is kept whole, so the dictionary is the output (dictStart marks the last dictionary reset, as in the SDK's processedPos).
const TOP = 0x1000000;

const probs = (n) => new Uint16Array(n).fill(1024);

class RangeDecoder {
  constructor(src, p, end) {
    this.src = src;
    this.p = p;
    this.end = end;
    this.range = 0xffffffff;
    this.code = 0;
    if (this.byte() !== 0) throw new Error('LZMA2: the first byte of a range coder is not 0');
    for (let i = 0; i < 4; i++) this.code = this.code * 256 + this.byte();
  }

  byte() {
    if (this.p >= this.end) throw new Error('LZMA2: unexpected end of a chunk');
    return this.src[this.p++];
  }

  bit(pr, i) {
    const p = pr[i];
    const bound = (this.range >>> 11) * p;
    let b;
    if (this.code < bound) {
      this.range = bound;
      pr[i] = p + ((2048 - p) >>> 5);
      b = 0;
    } else {
      this.range -= bound;
      this.code -= bound;
      pr[i] = p - (p >>> 5);
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

  tree(pr, base, n) {
    let m = 1;
    for (let i = 0; i < n; i++) m = (m << 1) | this.bit(pr, base + m);
    return m - (1 << n);
  }

  reverseTree(pr, base, n) {
    let m = 1, sym = 0;
    for (let i = 0; i < n; i++) {
      const b = this.bit(pr, base + m);
      m = (m << 1) | b;
      sym |= b << i;
    }
    return sym;
  }
}

const lenCoder = () => ({ choice: probs(2), low: probs(128), mid: probs(128), high: probs(256) });

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

class Lzma2Decoder {
  constructor() {
    this.out = Buffer.alloc(1 << 20);
    this.n = 0;
    this.dictStart = 0;
    this.props = null;
    this.state = null;
  }

  reserve(extra) {
    let size = this.out.length;
    while (this.n + extra > size) size *= 2;
    if (size !== this.out.length) {
      const g = Buffer.alloc(size);
      this.out.copy(g, 0, 0, this.n);
      this.out = g;
    }
  }

  copyStored(src, p, size) {
    this.reserve(size);
    src.copy(this.out, this.n, p, p + size);
    this.n += size;
  }

  setProps(b) {
    if (b >= 225) throw new Error('LZMA2: bad property byte');
    const lc = b % 9;
    const lp = Math.floor(b / 9) % 5;
    const pb = Math.floor(b / 45);
    if (lc + lp > 4) throw new Error('LZMA2: lc + lp > 4');
    this.props = { lc, lp, pb };
  }

  resetState() {
    if (!this.props) throw new Error('LZMA2: an LZMA chunk before its properties');
    const { lc, lp } = this.props;
    this.state = {
      lit: probs(0x300 << (lc + lp)),
      M: { slot: probs(256), pos: probs(115), align: probs(16) },
      isMatch: probs(192), isRep: probs(12), isRepG0: probs(12), isRepG1: probs(12), isRepG2: probs(12), isRep0Long: probs(192),
      lenC: lenCoder(), repLenC: lenCoder(),
      st: 0, rep0: 0, rep1: 0, rep2: 0, rep3: 0,
    };
  }

  put(b) {
    this.reserve(1);
    this.out[this.n++] = b;
  }

  decodeChunk(src, start, end, target) {
    const rc = new RangeDecoder(src, start, end);
    const { lc, lp, pb } = this.props;
    const pbMask = (1 << pb) - 1, lpMask = (1 << lp) - 1;
    const s = this.state;
    while (this.n < target) {
      const pos = this.n - this.dictStart;
      const posState = pos & pbMask;
      if (rc.bit(s.isMatch, (s.st << 4) + posState) === 0) {
        const prev = pos > 0 ? this.out[this.n - 1] : 0;
        const base = 0x300 * (((pos & lpMask) << lc) + (prev >> (8 - lc)));
        let sym = 1;
        if (s.st >= 7) {
          let mb = this.out[this.n - s.rep0 - 1];
          do {
            const mbit = (mb >> 7) & 1;
            mb <<= 1;
            const b = rc.bit(s.lit, base + ((1 + mbit) << 8) + sym);
            sym = (sym << 1) | b;
            if (mbit !== b) break;
          } while (sym < 0x100);
        }
        while (sym < 0x100) sym = (sym << 1) | rc.bit(s.lit, base + sym);
        this.put(sym & 0xff);
        s.st = s.st < 4 ? 0 : s.st < 10 ? s.st - 3 : s.st - 6;
        continue;
      }
      let len;
      if (rc.bit(s.isRep, s.st) === 1) {
        if (pos === 0) throw new Error('LZMA2: a repeat at the start of a dictionary');
        if (rc.bit(s.isRepG0, s.st) === 0) {
          if (rc.bit(s.isRep0Long, (s.st << 4) + posState) === 0) {
            s.st = s.st < 7 ? 9 : 11;
            this.put(this.out[this.n - s.rep0 - 1]);
            continue;
          }
        } else {
          let dist;
          if (rc.bit(s.isRepG1, s.st) === 0) {
            dist = s.rep1;
          } else {
            if (rc.bit(s.isRepG2, s.st) === 0) {
              dist = s.rep2;
            } else {
              dist = s.rep3;
              s.rep3 = s.rep2;
            }
            s.rep2 = s.rep1;
          }
          s.rep1 = s.rep0;
          s.rep0 = dist;
        }
        len = decodeLen(rc, s.repLenC, posState);
        s.st = s.st < 7 ? 8 : 11;
      } else {
        s.rep3 = s.rep2;
        s.rep2 = s.rep1;
        s.rep1 = s.rep0;
        len = decodeLen(rc, s.lenC, posState);
        s.st = s.st < 7 ? 7 : 10;
        const dist = decodeDist(rc, s.M, len);
        if (dist === 0xffffffff) throw new Error('LZMA2: an end marker inside a chunk');
        s.rep0 = dist;
      }
      if (s.rep0 >= pos) throw new Error('LZMA2: a match before the dictionary');
      len += 2;
      for (let k = 0; k < len && this.n < target; k++) this.put(this.out[this.n - s.rep0 - 1]);
    }
  }
}

/** the bytes that the LZMA2 stream `src` decodes to: its first byte is the dictionary size, then the chunks, ended by a 0 control byte */
export function lzma2Decode(src) {
  const d = new Lzma2Decoder();
  let p = 1;
  for (;;) {
    if (p >= src.length) throw new Error('LZMA2: the stream ends without its end byte');
    const c = src[p++];
    if (c === 0) break;
    if (c === 1 || c === 2) {
      if (p + 2 > src.length) throw new Error('LZMA2: truncated stored chunk');
      const size = src.readUInt16BE(p) + 1;
      p += 2;
      if (c === 1) d.dictStart = d.n;
      if (p + size > src.length) throw new Error('LZMA2: truncated stored chunk');
      d.copyStored(src, p, size);
      p += size;
      continue;
    }
    if (c < 0x80) throw new Error(`LZMA2: bad control byte ${c}`);
    if (p + 4 > src.length) throw new Error('LZMA2: truncated LZMA chunk header');
    const unpack = ((c & 0x1f) << 16) + src.readUInt16BE(p) + 1;
    const pack = src.readUInt16BE(p + 2) + 1;
    p += 4;
    const mode = (c >> 5) & 3;
    if (mode >= 2) d.setProps(src[p++]);
    if (mode === 3) d.dictStart = d.n;
    if (mode >= 1 || !d.state) d.resetState();
    if (p + pack > src.length) throw new Error('LZMA2: truncated LZMA chunk');
    d.decodeChunk(src, p, p + pack, d.n + unpack);
    p += pack;
  }
  return d.out.subarray(0, d.n);
}
