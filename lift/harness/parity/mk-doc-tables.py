#!/usr/bin/env python3
"""parity1: markdown tables for the decision document from a master table (mk-master.py) and Chrome results. usage: mk-doc-tables.py MASTER.tsv OUTDIR [LABEL=chrome-result.json ...]"""
import csv, json, sys, os
rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t')); out = sys.argv[2]; os.makedirs(out, exist_ok=True)
labels = [c[:-len('_cpu_ms')] for c in rows[0] if c.endswith('_cpu_ms')]
def f(x):
    try: return float(x)
    except Exception: return None
def fmt(x, d=0):
    return '-' if x is None else ('%.*f' % (d, x))
chrome = [(a.split('=')[0], json.load(open(a.split('=')[1]))) for a in sys.argv[3:]]
# Table A: all operations
cols = os.environ['DOC_COLS'].split(',') if os.environ.get('DOC_COLS') else [l for l in labels if l not in ('Wship',)]
hdr = ['セッション', '操作', '種別', '仮想 ms', 'N1 ms', 'N1 /vs', 'N2'] + sum([[l + ' ms', l + '/N1'] for l in cols if l != 'N1'], [])
L = ['| ' + ' | '.join(hdr) + ' |', '|' + '---|' * len(hdr)]
for r in rows:
    if r['category'] == 'calibration': continue
    n1 = f(r['N1_cpu_ms']); v = f(r['virtual_ms'])
    cells = [r['session'], r['op'] + (' (準備)' if r['category'] == 'prelude' else ''), r['kind'], r['virtual_ms'], fmt(n1), (fmt(n1 / v, 2) if (n1 is not None and v) else '-'), '測れなかった']
    for l in cols:
        if l == 'N1': continue
        a = f(r[l + '_cpu_ms']); cells += [fmt(a), (fmt(a / n1, 2) if (a is not None and n1) else '-')]
    L.append('| ' + ' | '.join(cells) + ' |')
open(os.path.join(out, 'table-A-node-all-ops.md'), 'w').write('\n'.join(L) + '\n')
# Table B: ranking by the gap V0 - N1 (ms) and by CPU seconds per virtual second
def rank(w, n, path, title):
    R = []
    for r in rows:
        if r['category'] in ('prelude', 'calibration'): continue
        a, b, v = f(r[w + '_cpu_ms']), f(r[n + '_cpu_ms']), f(r['virtual_ms'])
        if a is None or b is None or not b: continue
        R.append((a - b, a / b, a, b, v, r))
    R.sort(key=lambda t: -t[0])
    L = ['**' + title + '**', '', '| 順 | セッション | 操作 | 種別 | 仮想 ms | %s ms | %s ms | 差 ms | 比 | %s CPU秒/仮想秒 |' % (n, w, w), '|---|---|---|---|---|---|---|---|---|---|']
    for i, (d, ratio, a, b, v, r) in enumerate(R[:25]):
        L.append('| %d | %s | %s | %s | %s | %.0f | %.0f | %.0f | %.2f | %.2f |' % (i + 1, r['session'], r['op'], r['kind'], r['virtual_ms'], b, a, d, ratio, a / v if v else 0))
    open(path, 'w').write('\n'.join(L) + '\n')
for w in [l for l in labels if l not in ('N1', 'N1fc', 'Wship') and (not os.environ.get('RANK_LABELS') or l in os.environ['RANK_LABELS'].split(','))]:
    rank(w, 'N1', os.path.join(out, 'rank-%s-vs-N1.md' % w), '%s と N1 の差(wasm − native)の大きい順(上位 25)' % w)
if chrome:
    ops = [o['op'] for o in chrome[0][1]['ops']]
    H = ['操作'] + sum([[l + ' Worker CPU 秒', l + ' 主スレッド CPU 秒', l + ' 画面更新/s'] for l, _ in chrome], []) + ['wall 秒 (' + chrome[0][0] + ')', 'load']
    L = ['| ' + ' | '.join(H) + ' |', '|' + '---|' * len(H)]
    for op in ops:
        c = [op]
        for l, r in chrome:
            d = next((o for o in r['ops'] if o['op'] == op), None)
            c += [('%.2f' % d['workerS'], '%.2f' % d['mainS'], '%.1f' % d['changedPerS']) if d else ('-', '-', '-')][0]
        d0 = next((o for o in chrome[0][1]['ops'] if o['op'] == op), None)
        c += [('%.1f' % d0['wallS']) if d0 else '-', str(d0['load']) if d0 else '-']
        L.append('| ' + ' | '.join(c) + ' |')
    open(os.path.join(out, 'table-C-chrome.md'), 'w').write('\n'.join(L) + '\n')
print('tables in', out)
