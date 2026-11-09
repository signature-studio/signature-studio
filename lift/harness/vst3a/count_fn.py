#!/usr/bin/env python3
"""Per-function instruction-counting copies of the units of a lifted module (decision 0165). usage: count_fn.py OUTDIR UNIT1.c UNIT2.c ...  (the units in generation order)
Like vst/count_instrument.py (same leaders, same basic-block rule) but every function gets a number (fid, in the order of the units) and every block calls
FLE_CNTF(fid, n, s, p, x, v) (fle_countf.h): n instructions, s scalar SSE floating point, p other SSE, x x87 (mnemonic f*), v VEX (mnemonic v*). The function entry gets FLE_CNTCALL(fid).
Writes OUTDIR/<unit>.c, OUTDIR/fnlist.tsv (fid, VA, unit) and OUTDIR/countf_arrays.c (the counter arrays)."""
import re, sys, os
outdir = sys.argv[1]; units = sys.argv[2:]
os.makedirs(outdir, exist_ok=True)
fn_start = re.compile(r'^void f_([0-9a-f]+)\(void\)$')
inst = re.compile(r'^(L_[0-9a-f]+): \{ /\* 0x[0-9a-f]+ ([A-Za-z0-9]+) \*/ ')
goto = re.compile(r'goto (L_[0-9a-f]+);')
SCALAR = re.compile(r'^(add|sub|mul|div|sqrt|min|max|mov|cmp|comi|ucomi|cvt|cvtt|rcp|rsqrt|round)(sd|ss)$|^(cvtsi2sd|cvtsi2ss|cvtsd2si|cvtss2si|cvttsd2si|cvttss2si|cvtsd2ss|cvtss2sd|comisd|comiss|ucomisd|ucomiss|movd|movq)$')
NOX87 = {'fxsave', 'fxrstor', 'fxsave64', 'fxrstor64', 'femms'}
fid = 0
fnl = open(os.path.join(outdir, 'fnlist.tsv'), 'w')
fnl.write('fid\tva\tunit\n')
for src in units:
    lines = open(src).read().split('\n')
    out = []
    i = 0; n = len(lines); inc_done = False
    while i < n:
        l = lines[i]
        if not inc_done and l.startswith('#include "fle_dispatch.h"'):
            out.append(l); out.append('#include "fle_countf.h"'); inc_done = True; i += 1; continue
        m0 = fn_start.match(l)
        if m0 and i + 1 < n and lines[i + 1] == '{':
            j = i + 2
            while j < n and lines[j] != '}': j += 1
            body = lines[i + 2:j]
            my = fid; fid += 1
            fnl.write('%d\t0x%s\t%s\n' % (my, m0.group(1), os.path.basename(src)))
            targets = set()
            for b in body:
                for t in goto.findall(b): targets.add(t)
            leader = [False] * len(body)
            prev_ends = True
            for k, b in enumerate(body):
                m = inst.match(b)
                if not m: continue
                if prev_ends or m.group(1) in targets: leader[k] = True
                prev_ends = ('goto ' in b) or ('return' in b) or ('fle_dispatch(' in b and 'R[4] -= 8' not in b) or ('fle_jumptab_miss' in b) or ('fle_landing' in b)
            new = list(body)
            k = 0; first = True
            while k < len(body):
                if leader[k]:
                    e = k + 1
                    while e < len(body) and not leader[e]: e += 1
                    cnt = s = p = x = v = 0
                    for q in range(k, e):
                        m = inst.match(body[q])
                        if not m: continue
                        cnt += 1
                        mn = m.group(2).lower()
                        if mn.startswith('v') and mn not in ('verr', 'verw', 'vmcall'): v += 1; continue
                        if mn.startswith('f') and mn not in NOX87: x += 1; continue
                        if 'X[' in body[q] or 'fle_ldx' in body[q] or 'fle_stx' in body[q]:
                            if SCALAR.match(mn): s += 1
                            else: p += 1
                    if cnt:
                        pre = 'FLE_CNTF(%d, %d, %d, %d, %d, %d); ' % (my, cnt, s, p, x, v)
                        if first: pre = 'FLE_CNTCALL(%d); ' % my + pre; first = False
                        new[k] = inst.sub(lambda mm: mm.group(0) + pre, new[k], count=1)
                    k = e
                else:
                    k += 1
            out.append(l); out.append('{'); out.extend(new); out.append('}')
            i = j + 1
            continue
        out.append(l)
        i += 1
    open(os.path.join(outdir, os.path.basename(src)), 'w').write('\n'.join(out))
fnl.close()
with open(os.path.join(outdir, 'countf_arrays.c'), 'w') as f:
    f.write('#include <stdint.h>\nconst uint32_t fle_fn_nf = %d;\n' % fid)
    for nm in ('n', 's', 'p', 'x', 'v', 'calls'):
        f.write('uint64_t fle_fn_%s[%d];\n' % (nm, fid))
print('functions', fid, file=sys.stderr)
