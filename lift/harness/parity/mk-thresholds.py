#!/usr/bin/env python3
"""parity1: a thresholds file for check.py from a master table (mk-master.py) of the accepted configuration: usage: mk-thresholds.py MASTER.tsv W_LABEL N_LABEL OUT.tsv [MARGIN=2.0]
max_ratio = max(2.5, margin * measured ratio); max_cpu_per_vs (continuous ops) = margin * measured + 0.05; max_cpu_ms (discrete ops) = margin * measured + 50 ms. These are REGRESSION GUARDS (the accepted state + a margin), not the targets."""
import csv, sys
rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t')); w, n, out = sys.argv[2], sys.argv[3], sys.argv[4]; m = float(sys.argv[5]) if len(sys.argv) > 5 else 2.0
def f(x):
    try: return float(x)
    except Exception: return None
L = ['# parity gate thresholds (decision 0226): regression guards = accepted state x %.2f (+ a small absolute term; the run-to-run spread of node CPU under load 10-25 is 0.6-1.8 x, decision 0226 section 8); targets (ratio 1.2, 1.0 CPU s per virtual s) are WARN lines of check.py' % m, '# session\top\tmax_ratio\tmax_cpu_per_vs\tmax_cpu_ms\tnote']
L.append('*\t*\t2.5\t2.0\t\tdefault guard for an op that has no row')
for r in rows:
    if r['category'] in ('prelude', 'calibration'): continue
    a, b, v = f(r[w + '_cpu_ms']), f(r[n + '_cpu_ms']), f(r['virtual_ms'])
    if a is None: continue
    ratio = max(2.5, m * a / b) if b else ''
    pv = (m * a / v + 0.05) if (v and r['kind'] == 'C') else ''
    ms = (m * a + 50) if r['kind'] == 'D' else ''
    L.append('%s\t%s\t%s\t%s\t%s\t%s' % (r['session'], r['op'], ('%.2f' % ratio) if ratio != '' else '', ('%.2f' % pv) if pv != '' else '', ('%.0f' % ms) if ms != '' else '', 'measured %s %.0f ms vs %s %.0f ms' % (w, a, n, b or 0)))
open(out, 'w').write('\n'.join(L) + '\n'); print(len(L), 'lines ->', out)
