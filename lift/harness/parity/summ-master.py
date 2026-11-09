#!/usr/bin/env python3
"""parity1: ratios and lists from a master table (mk-master.py). usage: summ-master.py MASTER.tsv NUM_LABEL DEN_LABEL [--md OUT]  (ratio = NUM cpu / DEN cpu per op; rows with both values, kind D/C, not prelude/calibration)"""
import sys, csv
rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t'))
num, den = sys.argv[2], sys.argv[3]
def f(x):
    try: return float(x)
    except Exception: return None
out = []
for r in rows:
    if r['category'] in ('prelude', 'calibration'): continue
    a, b = f(r[num + '_cpu_ms']), f(r[den + '_cpu_ms'])
    if a is None or b is None or b <= 0: continue
    out.append((a / b, r, a, b))
out.sort(key=lambda t: -t[0])
print('%s / %s: n=%d  max %.2f (%s %s)  min %.2f (%s %s)  median %.2f' % (num, den, len(out), out[0][0], out[0][1]['session'], out[0][1]['op'], out[-1][0], out[-1][1]['session'], out[-1][1]['op'], sorted(t[0] for t in out)[len(out) // 2]))
# CPU-weighted: sum of CPU of the continuous ops
cs = [t for t in out if t[1]['kind'] == 'C']
print('continuous ops: sum %s %.1f s, sum %s %.1f s, ratio %.2f' % (num, sum(t[2] for t in cs) / 1000, den, sum(t[3] for t in cs) / 1000, sum(t[2] for t in cs) / sum(t[3] for t in cs)))
print('%-8s %-22s %5s %9s %9s %6s' % ('session', 'op', 'kind', num + '_ms', den + '_ms', 'ratio'))
for ratio, r, a, b in out[:int(sys.argv[4]) if len(sys.argv) > 4 else 12]:
    print('%-8s %-22s %5s %9.0f %9.0f %6.2f' % (r['session'], r['op'], r['kind'], a, b, ratio))
