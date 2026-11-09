"""Decision 0165 (vst3a): the lifted `jmp [IAT]` thunks and `call [IAT]` sites of ORDINAL imports (fe-lift names them dll!#N; --extra-imports cannot carry such a row because its parser reads '#' as a
comment) are emitted as `fle_unsupported(va)`. This rewrites exactly those lines of the generated C into the form that a row would have produced:
  thunk  : { void F(void); F(); return; }                                         (the form of f_180a1b140 for a named import)
  call   : { void F(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C(next)); F(); R[4] += 8; }   (decision 0077's form)
F is the function named in the slots file written by mk_stubs.py. Nothing else is touched. usage: patch_ordinals.py FILE gen.c slots-file"""
import re, struct, sys, capstone
from capstone import x86
fn, gen, slots = sys.argv[1:4]
d = open(fn, 'rb').read()
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
amap = {}
for l in open(slots):
    a, f, dll = l.split()
    amap[int(a, 16)] = f
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); md.detail = True
src = open(gen, 'rb').read()
pat = re.compile(rb'^(L_([0-9a-f]+): \{ /\* 0x[0-9a-f]+ (Jmp|Call) \*/ )fle_unsupported\(UINT64_C\(0x([0-9a-f]+)\)\); \}$', re.M)
cnt = {'Jmp': 0, 'Call': 0}; other = 0
def sub(m):
    global other
    va = int(m.group(2), 16)
    o = r2o(va - base)
    ins = next(md.disasm(d[o:o + 16], va))
    mem = ins.operands[0].mem if ins.operands and ins.operands[0].type == x86.X86_OP_MEM else None
    if mem is None or ins.reg_name(mem.base) != 'rip':
        other += 1; return m.group(0)
    slot = ins.address + ins.size + mem.disp
    f = amap.get(slot)
    if f is None:
        other += 1; return m.group(0)
    kind = m.group(3).decode()
    cnt[kind] += 1
    fb = f.encode()
    if kind == 'Jmp':
        return m.group(1) + b'{ void ' + fb + b'(void); ' + fb + b'(); return; }}'
    nxt = ins.address + ins.size
    return m.group(1) + b'{ void ' + fb + b'(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C(0x%x)); ' % nxt + fb + b'(); R[4] += 8; }}'
out = pat.sub(sub, src)
open(gen, 'wb').write(out)
print('patch_ordinals: jmp thunks', cnt['Jmp'], 'call sites', cnt['Call'], 'other unsupported Jmp/Call lines left', other)
