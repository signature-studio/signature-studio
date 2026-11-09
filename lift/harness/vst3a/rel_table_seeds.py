"""Decision 0165 (vst3a): seeds for the jump tables of the table-relative form that fe-lift (decisions 0089/0109) does not derive:
    lea  B,[rip+T] ; ... ; movsxd X,dword [B+I*4] ; add X,B ; jmp X          (entries are signed 32-bit offsets from T, not RVAs)
For each `jmp X` of this form the entries (T + offset) are read and written as seeds, so the lifted `jmp X` (an fle_dispatch of the target) finds a function entry (the same device as
vst2's S-vst-jt-bound for the image-base form). The bound N comes from the nearest earlier `cmp I',imm` + `ja/jae` within 60 instructions when I' names the same register family
as I (N = imm + 1, or imm for jae); otherwise the entries are read while the target lies inside the same .pdata function (at most 1024; reported as 'by neighbourhood'). A site where
no entry is plausible is reported, not guessed. usage: rel_table_seeds.py FILE OUT_seeds.txt OUT_sites.tsv"""
import struct, sys, bisect, capstone
from capstone import x86
fn, out_seeds, out_sites = sys.argv[1:4]
d = open(fn, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    ch = struct.unpack_from('<I', d, s + 36)[0]
    name = d[s:s + 8].rstrip(b'\0')
    secs.append((va, vs, rs, rp, ch, name))
def r2o(rva):
    for va, vs, rs, rp, ch, nm in secs:
        if va <= rva < va + min(vs, rs) if rs else va <= rva < va + vs: return rp + rva - va
def is_exec(rva):
    for va, vs, rs, rp, ch, nm in secs:
        if va <= rva < va + max(vs, rs): return bool(ch & 0x20000000)
    return False
pd = next(s for s in secs if s[5] == b'.pdata')
funcs = []
o = pd[3]
for k in range(pd[1] // 12):
    b, e, u = struct.unpack_from('<III', d, o + 12 * k)
    if b: funcs.append((b, e))
funcs.sort()
# leaf functions have no .pdata entry: the executable bytes between .pdata ranges are scanned too, each gap (minus int3 padding at both ends) as one function
gaps = []
for va, vs, rs, rp, ch, nm in secs:
    if not (ch & 0x20000000): continue
    cur = va; end = va + vs
    for (b, e) in [f for f in funcs if va <= f[0] < end]:
        if b > cur: gaps.append((cur, b))
        cur = max(cur, e)
    if cur < end: gaps.append((cur, end))
nfunc_pdata = len(funcs)
for (b, e) in gaps:
    o = r2o(b)
    if o is None: continue
    while b < e and d[o] == 0xcc: b += 1; o += 1
    while e > b and d[r2o(e - 1)] == 0xcc: e -= 1
    if e - b >= 2: funcs.append((b, e))
funcs.sort(); starts = [f[0] for f in funcs]
print('pdata functions', nfunc_pdata, 'gap chunks', len(funcs) - nfunc_pdata, file=sys.stderr)
def func_of(rva):
    k = bisect.bisect_right(starts, rva) - 1
    return funcs[k] if k >= 0 and funcs[k][0] <= rva < funcs[k][1] else None
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); md.detail = True
def fam(name):
    n = name.lower()
    for a, b in (('eax', 'rax'), ('ecx', 'rcx'), ('edx', 'rdx'), ('ebx', 'rbx'), ('esi', 'rsi'), ('edi', 'rdi'), ('ebp', 'rbp'), ('esp', 'rsp')):
        if n in (a, b, a[1:], 'ax' if a == 'eax' else a[1:]) : return b
    if n.startswith('r') and n[1:].rstrip('dwb').isdigit(): return 'r' + n[1:].rstrip('dwb')
    return n
seeds = set(); rows = []; nsites = 0
for (b, e) in funcs:
    o = r2o(b)
    if o is None or not is_exec(b): continue
    ins = list(md.disasm(d[o:o + (e - b)], base + b))
    for idx, i in enumerate(ins):
        if i.mnemonic != 'jmp' or len(i.operands) != 1 or i.operands[0].type != x86.X86_OP_REG: continue
        X = i.reg_name(i.operands[0].reg)
        # add X,B before
        add = None
        for k in range(idx - 1, max(idx - 8, -1), -1):
            j = ins[k]
            if j.mnemonic == 'add' and len(j.operands) == 2 and j.operands[0].type == x86.X86_OP_REG and j.operands[1].type == x86.X86_OP_REG and j.reg_name(j.operands[0].reg) == X:
                add = (k, j.reg_name(j.operands[1].reg)); break
        if not add: continue
        k_add, B = add
        mv = None
        for k in range(k_add - 1, max(k_add - 8, -1), -1):
            j = ins[k]
            if j.mnemonic in ('movsxd', 'mov') and len(j.operands) == 2 and j.operands[1].type == x86.X86_OP_MEM:
                m = j.operands[1].mem
                if m.scale == 4 and m.index and m.base and j.reg_name(m.base) == B and j.reg_name(j.operands[0].reg) in (X, X.replace('r', 'e', 1) if X.startswith('r') and len(X) == 3 else X, X + 'd'):
                    mv = (k, j); break
        if not mv: continue
        k_mv, jm = mv
        I = jm.reg_name(jm.operands[1].mem.index)
        T = None
        callee_saved = B in ('rbx', 'rbp', 'rsi', 'rdi', 'r12', 'r13', 'r14', 'r15')
        for k in range(k_mv - 1, max(k_mv - 4000, -1) if callee_saved else max(k_mv - 200, -1), -1):
            j = ins[k]
            if j.mnemonic == 'lea' and len(j.operands) == 2 and j.operands[0].type == x86.X86_OP_REG and j.reg_name(j.operands[0].reg) == B and j.operands[1].type == x86.X86_OP_MEM and j.reg_name(j.operands[1].mem.base) == 'rip':
                T = j.address + j.size + j.operands[1].mem.disp; break
            if not callee_saved and j.mnemonic in ('call', 'ret'): break
        if T is None: continue
        nsites += 1
        N = None; how = ''
        fams = {fam(I)}
        for k in range(k_mv - 1, max(k_mv - 10, -1), -1):   # the index may have been copied from the register that was compared: mov I,S
            j = ins[k]
            if j.mnemonic in ('mov', 'movzx', 'movsxd') and len(j.operands) == 2 and j.operands[0].type == x86.X86_OP_REG and j.operands[1].type == x86.X86_OP_REG and fam(j.reg_name(j.operands[0].reg)) in fams:
                fams.add(fam(j.reg_name(j.operands[1].reg)))
        for k in range(idx - 1, max(idx - 60, 0), -1):
            j = ins[k]
            if j.mnemonic in ('ja', 'jae') and k >= 1 and ins[k - 1].mnemonic == 'cmp':
                c = ins[k - 1]
                if len(c.operands) == 2 and c.operands[0].type == x86.X86_OP_REG and c.operands[1].type == x86.X86_OP_IMM and fam(c.reg_name(c.operands[0].reg)) in fams:
                    N = c.operands[1].imm + (1 if j.mnemonic == 'ja' else 0); how = 'cmp'; break
        ot = r2o(T - base)
        if ot is None: rows.append((i.address, T, 0, 'table not mapped')); continue
        ents = []
        if N is not None and 0 < N <= 4096:
            ents = [T + struct.unpack_from('<i', d, ot + 4 * n)[0] for n in range(N)]
        else:
            how = 'neighbourhood'
            for n in range(1024):
                t = T + struct.unpack_from('<i', d, ot + 4 * n)[0]
                f = func_of(t - base)
                if f is None or not (f[0] <= (i.address - base) < f[1]) or f != func_of(i.address - base): break
                ents.append(t)
        bad = [t for t in ents if not is_exec(t - base)]
        if bad or not ents: rows.append((i.address, T, len(ents), how + ': %d targets not executable' % len(bad))); continue
        seeds.update(ents)
        rows.append((i.address, T, len(ents), how))
with open(out_seeds, 'x') as f:
    for s in sorted(seeds): f.write('0x%x\n' % s)
with open(out_sites, 'w') as f:
    f.write('jmp_va\ttable_va\tentries\thow\n')
    for r in rows: f.write('0x%x\t0x%x\t%d\t%s\n' % r)
print('table-relative jump sites', nsites, 'seeds', len(seeds), 'by cmp', sum(1 for r in rows if r[3] == 'cmp'), 'by neighbourhood', sum(1 for r in rows if r[3] == 'neighbourhood'), 'rejected', sum(1 for r in rows if 'not' in r[3]))
