#!/usr/bin/env python3
"""lat1: the trace table from runs/trace-*/join.json. usage: mk-trace-table.py RUNS_DIR"""
import json, glob, os, sys
def q(v, p):
    v = sorted(x for x in v if x is not None)
    return None if not v else v[min(len(v) - 1, max(0, -(-p * len(v) // 100) - 1))]
def f(x): return '-' if x is None else ('%.1f' % x)
print('| config | state | changing draws joined | put -> rAF callback p50 / p95 | rAF callback -> compositor frame presented (PipelineReporter end) p50 / p95 | put -> presented p50 / p95 | compositor frames in the trace (presented all / partial / dropped / no update) |')
print('|---|---|---:|---|---|---|---|')
for d in sorted(glob.glob(sys.argv[1] + '/trace-*-*/')):
    j = os.path.join(d, 'join.json')
    if not os.path.exists(j): continue
    r = json.load(open(j)); rows = r['rows']; sc = r['stateCount']
    cfg, st = os.path.basename(d.rstrip('/'))[len('trace-'):].rsplit('-', 1)
    if cfg == 'hz120': continue   # the compositor's frames overlap at 125 draws/s and the begin/end pairing of PipelineReporter is unreliable: not reported
    pa = [x['putToPresented'] for x in rows]; ra = [x['rafToPresented'] for x in rows]; pr = [x['putToRaf'] for x in rows]
    print('| %s | %s | %d | %s / %s | %s / %s | %s / %s | %d / %d / %d / %d |' % (cfg, st, len(rows), f(q(pr, 50)), f(q(pr, 95)), f(q(ra, 50)), f(q(ra, 95)), f(q(pa, 50)), f(q(pa, 95)), sc.get('STATE_PRESENTED_ALL', 0), sc.get('STATE_PRESENTED_PARTIAL', 0), sc.get('STATE_DROPPED', 0), sc.get('STATE_NO_UPDATE_DESIRED', 0)))
