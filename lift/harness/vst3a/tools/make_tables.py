#!/usr/bin/env python3
"""Decision 0177 (srm1): the block-time table from the summaries of parse_ab.py (any number of AB summary TSVs are merged: for the same (build, case) the runs are pooled by taking the min of
the minima and the median of the medians over all rows of the RAW TSVs given with --raw).
usage: make_tables.py OUT.tsv RAW1.tsv [RAW2.tsv ...]
Output columns: case, build, n_runs, min_ms, median_of_min_ms, median_of_median_ms, load_range, fraction_of_period (min_ms / 92.88), ratio_to_native (min), ns_per_source_instruction (min_ms / instructions of
the block as counted by decision 0165's COUNT build), hashes."""
import sys, collections, statistics as st
P = 4096 / 44100 * 1000
ICOUNT = {'0': 30096486, '1': 27866096, '2': 110447250, '3': 26139780, '4': 23370754, '5': 51333255}   # decision 0165 tables/icount.tsv (state N, one held note, 1 block); chord: not counted
rows = []
for fn in sys.argv[2:]:
    for i, l in enumerate(open(fn)):
        f = l.rstrip('\n').split('\t')
        if i == 0 or len(f) < 10 or not f[7]: continue
        rows.append(dict(build=f[0], case=f[1], lb=float(f[3]), la=float(f[4]), med=float(f[6]), mn=float(f[7]), hash=f[9]))
g = collections.defaultdict(list)
for r in rows: g[(r['case'], r['build'])].append(r)
nat = {c: min(r['mn'] for r in rs) for (c, b), rs in g.items() if b.startswith('native')}
out = ['case\tbuild\tn_runs\tmin_ms\tmedian_of_min_ms\tmedian_of_median_ms\tload_min\tload_max\tfraction_of_period\tratio_to_native\tns_per_source_instr\thashes']
def key(k):
    c, b = k
    return ({'0': 0, '1': 1, '2': 2, '3': 3, '4': 4, '5': 5, 'chord': 6, 'idle': 7}.get(c, 9), b)
for (c, b) in sorted(g, key=key):
    rs = g[(c, b)]
    mn = min(r['mn'] for r in rs)
    loads = [x for r in rs for x in (r['lb'], r['la'])]
    ic = ICOUNT.get(c)
    out.append('\t'.join([c, b, str(len(rs)), f'{mn:.1f}', f"{st.median(r['mn'] for r in rs):.1f}", f"{st.median(r['med'] for r in rs):.1f}", f'{min(loads):.1f}', f'{max(loads):.1f}',
                          f'{mn / P:.3f}', f'{mn / nat[c]:.2f}' if c in nat else '', f'{mn * 1e6 / ic:.2f}' if ic else '', ','.join(sorted({r['hash'] for r in rs}))]))
open(sys.argv[1], 'w').write('\n'.join(out) + '\n')
print('\n'.join(out))
