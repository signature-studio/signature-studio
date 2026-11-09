#!/usr/bin/env python3
"""tp-analyze5.py RUNDIR TAG (ci1, 2026-11-08, PM 05:45): shape of phase C (tInject .. firstFrame) of the File clicks on the UI worker.
For every File click: the UI worker's samples in 2 ms bins relative to tInject (class per bin by time weight), the contiguous '(program)' runs >= 1 ms (start offset, length),
and for each such run which other Workers were in wasm / (program) / idle during the run (fraction by time).
Output: per-click compact timeline + the aggregate of the runs. Markdown on stdout."""
import sys, json, collections, bisect, statistics
exec(open(__file__.rsplit('/', 1)[0] + '/tp-analyze2.py').read().split('def main():')[0])

def main():
    d = sys.argv[1]; tag = sys.argv[2]
    t, off, rpid, names = load(d)
    prof = profiles(t, rpid)
    cnt = collections.Counter(e['tid'] for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask')
    wk = [tid for tid in cnt if names.get((rpid, tid)) == 'DedicatedWorker thread']
    ui = max(wk, key=lambda x: cnt[x])
    j = json.load(open(d + '/lat-tp-play.json'))
    recs = [r for r in j['records'] if r.get('frames') and r['kind'] == 2 and json.load(open(d + '/sync-a.json'))['epochMs'] <= r['tPage'] <= json.load(open(d + '/sync-b.json'))['epochMs']]
    to_us = lambda ms: ms * 1000 - off
    pu = [q for q in prof.values() if q['tid'] == ui][0]
    others = [q for q in prof.values() if q['tid'] != ui and names.get((rpid, q['tid'])) == 'DedicatedWorker thread']
    print(f'## {tag}: File clicks n={len(recs)}, UI worker tid {ui}, other Workers {len(others)}')
    runs_all = []
    for ci, r in enumerate(recs):
        lo, hi = to_us(r['tInject']), to_us(min(r['frames'].values()))
        i0, i1 = bisect.bisect_left(pu['times'], lo), bisect.bisect_left(pu['times'], hi)
        seq = [(pu['times'][i] - lo, pu['dts'][i], cls(pu['nodes'].get(pu['samples'][i], {}))) for i in range(i0, i1)]
        # runs of (program) allowing 1 non-program sample inside? no: strict contiguous
        runs = []; cur = None
        for tt, dt, c in seq:
            if c == 'program':
                if cur is None: cur = [tt, dt]
                else: cur[1] += dt
            else:
                if cur is not None: runs.append(cur); cur = None
        if cur is not None: runs.append(cur)
        big = [x for x in runs if x[1] >= 1000]
        span = (hi - lo) / 1000
        # 2 ms bins
        nb = int(span // 2) + 1; bins = [collections.Counter() for _ in range(nb)]
        for tt, dt, c in seq:
            bins[min(nb - 1, int(tt // 2000))][c] += dt
        line = ''.join(('P' if b['program'] > 0.5 * sum(b.values()) else 'w' if b['wasm'] > 0.5 * sum(b.values()) else 'i' if b['idle'] > 0.5 * sum(b.values()) else 'j') if sum(b.values()) else '.' for b in bins)
        print(f'- click {ci + 1}: C span {span:.1f} ms; (program) total {sum(x[1] for x in runs) / 1000:.1f} ms in {len(runs)} runs, runs >= 1 ms: ' + ', '.join(f'@{x[0] / 1000:.1f}+{x[1] / 1000:.1f}' for x in big) + f'\n    2 ms bins [P=(program) w=wasm i=idle j=js]: {line}')
        for x in big:
            a, b = lo + x[0], lo + x[0] + x[1]
            row = collections.Counter(); tot = 0
            for q in others:
                k0, k1 = bisect.bisect_left(q['times'], a), bisect.bisect_left(q['times'], b)
                for sid, dt in zip(q['samples'][k0:k1], q['dts'][k0:k1]):
                    row[cls(q['nodes'].get(sid, {}))] += dt; tot += dt
            runs_all.append((x[1] / 1000, {k: v / tot for k, v in row.items()} if tot else {}))
    if runs_all:
        L = [a for a, _ in runs_all]
        print(f'\n### (program) runs >= 1 ms on the UI worker in phase C: {len(L)} runs over {len(recs)} clicks; length median {statistics.median(L):.1f} ms, max {max(L):.1f} ms, total {sum(L) / len(recs):.1f} ms per click')
        for c in ('wasm', 'program', 'idle', 'js'):
            vv = [m.get(c, 0.0) for _, m in runs_all if m]
            print(f'- other Workers during these runs, mean share {c}: {100 * statistics.mean(vv):.0f}%')
main()
