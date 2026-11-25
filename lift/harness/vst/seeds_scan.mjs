#!/usr/bin/env node
// The seeds that a VST3 image needs beyond its .pdata (the node replacement of the seed scans of the older builds; #1367).
// usage: node seeds_scan.mjs PE > SEEDS.txt
// Prints one VA per line (sorted, unique): (a) the case labels of MSVC switch tables, read as `mov r32,[base+idx*4+disp32]` with base = ImageBase and
// ImageBase-relative int32 entries, taken while they lie in the same .pdata function (or in a window of 0x400 around the jump in a leaf function, which has no
// .pdata entry); (b) the targets of `lea r64,[rip+disp32]` in .text. A case label or an address-taken function that is not a .pdata start is a function
// entry only with its seed (fe-lift --seeds); without it the dispatch of the lifted code stops by name (fle_dispatch_miss).
import fs from 'node:fs';

const [pePath] = process.argv.slice(2);
if (!pePath) {
  console.error('usage: node seeds_scan.mjs PE > SEEDS.txt');
  process.exit(64);
}
const b = fs.readFileSync(pePath);
const lfa = b.readUInt32LE(0x3c);
const nsec = b.readUInt16LE(lfa + 6);
const optSize = b.readUInt16LE(lfa + 20);
const opt = lfa + 24;
const imageBase = Number(b.readBigUInt64LE(opt + 24));
const dirs = opt + 112;
const secs = [];
for (let i = 0; i < nsec; i++) {
  const o = opt + optSize + i * 40;
  secs.push({ name: b.toString('latin1', o, o + 8).replace(/\0/g, ''), va: b.readUInt32LE(o + 12), vs: b.readUInt32LE(o + 8), raw: b.readUInt32LE(o + 20), rs: b.readUInt32LE(o + 16) });
}
const toOff = (rva) => {
  const s = secs.find((x) => rva >= x.va && rva < x.va + Math.max(x.vs, x.rs));
  return s ? rva - s.va + s.raw : -1;
};
const exRva = b.readUInt32LE(dirs + 3 * 8);
const exSize = b.readUInt32LE(dirs + 3 * 8 + 4);
const funcs = [];
for (let o = toOff(exRva); o < toOff(exRva) + exSize; o += 12) funcs.push([b.readUInt32LE(o), b.readUInt32LE(o + 4)]);
funcs.sort((x, y) => x[0] - y[0]);
const funcOf = (rva) => {
  let lo = 0, hi = funcs.length - 1, found = null;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (funcs[mid][0] <= rva) { found = funcs[mid]; lo = mid + 1; } else hi = mid - 1;
  }
  return found && rva < found[1] ? found : null;
};

const text = secs.find((s) => s.name === '.text');
const textVA = imageBase + text.va;
const textEnd = textVA + Math.max(text.vs, text.rs);
const inText = (t) => t >= textVA && t < textEnd;
const seeds = new Set();
const base = text.raw;
const end = text.raw + text.rs;

// (a) switch tables: 8b modrm(mod=10, rm=100) SIB(scale=4, index not none) disp32, optionally with a REX prefix
for (let p = base; p + 8 < end; p++) {
  let q = p;
  if ((b[q] & 0xf0) === 0x40) q++;
  if (b[q] !== 0x8b) continue;
  const modrm = b[q + 1], sib = b[q + 2];
  if ((modrm & 0xc7) !== 0x84 || (sib & 0xc0) !== 0x80 || (sib & 0x38) === 0x20) continue;
  const disp = b.readInt32LE(q + 3);
  const jmpRva = p - base + text.va;
  const f = funcOf(jmpRva);
  const lo = f ? imageBase + f[0] : imageBase + jmpRva - 0x400;
  const hi = f ? imageBase + f[1] : imageBase + jmpRva + 0x400;
  const toff = toOff(disp);
  if (toff < 0) continue;
  const got = [];
  for (let i = 0; i < 256; i++) {
    const t = imageBase + b.readInt32LE(toff + 4 * i);
    if (t < lo || t >= hi) break;
    got.push(t);
  }
  if (got.length >= 2) for (const t of got) seeds.add(t);
}

