#!/usr/bin/env python3
"""b2d2: compare two g3-lines files (g3-snap, g3-surface, g3-wintable, g3-progress lines; g3-layered is ignored because the wasm harness does not print it, as in b1; the 'entries N' field of g3-snap is the native build's hook counter and is dropped).
usage: g3cmp.py REF TEST [LABEL]   -> prints IDENTICAL or the count of differing lines per kind and the first differing lines."""
import re, sys, collections
def norm(path):
    out = []
    for l in open(path, errors='replace'):
        if re.match(r'g3-(snap|surface|wintable|progress):', l):
            out.append(re.sub(r' entries \d+', '', l.rstrip('\n')))
    return out
a, b = norm(sys.argv[1]), norm(sys.argv[2])
label = sys.argv[3] if len(sys.argv) > 3 else sys.argv[2]
kinds = lambda xs: collections.Counter(x.split(':')[0] for x in xs)
if a == b:
    print(f'{label} vs {sys.argv[1].split("/")[-1]}: IDENTICAL ({len(a)} lines: {dict(kinds(a))})')
else:
    n = max(len(a), len(b)); diff = collections.Counter(); first = []
    for i in range(n):
        x = a[i] if i < len(a) else None; y = b[i] if i < len(b) else None
        if x != y:
            diff[(x or y).split(':')[0]] += 1
            if len(first) < 3: first.append((x, y))
    print(f'{label} vs {sys.argv[1].split("/")[-1]}: DIFFERENT ({len(a)} vs {len(b)} lines; differing by kind {dict(diff)}, total {sum(diff.values())})')
    for x, y in first: print('  ref :', (x or '-')[:170]); print('  test:', (y or '-')[:170])
