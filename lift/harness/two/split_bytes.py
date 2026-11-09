"""Measurement aid (decision 0130, PROVISIONAL): split a fe-lift C file of ONE module into translation units by a byte budget instead of a function count.
usage: split_bytes.py IN.c OUTPREFIX MAX_BYTES
The same prologue rule as crates/fe-lift/tests/realmod_s1/tools/split_mod.py (R/FL/X turned into externs, the dispatch/resp table section is not put in any part), the same function text
(copied verbatim, in the same order); only the places where the parts end differ: a part ends before the function that would take it over MAX_BYTES (a single function larger than MAX_BYTES
is a part of its own). Why: split_mod.py's 800 functions per part make m1-00 of the dsp module 88 MB of C, which needs 8.8 GB of RSS in the -O0 compile (decision 0128); the work of decision
0130 may not go over 7 GB in total. The parts are named OUTPREFIX-NN.c (two digits, as build_nmod_wasm.sh globs them). Nothing is written over an existing file (mode 'x').
It writes nothing else (no tables file: the wasm build takes the merged dispatch table from common.c)."""
import re
import sys

src, prefix, budget = sys.argv[1], sys.argv[2], int(sys.argv[3])
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
starts.append(len(body))
sizes = []
for k in range(len(starts) - 1):
    sizes.append(sum(len(l) + 1 for l in body[starts[k]:starts[k + 1]]))
parts = []
cur_a, cur_sz = 0, 0
for k, sz in enumerate(sizes):
    if k > cur_a and cur_sz + sz > budget:
        parts.append((cur_a, k))
        cur_a, cur_sz = k, 0
    cur_sz += sz
parts.append((cur_a, len(sizes)))
assert len(parts) < 100, len(parts)
tot = 0
for n, (a, b) in enumerate(parts):
    with open('%s-%02d.c' % (prefix, n), 'x') as f:
        f.write('\n'.join(pro) + '\n')
        f.write('\n'.join(body[starts[a]:starts[b]]) + '\n')
    tot += b - a
assert tot == len(sizes)
print('functions', len(sizes), 'parts', len(parts), 'largest part', max(sum(sizes[a:b]) for a, b in parts), 'bytes')