// (c) switch tables relative to their own address (the MSVC form `lea rB,[rip+T]; movsxd rT,[rB+rI*4]; add rT,rB; jmp rT`, the case label is T + the int32 entry;
// fe-decode's jumptab reads only the ImageBase form of (a)). The entry count is the bound of the cmp in front of the table. The same taking as (a): in the .pdata
// function of the jump, at least two targets. REX bits of the base, index and target registers are read (r8-r15 too).
for (let p = base + 24; p + 16 < end; p++) {
  if ((b[p] & 0xf8) !== 0x48 || b[p + 1] !== 0x63) continue;
  const rex = b[p] & 7, modrm = b[p + 2], sib = b[p + 3];
  const mod = modrm >> 6;
  if ((modrm & 0x07) !== 0x04 || mod > 1 || (sib & 0xc0) !== 0x80 || (mod === 0 && (sib & 7) === 5)) continue;
  const disp = mod === 1 ? b.readInt8(p + 4) : 0, ilen = mod === 1 ? 5 : 4;
  const rI = ((sib >> 3) & 7) | ((rex & 2) << 2), rB = (sib & 7) | ((rex & 1) << 3), rT = ((modrm >> 3) & 7) | ((rex & 4) << 1);
  if (((sib >> 3) & 7) === 4 && !(rex & 2)) continue;
  let T = null;
  for (let q = p - 1024; q < p; q++) {
    if (q < base || (b[q] & 0xf8) !== 0x48 || b[q + 1] !== 0x8d || (b[q + 2] & 0xc7) !== 0x05) continue;
    if ((((b[q + 2] >> 3) & 7) | ((b[q] & 4) << 1)) === rB) T = imageBase + text.va + (q + 7 - base) + b.readInt32LE(q + 3);
  }
  if (T === null) continue;
  let jump = false;
  for (let k = p + ilen; k < p + 16 && !jump; k++) {
    if ((b[k] & 0xf8) !== 0x48) continue;
    const addM = b[k + 1] === 0x01 ? b[k + 2] === (0xc0 | ((rB & 7) << 3) | (rT & 7)) : b[k + 1] === 0x03 && b[k + 2] === (0xc0 | ((rT & 7) << 3) | (rB & 7));
    if (addM) for (let j = k + 3; j < k + 16; j++) if (b[j] === 0xff && b[j + 1] === (0xe0 | (rT & 7))) jump = true;
  }
  if (!jump) continue;
  let guard = -1;
  for (let q = p - 128; q < p - 3; q++) {
    if (q < base) continue;
    const g = (b[q] & 0xf0) === 0x40 ? q + 1 : q;
    if (b[g] === 0x83 && (b[g + 1] & 0xf8) === 0xf8) guard = b[g + 2];
  }
  if (guard > 255) continue;
  const jmpRva = p - base + text.va;
  const f = funcOf(jmpRva);
  const lo = f ? imageBase + f[0] : imageBase + jmpRva - 0x400;
  const hi = f ? imageBase + f[1] : imageBase + jmpRva + 0x400;
  const toff = toOff(T + disp - imageBase);
  if (toff < 0) continue;
  const got = [];
  const cap = guard >= 0 ? guard : 31;
  for (let i = 0; i <= cap; i++) {
    const t = T + b.readInt32LE(toff + 4 * i);
    if (t < lo || t >= hi) break;
    got.push(t);
  }
  if (got.length >= 2) for (const t of got) seeds.add(t);
}

// (b) lea r64,[rip+disp32] (REX.W 8d modrm=00xxx101)
for (let p = base; p + 8 <= end; p++) {
  if ((b[p] & 0xf8) !== 0x48 || b[p + 1] !== 0x8d || (b[p + 2] & 0xc7) !== 0x05) continue;
  const disp = b.readInt32LE(p + 3);
  const t = imageBase + text.va + (p + 7 - base) + disp;
  if (inText(t)) seeds.add(t);
}

process.stdout.write([...seeds].sort((x, y) => x - y).map((v) => `0x${v.toString(16)}`).join('\n') + '\n');
console.error(`seeds_scan: ${seeds.size} seeds`);
