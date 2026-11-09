#!/usr/bin/env python3
"""Split a fe-lift generated C file into N translation units at function boundaries.
usage: split_c.py IN.c OUTPREFIX FUNCS_PER_PART
Part 0 keeps the prologue as generated (globals, prototypes, dispatch and resp tables).
Parts >0 get the prologue up to the dispatch table with the globals turned into externs.
Nothing is written over an existing file (open mode 'x')."""
import re, sys
src, prefix, per = sys.argv[1], sys.argv[2], int(sys.argv[3])
lines = open(src).read().split('\n')
first_def = next(i for i, l in enumerate(lines) if re.match(r'^void f_[0-9a-f]+\(void\)$', l) and lines[i+1] == '{')
table = next(i for i, l in enumerate(lines) if l.startswith('const fle_fn_entry fle_dispatch_table[]'))
pro0 = lines[:first_def]
proN = lines[:table]
proN = [re.sub(r'^uint64_t R\[16\];', 'extern uint64_t R[16];', l) for l in proN]
proN = [re.sub(r'^fle_eflags FL = .*;$', 'extern fle_eflags FL;', l) for l in proN]
proN = [re.sub(r'^fle_xmm X\[16\];', 'extern fle_xmm X[16];', l) for l in proN]
body = lines[first_def:]
starts = [i for i, l in enumerate(body) if re.match(r'^void f_[0-9a-f]+\(void\)$', l) and body[i+1] == '{']
parts = []
for k in range(0, len(starts), per):
    a = starts[k]
    b = starts[k+per] if k+per < len(starts) else len(body)
    parts.append(body[a:b])
for n, p in enumerate(parts):
    name = '%s-%02d.c' % (prefix, n)
    with open(name, 'x') as f:
        f.write('\n'.join(pro0 if n == 0 else proN) + '\n')
        f.write('\n'.join(p) + '\n')
print('functions', len(starts), 'parts', len(parts))
