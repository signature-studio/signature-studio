#!/usr/bin/env python3
"""Instruction-counting copies of the units of a lifted module (decision 0156). usage: count_instrument.py [--per-insn] IN.c OUT.c   (--per-insn: every instruction is a block of its own; for checking the block counts against it)
Reads a unit written by split_bytes.py (one instruction per line, `L_<va>: { /* 0x<va> <Mnemonic> */ ... }`, functions `void f_<va>(void)` ... `}`) and writes it with
`FLE_CNT(n, s, p);` at the start of every basic block of every function (fle_count.h). Leaders: the first instruction, every label that some `goto` of the function names, and
the instruction after a line that contains a `goto`, a `return` or an indirect transfer. n counts the instructions of the block, s the scalar-SSE floating-point ones, p the other
SSE ones (a line whose body uses X[]); the rest are 'integer'. Nothing else in the file is changed (the prologue gets one #include)."""
import re, sys
per_insn = "--per-insn" in sys.argv
args = [a for a in sys.argv[1:] if not a.startswith("--")]
src, dst = args[0], args[1]
lines = open(src).read().split('\n')
fn_start = re.compile(r'^void f_[0-9a-f]+\(void\)$')
inst = re.compile(r'^(L_[0-9a-f]+): \{ /\* 0x[0-9a-f]+ ([A-Za-z0-9]+) \*/ ')
goto = re.compile(r'goto (L_[0-9a-f]+);')
SCALAR = re.compile(r'^(add|sub|mul|div|sqrt|min|max|mov|cmp|comi|ucomi|cvt|cvtt|rcp|rsqrt|round)(sd|ss)$|^(cvtsi2sd|cvtsi2ss|cvtsd2si|cvtss2si|cvttsd2si|cvttss2si|cvtsd2ss|cvtss2sd|comisd|comiss|ucomisd|ucomiss|movd|movq)$')
out = []
i = 0
n = len(lines)
# the prologue gets the include once (after the last #include of the first block)
inc_done = False
nblocks = 0; ninst = 0
while i < n:
    l = lines[i]
    if not inc_done and l.startswith('#include "fle_dispatch.h"'):
        out.append(l); out.append('#include "fle_count.h"'); inc_done = True; i += 1; continue
    if fn_start.match(l) and i + 1 < n and lines[i + 1] == '{':
        # collect the function body
        j = i + 2
        while j < n and lines[j] != '}':
            j += 1
        body = lines[i + 2:j]
        targets = set()
        for b in body:
            for t in goto.findall(b):
                targets.add(t)
        # leaders
        leader = [False] * len(body)
        prev_ends = True
        for k, b in enumerate(body):
            m = inst.match(b)
            if not m:
                continue
            if per_insn or prev_ends or m.group(1) in targets:
                leader[k] = True
            prev_ends = ('goto ' in b) or ('return' in b) or ('fle_dispatch(' in b and 'R[4] -= 8' not in b) or ('fle_jumptab_miss' in b) or ('fle_landing' in b)
        # counts per block
        new = list(body)
        k = 0
        while k < len(body):
            if leader[k]:
                e = k + 1
                while e < len(body) and not leader[e]:
                    e += 1
                cnt = s = p = 0
                for q in range(k, e):
                    m = inst.match(body[q])
                    if not m:
                        continue
                    cnt += 1
                    mn = m.group(2).lower()
                    uses_x = 'X[' in body[q] or 'fle_ldx' in body[q] or 'fle_stx' in body[q]
                    if uses_x:
                        if SCALAR.match(mn): s += 1
                        else: p += 1
                if cnt:
                    new[k] = inst.sub(lambda m: m.group(0) + 'FLE_CNT(%d, %d, %d); ' % (cnt, s, p), new[k], count=1)
                    nblocks += 1; ninst += cnt
                k = e
            else:
                k += 1
        out.append(l); out.append('{')
        out.extend(new)
        out.append('}')
        i = j + 1
        continue
    out.append(l)
    i += 1
open(dst, 'x').write('\n'.join(out))
print('blocks', nblocks, 'instructions', ninst, file=sys.stderr)
