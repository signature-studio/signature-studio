"""Decision 0165 (vst3a): the two SHRD / SHLD sites of Serum2.vst3 (outside fe-lift's instruction set) as C, written by hand and checked against the disassembly:
  0x18020c9cf  shrd r10d, eax, 0x16     (state decoding; reached by IComponent::setState)
  0x1804975d8  shld r10, r15, 0x20      (not reached in the runs of this decision)
Intel definition (count masked to 5 / 6 bits; count 0: nothing changes): SHRD: res = (d >> c) | (s << (W-c)), CF = bit c-1 of d; SHLD: res = (d << c) | (s >> (W-c)), CF = bit W-c of d;
SF/ZF/PF from the result; OF (count 1 only) = sign change; AF undefined. usage: patch_shd.py FILE gen.c"""
import re, struct, sys, capstone
fn, gen = sys.argv[1:3]
d = open(fn, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    secs.append((va, vs, rs, rp))
def r2o(r):
    for va, vs, rs, rp in secs:
        if va <= r < va + max(vs, rs): return rp + r - va
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
DELTA = base - 0x180000000   # decision 0177 (srm1): the same two sites in a rebased copy of the PE (wasm needs the image below GLOBAL_BASE); 0 for the original image
SH1, SH2 = 0x18020c9cf + DELTA, 0x1804975d8 + DELTA
def check(va, text):
    o = r2o(va - base)
    i = next(md.disasm(d[o:o + 16], va))
    assert '%s %s' % (i.mnemonic, i.op_str) == text, (hex(va), i.mnemonic, i.op_str)
check(SH1, 'shrd r10d, eax, 0x16')
check(SH2, 'shld r10, r15, 0x20')
PF = '!__builtin_parity((unsigned)(res & 0xff))'
body = {
 SH1: 'Shrd', SH2: 'Shld',
}
c32 = ('{ uint32_t d = (uint32_t)R[10]; uint32_t s = (uint32_t)R[0]; unsigned c = 0x16 & 31; if (c) { uint32_t res = (d >> c) | (s << (32 - c)); FL.cf = (d >> (c - 1)) & 1; '
       'FL.of = (c == 1) ? (((res ^ d) >> 31) & 1) : FLE_UNDEF; FL.zf = (res == 0); FL.sf = res >> 31; FL.pf = ' + PF + '; FL.af = FLE_UNDEF; R[10] = res; } }')
c64 = ('{ uint64_t d = R[10]; uint64_t s = R[15]; unsigned c = 0x20 & 63; if (c) { uint64_t res = (d << c) | (s >> (64 - c)); FL.cf = (d >> (64 - c)) & 1; '
       'FL.of = (c == 1) ? (((res ^ d) >> 63) & 1) : FLE_UNDEF; FL.zf = (res == 0); FL.sf = res >> 63; FL.pf = ' + PF + '; FL.af = FLE_UNDEF; R[10] = res; } }')
txt = {SH1: c32, SH2: c64}
src = open(gen, 'rb').read()
n = 0
for va, nm in body.items():
    pat = re.compile(rb'^(L_%x: \{ /\* 0x%x %s \*/ )fle_unsupported\(UINT64_C\(0x%x\)\); \}$' % (va, va, nm.encode(), va), re.M)
    src, k = pat.subn(lambda m: m.group(1) + txt[va].encode() + b'}', src)
    n += k
open(gen, 'wb').write(src)
print('patch_shd: lines', n)
