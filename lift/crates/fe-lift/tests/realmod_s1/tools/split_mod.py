"""design aid (option S, shared machine state): split a fe-lift C file for ONE module into translation units.
usage: split_mod.py IN.c OUTPREFIX FUNCS_PER_PART
 - every part gets the prologue up to the dispatch table with R/FL/X turned into externs (like split_c.py parts > 0),
   so no part defines R, FL, X;
 - the dispatch table + resp table section (between the table start and the first function) is NOT put in any part;
   it is written to OUTPREFIX-tables.txt for merge_tables.py.
Nothing is written over an existing file (mode 'x')."""
import re, sys
src, prefix, per = sys.argv[1], sys.argv[2], int(sys.argv[3])
lines = open(src).read().split('\n')
fd = re.compile(r'^void f_[0-9a-f]+\(void\)$')
first_def = next(i for i, l in enumerate(lines) if fd.match(l) and lines[i + 1] == '{')
table = next(i for i, l in enumerate(lines) if l.startswith('const fle_fn_entry fle_dispatch_table[]'))
assert table < first_def
pro = lines[:table]
pro = [re.sub(r'^uint64_t R\[16\];', 'extern uint64_t R[16];', l) for l in pro]
pro = [re.sub(r'^fle_eflags FL = .*;$', 'extern fle_eflags FL;', l) for l in pro]
pro = [re.sub(r'^fle_xmm X\[16\];', 'extern fle_xmm X[16];', l) for l in pro]
body = lines[first_def:]
starts = [i for i, l in enumerate(body) if fd.match(l) and body[i + 1] == '{']
with open(prefix + '-tables.txt', 'x') as f:
    f.write('\n'.join(lines[table:first_def]) + '\n')
n = 0
for k in range(0, len(starts), per):
    a = starts[k]
    b = starts[k + per] if k + per < len(starts) else len(body)
    with open('%s-%02d.c' % (prefix, n), 'x') as f:
        f.write('\n'.join(pro) + '\n')
        f.write('\n'.join(body[a:b]) + '\n')
    n += 1
print('functions', len(starts), 'parts', n)
