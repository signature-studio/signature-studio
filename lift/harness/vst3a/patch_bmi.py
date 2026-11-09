"""Decision 0165 (vst3a): TZCNT (F3 0F BC) and LZCNT (F3 0F BD) are outside the instruction set of fe-lift (584 + 4 sites in Serum2.vst3; clang emits `rep bsf` for cttz, which a
disassembler shows as tzcnt). This produces their C without changing fe-lift: a copy of the PE with the F3 prefix byte of each such site replaced by 2E (a CS override, ignored in 64-bit mode)
decodes as BSF / BSR, which fe-lift lifts; the BSF/BSR line of each site is taken from the lift of that copy and turned into the TZCNT/LZCNT line:
  tzcnt: k = s ? ctz(s) : W ; dst = k ; CF = (s == 0) ; ZF = (k == 0) ; OF = SF = AF = PF = FLE_UNDEF (undefined in the Intel manual; a read stops the run)
  lzcnt: k = s ? W-1-msb(s) : W ; (same flags)
W is the operand width (from the destination of the instruction, by capstone). The Intel definition of the two instructions is used (the machine has BMI1/ABM). Nothing else in the generated C is touched.
usage: patch_bmi.py FILE gen.c FEBIN WORKDIR TAG LIFT_ARGS...   (LIFT_ARGS as for the main lift)"""
import re, struct, subprocess, sys, shutil, os, capstone
fn, gen, febin, work, tag = sys.argv[1:6]
lift_args = sys.argv[6:]
src = open(gen, 'rb').read()
pat = re.compile(rb'^L_([0-9a-f]+): \{ /\* 0x[0-9a-f]+ (Tzcnt|Lzcnt) \*/ fle_unsupported\(UINT64_C\(0x[0-9a-f]+\)\); \}$', re.M)
sites = sorted({(int(m.group(1), 16), m.group(2).decode()) for m in pat.finditer(src)})   # a shared body can be in several functions: one entry per VA
d = bytearray(open(fn, 'rb').read())
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    secs.append((va, vs, rs, rp))
def r2o(rva):
    for va, vs, rs, rp in secs:
        if va <= rva < va + max(vs, rs): return rp + rva - va
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); md.detail = True
width = {}
for va, kind in sites:
    o = r2o(va - base)
    assert d[o] == 0xF3, hex(va)
    ins = next(md.disasm(bytes(d[o:o + 16]), va))
    assert ins.mnemonic == kind.lower(), (hex(va), ins.mnemonic)
    width[va] = ins.operands[0].size * 8
    d[o] = 0x2E
os.makedirs(work, exist_ok=True)
pf = os.path.join(work, 'bmi-as-bsf-%s.pe' % tag)
open(pf, 'wb').write(bytes(d))
out = os.path.join(work, 'bmi-as-bsf-%s.c' % tag)
if os.path.exists(out): os.remove(out)
subprocess.run([os.path.join(febin, 'fe-lift'), 'lift', pf, out] + lift_args, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
alt = {}
lp = re.compile(rb'^L_([0-9a-f]+): \{ /\* 0x[0-9a-f]+ (Bsf|Bsr) \*/ (.*)$', re.M)
want = {va for va, k in sites}
for m in lp.finditer(open(out, 'rb').read()):
    va = int(m.group(1), 16)
    if va in want: alt[va] = (m.group(2).decode(), m.group(3).decode())
missing = want - set(alt)
assert not missing, [hex(v) for v in sorted(missing)[:5]]
nt = nl = 0
def sub(m):
    global nt, nl
    va = int(m.group(1), 16); kind = m.group(2).decode()
    bk, txt = alt[va]
    W = width[va]
    mm = re.match(r'\{ uint64_t s = (.*?); FL\.cf = FL\.of = FL\.sf = FL\.af = 0; if \(s == 0\) \{ FL\.zf = 1; FL\.pf = 1; \} else \{ uint64_t k = (.*?); (.*?) FL\.zf = 0; FL\.pf = FLE_UNDEF; \} \}\}$', txt)
    assert mm, txt
    s_expr, k_expr, assign = mm.groups()
    if kind == 'Tzcnt':
        assert bk == 'Bsf' and k_expr == '(uint64_t)__builtin_ctzll(s)', k_expr
        kz = '(uint64_t)__builtin_ctzll(s)'; nt += 1
    else:
        assert bk == 'Bsr' and k_expr == '(uint64_t)(63 - __builtin_clzll(s))', k_expr
        kz = '(uint64_t)(%d - (63 - __builtin_clzll(s)))' % (W - 1); nl += 1
    new = '{ uint64_t s = %s; uint64_t k = s ? %s : %dU; %s FL.cf = (s == 0); FL.zf = (k == 0); FL.of = FL.sf = FL.af = FL.pf = FLE_UNDEF; }}' % (s_expr, kz, W, assign)
    return ('L_%x: { /* 0x%x %s */ ' % (va, va, kind)).encode() + new.encode()
res = pat.sub(sub, src)
open(gen, 'wb').write(res)
print('patch_bmi: tzcnt', nt, 'lzcnt', nl)
