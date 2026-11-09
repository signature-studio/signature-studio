#!/usr/bin/env python3
"""pw1 (decision 0186): rows.jsonl -> the Markdown tables of the decision document. usage: mdtable.py rows.jsonl [...]  (stdout)
One line per repetition: late counts, worst lateness, cores, load and the foreign load during the window, so that a run that was starved by other processes can be told apart."""
import json, sys

def label(r):
    s = r['serum'] or '-'
    return s + (' + OTT x%d' % r['ott'] if r['ott'] else '') + ('' if r['warm'] else ' (冷間)') + (' [暖機 %d s]' % (r['warmms'] // 1000) if r.get('warmms') else (' [暖機 150 ブロック]' if r['warm'] else ''))

agg = '--agg' in sys.argv
rows = []
import os
for f in [a for a in sys.argv[1:] if not a.startswith('--') and os.path.exists(a)]:
    for l in open(f):
        r = json.loads(l)
        if r.get('mode') == 'ring': continue
        if 'serum' in r: rows.append(r)
if agg:
    import collections
    g = collections.OrderedDict()
    for r in rows:
        g.setdefault((label(r), r.get('cpus') or ('0-%d' % (r['hc'] - 1) if r['hc'] < 16 else '全'), r['hc']), []).append(r)
    print('| 構成(Serum の状態 / OTT) | CPU(HC) | 回 | 遅れ A1 / 650(各回) | うち自身で周期超え(各回) | 遅れ A2(各回) | 最大の遅れ ms(各回) | 必要コア 全体(平均 [最小〜最大]) | うち Worker | 他プロセスの CPU 平均(各回) | load(各回の前→後) |')
    print('|---|---|---:|---|---|---|---|---|---:|---|---|')
    for (lab, cpus, hc), rs in g.items():
        cs = [r['coresTotal'] for r in rs]
        print('| %s | %s(%d) | %d | %s | %s | %s | %s | %.2f [%.2f〜%.2f] | %.2f | %s | %s |' % (lab, cpus, hc, len(rs), ' / '.join(str(r['lateA1']) for r in rs), ' / '.join(str(r.get('ownOverrun', '?')) for r in rs), ' / '.join(str(r['lateA2']) for r in rs), ' / '.join('%.0f' % r['lateMaxA1'] for r in rs),
              sum(cs) / len(cs), min(cs), max(cs), sum(r['coresWorkers'] for r in rs) / len(rs), ' / '.join(('%.1f' % r['foreignMean']) if 'foreignMean' in r else ('%.1f*' % (r['coresMachineBusyOnMask'] - r['coresTotal'])) for r in rs),
              ' / '.join('%s→%s' % (r['load_in'].split()[0], r['load_out'].split()[0]) for r in rs)))
    sys.exit(0)
print('| 構成(Serum の状態 / OTT) | CPU | 回 | load 前→後 | 遅れ A1 | A1 先頭5 | 遅れ A2 | 最大の遅れ ms | 塊 | 必要コア(全体/Worker) | 他プロセスの CPU(平均/p90/最大) | Worker の run-queue 待ち 最大 s | Serum process p50 / 最大 ms |')
print('|---|---|---:|---|---:|---:|---:|---:|---:|---|---|---:|---|')
for r in rows:
    sp = [p for p in r['per'] if p.get('kind') == 'serum']
    fo = '%.1f/%.1f/%.1f' % (r['foreignMean'], r['foreignP90'], r['foreignMax']) if 'foreignMean' in r else '%.1f(窓全体)' % (r['coresMachineBusyOnMask'] - r['coresTotal'])
    print('| %s | %s | %s | %s→%s | %d | %d | %d | %.0f | %d | %.2f / %.2f | %s | %.1f | %s |' % (
        label(r), r.get('cpus') or ('0-%d' % (r['hc'] - 1) if r['hc'] < 16 else '全'), r['name'].rsplit('-r', 1)[1], r['load_in'].split()[0], r['load_out'].split()[0], r['lateA1'], r['lateA1_first5'], r['lateA2'], r['lateMaxA1'], r['eventsA1'],
        r['coresTotal'], r['coresWorkers'], fo, max(r.get('workerRunDelayMs') or [0]) / 1000,
        ' '.join('%.0f/%.0f' % (p['procP50'], p['procMax']) for p in sp) if sp else '-'))
