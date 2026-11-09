#!/usr/bin/env python3
"""alpha1: prints the `series` of a sdrive-a1.mjs report.json as a table (time ms, colour of every point) and the verdict: the first point's colour moves from the menu colour to the colour below without passing black.
usage: series-report.py REPORT.json SERIES_FADE SERIES_AFTER [OUT.tsv]"""
import json, sys
rep = json.load(open(sys.argv[1]))
fade, after = sys.argv[2], sys.argv[3]
for sc in rep:
    s = sc.get('series') or {}
    if fade not in s: continue
    rows = s[fade]['rows']; pts = s[fade]['points']
    under = s[after]['rows'][-1]['px'][0] if after in s else None
    out = ['t_ms\t' + '\t'.join('p%d_%d_%d' % (i, x, y) for i, (x, y) in enumerate(pts))]
    for r in rows: out.append('%d\t%s' % (r['t'], '\t'.join(','.join(map(str, c)) for c in r['px'])))
    print('\n'.join(out))
    first = rows[0]['px'][0]
    seq = [r['px'][0] for r in rows]
    lo = [min(f, u) - 2 for f, u in zip(first, under)] if under else None
    hi = [max(f, u) + 2 for f, u in zip(first, under)] if under else None
    black = sum(1 for c in seq if c == [0, 0, 0] and under != [0, 0, 0])
    outside = sum(1 for c in seq if under and any(not (lo[k] <= c[k] <= hi[k]) for k in range(3)))
    print('# under (after the window is gone):', under, ' first:', first, ' samples:', len(seq), ' black samples:', black, ' outside [first, under] +-2:', outside)
    if len(sys.argv) > 4: open(sys.argv[4], 'w').write('\n'.join(out) + '\n')
