#!/usr/bin/env python3
"""hyb1: drop from a unit of the generated C the bodies of the functions that the direct back end lowers (the prototypes stay: they become imports of the C part).
usage: strip_units2.py UNIT.c LOWERED.txt OUT.c   (LOWERED: 0x<va> per line). Prints `<unit> dropped N kept M`; writes OUT.c only when something was dropped. The SEH wrapper f_X / body fb_X pairs are never in the list."""
import sys, re
src, lst, out = sys.argv[1:4]
low = set(int(l.split()[0], 16) for l in open(lst) if l.strip())
lines = open(src).read().split('\n')
res = []
i = 0
dropped = kept = 0
pat = re.compile(r'^void f_([0-9a-f]+)\(void\)$')
while i < len(lines):
    m = pat.match(lines[i])
    if m and i + 1 < len(lines) and lines[i + 1] == '{':
        va = int(m.group(1), 16)
        j = i + 2
        while lines[j] != '}':
            j += 1
        if va in low:
            dropped += 1
        else:
            kept += 1
            res.extend(lines[i:j + 1])
        i = j + 1
        continue
    res.append(lines[i])
    i += 1
if dropped:
    open(out, 'w').write('\n'.join(res))
print(src.split('/')[-1], 'dropped', dropped, 'kept', kept)
