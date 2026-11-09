#!/usr/bin/env python3
"""pw1 (decision 0186): measured required cores against the estimates of decisions 0173 (mt1) / 0181 (hyb2).
usage: estimate.py OUT.tsv rows1.jsonl [rows2.jsonl ...]
Formula (hyb2 section 7 / mt1 budget): cores = (sum of the plug-ins' ms per block) / 92.879 ms; Serum ms = the node minimum per block of the hybrid N=123 (hyb2 table: 32/27/23/20/64/87 for the states 0/1/3/4/5 and
the 4-note chord; this tree's own node minima 31.0/28.0/24.0/20.9/65.0/88.0), silent 1.0 ms (hyb2 idle), OTT 11.3 ms (mt1/hyb2 assumption). Capacity of the reading = (hardwareConcurrency - 2) * e, e = 0.6/0.8/1.0."""
import json, sys, collections

P = 4096 / 44100 * 1000
HYB2 = {'0': 32.0, '1': 27.0, '3': 23.0, '4': 20.0, '5': 64.0, '3c': 87.0}
MINE = {'0': 31.0, '1': 28.0, '3': 24.0, '4': 20.9, '5': 65.0, '3c': 88.0}
OTT = 11.3
SILENT = 1.0
out = sys.argv[1]
groups = collections.OrderedDict()
for f in sys.argv[2:]:
    for l in open(f):
        r = json.loads(l)
        if r.get('mode') == 'ring': continue
        if 'serum' not in r: continue
        groups.setdefault((r['tag'] + ('' if r.get('warmms') else '[w150]'), r['hc'], r['serum'], r['ott']), []).append(r)

def est(table, serum, ott, ottms=None):
    ms = 0.0
    for e in [x for x in serum.split(',') if x]:
        ms += SILENT if e.endswith('s') else table[e]
    return (ms + ott * (OTT if ottms is None else ottms)) / P

hdr = ['tag', 'hc', 'serum', 'ott', 'reps', 'est_hyb2_table', 'est_this_tree_node_min', 'est_node_min_with_OTT_10.5ms', 'sum_Chrome_proc_mean/P', 'measured_worker_threads_cores', 'measured_whole_chrome_cores', 'measured_workers/est_hyb2', 'capacity_e0.6', 'capacity_e0.8', 'capacity_e1.0']
rows = [hdr]
for (tag, hc, serum, ott), rs in groups.items():
    e1, e2, e3 = est(HYB2, serum, ott), est(MINE, serum, ott), est(MINE, serum, ott, 10.5)
    sp = sum(sum(p['procMean'] for p in r['per'] if p.get('procMean') is not None) / r['periodMs'] for r in rs) / len(rs)
    w = sum(r['coresWorkers'] for r in rs) / len(rs)
    t = sum(r['coresTotal'] for r in rs) / len(rs)
    cap = [max(0, hc - 2) * e for e in (0.6, 0.8, 1.0)]
    rows.append([tag, hc, serum, ott, len(rs), '%.2f' % e1, '%.2f' % e2, '%.2f' % e3, '%.2f' % sp, '%.2f' % w, '%.2f' % t, '%.2f' % (w / e1) if e1 else '', *['%.1f' % c for c in cap]])
with open(out, 'w') as f:
    for r in rows:
        f.write('\t'.join(str(x) for x in r) + '\n')
print(open(out).read())
