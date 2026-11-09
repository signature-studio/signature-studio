#!/usr/bin/env python3
"""parity1: the statistics table of the decision document: per wasm configuration, the ratio W/N1 over all operations (not prelude/calibration): n, median, max (op), number above 1.2 and above 2.0, number at or below 1.0, the ratio of the summed CPU of the continuous operations, and the same against N1fc. usage: mk-stats.py MASTER.tsv LABEL..."""
import csv, sys, statistics
rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t')); labs = sys.argv[2:]
def f(x):
    try: return float(x)
    except Exception: return None
def stat(w, n):
    R = [(f(r[w + '_cpu_ms']) / f(r[n + '_cpu_ms']), r) for r in rows if r['category'] not in ('prelude', 'calibration') and f(r.get(w + '_cpu_ms')) is not None and f(r.get(n + '_cpu_ms'))]
    if not R: return None
    C = [r for _, r in R if r['kind'] == 'C']
    cs = sum(f(r[w + '_cpu_ms']) for r in C) / sum(f(r[n + '_cpu_ms']) for r in C)
    mx = max(R, key=lambda t: t[0])
    return len(R), statistics.median(x for x, _ in R), mx[0], mx[1]['session'] + ' ' + mx[1]['op'], sum(1 for x, _ in R if x > 1.2), sum(1 for x, _ in R if x > 2.0), sum(1 for x, _ in R if x <= 1.0), cs
print('| W | 比の基準 | 操作数 | 中央 | 最大(操作) | 1.2 倍超 | 2 倍超 | 1.0 以下 | 連続操作の CPU 合計比 |\n|---|---|---|---|---|---|---|---|---|')
for n in ('N1', 'N1fc'):
    for w in labs:
        if w == n:
            continue
        s = stat(w, n)
        if s:
            print('| %s | %s | %d | %.2f | %.2f (%s) | %d | %d | %d | %.2f |' % (w, n, s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]))
