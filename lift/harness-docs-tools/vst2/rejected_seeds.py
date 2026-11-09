"""Seeds for the MSVC jump tables that fe-lift rejected (`msvc_rejected 0x<jmp VA>: ...` lines of its report): decision 0156, tools only (fe-lift is not changed).
For each rejected `jmp reg` the script looks back for the idiom  mov r32,[base + idx*4 + TABLE] ; add r64,base ; jmp r64  (base = ImageBase, as the lifter's own
proof requires), takes the bound N from the nearest preceding `cmp idx, imm` (N = imm + 1), reads N 32-bit RVAs at TABLE and writes ImageBase + RVA as seeds.
A site whose idiom or bound is not found is reported (not guessed). usage: rejected_seeds.py FILE fe-lift-report.txt OUT_seeds.txt"""
import re, struct, sys, capstone
from capstone import x86
fn, rep, out = sys.argv[1:4]
d = open(fn, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    ch = struct.unpack_from('<I', d, s + 36)[0]
    secs.append((va, vs, rs, rp, ch))
def r2o(rva):
    for va, vs, rs, rp, ch in secs:
        if va <= rva < va + max(vs, rs): return rp + rva - va
def is_exec(rva):
    for va, vs, rs, rp, ch in secs:
        if va <= rva < va + max(vs, rs): return bool(ch & 0x20000000)
    return False
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); md.detail = True
sites = [int(m.group(1), 16) for m in re.finditer(r'^msvc_rejected\s+(0x[0-9a-f]+):', open(rep).read(), re.M)]
seeds = set(); notes = []
for J in sites:
    best = None
    for back in range(8, 120):
        start = J - back
        o = r2o(start - base)
        if o is None: continue
        ins = list(md.disasm(d[o:o + back + 16], start))
        addrs = [i.address for i in ins]
        if J not in addrs: continue
        seq = ins[:addrs.index(J) + 1]
        if seq[-1].mnemonic != 'jmp' or len(seq[-1].operands) != 1 or seq[-1].operands[0].type != x86.X86_OP_REG: continue
        # the nearest mov r32,[b + i*4 + disp] before the jmp
        for k in range(len(seq) - 2, -1, -1):
            i = seq[k]
            if i.mnemonic == 'mov' and len(i.operands) == 2 and i.operands[1].type == x86.X86_OP_MEM and i.operands[1].mem.scale == 4 and i.operands[1].mem.index and i.operands[1].mem.base:
                best = (seq, k, i); break
        if best: break
    if not best:
        notes.append('%#x: idiom not found' % J); continue
    seq, k, mv = best
    tbl = mv.operands[1].mem.disp
    idx = mv.operands[1].mem.index
    # the bound: nearest earlier `cmp idx, imm` in the bytes before the mov (up to 200 bytes back; the instructions are re-decoded from the sequence start)
    N = None
    start = seq[0].address
    for back in range(0, 260):
        s0 = start - back
        o = r2o(s0 - base)
        if o is None: continue
        ins = list(md.disasm(d[o:o + back + 4], s0))
        if not ins or ins[-1].address + ins[-1].size != start and (len(ins) == 0 or start not in [x.address for x in ins]): continue
        ins = [x for x in ins if x.address < start]
        for x in reversed(ins):
            if x.mnemonic == 'cmp' and len(x.operands) == 2 and x.operands[1].type == x86.X86_OP_IMM and x.operands[0].type == x86.X86_OP_REG:
                r = x.reg_name(x.operands[0].reg)
                if r == md.reg_name(idx) or r[1:] == md.reg_name(idx)[1:] or r.rstrip('dwl') == md.reg_name(idx).rstrip('dwl'):
                    N = x.operands[1].imm + 1; break
        if N: break
    o = r2o(tbl)
    how = 'N from cmp'
    if not N or N > 4096:
        # no bound found: read consecutive RVAs while they are executable and within 0x1800 bytes of the jump (S-vst-jt-bound: an over-read adds only entries of
        # neighbouring tables of the same routine, which are code addresses; the lift report's bad_nodes and the run tell if one is not)
        N = 0
        while N < 64:
            v = struct.unpack_from('<I', d, o + 4 * N)[0]
            if not is_exec(v) or abs(base + v - J) > 0x1800: break
            N += 1
        how = 'N by neighbourhood (no cmp found)'
        if N == 0:
            notes.append('%#x: table %#x, no entry' % (J, tbl)); continue
    tg = [base + struct.unpack_from('<I', d, o + 4 * n)[0] for n in range(N)]
    bad = [t for t in tg if not is_exec(t - base)]
    if bad:
        notes.append('%#x: table %#x N=%d has %d targets outside executable sections: not used' % (J, tbl, N, len(bad))); continue
    seeds.update(tg)
    notes.append('%#x: table RVA %#x N=%d (%s) -> %d distinct targets' % (J, tbl, N, how, len(set(tg))))
with open(out, 'x') as f:
    for s in sorted(seeds): f.write('0x%x\n' % s)
print('\n'.join(notes)); print('rejected sites', len(sites), 'seeds', len(seeds))
