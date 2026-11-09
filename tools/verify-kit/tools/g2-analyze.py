#!/usr/bin/env python3
"""G2 latency analyzer for RT-D3 vs E13 (ci1, 2026-11-07).

Reads only latDump JSON files (LatencyTracker records) and report.json from each
run directory. The records are cumulative; records in a dump are filtered to
seq > the previous dump's max seq (same rule as lat-ab.py), so a record belongs
to the phase that created it.

Primary gate metric (documented in MEASUREMENT-CONDITIONS.md):
  queue insertion: tInject - tEvent for mouse pointermove (kind=1) records created
  by the Mixer-header drag in section c-mixer-drag.
Secondary user-visible T metrics:
  queue insertion: tInject - tPage for pointermove records;
  answered-input latency: first answered frame - tPage for pointerdown (kind=2)
  and keydown (kind=5) records in the a-idle-menu and b-play-menu sections.
Percentiles are computed on sorted values using the same order-statistic as
lat-ab.py percentile(v, p) = sorted[max(0, ceil(p/100*n)-1)].
"""
import sys, json, os, statistics
ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu']

def pct(vals, p):
    if not vals:
        return None
    v = sorted(vals)
    return v[min(len(v) - 1, max(0, ((p * len(v) + 99) // 100) - 1))]

def sec_records(path):
    out = {}
    prev = -1
    for sec in ORDER:
        try:
            d = json.load(open(os.path.join(path, f'lat-{sec}.json')))
        except FileNotFoundError:
            continue
        recs = d.get('records', [])
        out[sec] = [r for r in recs if (r.get('seq') or 0) > prev]
        if recs:
            prev = max(prev, max(r.get('seq') or 0 for r in recs))
    return out

def vals(recs, fn):
    r = []
    for x in recs:
        try:
            v = fn(x)
        except Exception:
            v = None
        if isinstance(v, (int, float)):
            r.append(float(v))
    return r

def fmt(values):
    v = sorted(values)
    if not v:
        return 'n=0'
    return (f"n={len(v)} median={statistics.median(v):.2f} p90={pct(v,90):.2f} "
            f"p99={pct(v,99):.2f} max={max(v):.2f}")

def summarize(recs, kind):
    rr = [r for r in recs if r.get('kind') == kind and not r.get('dropped') and r.get('tInject') is not None]
    return {
        'sent': len(rr),
        'queue_event_inject': vals(rr, lambda r: r['tInject'] - r['tEvent'] if r.get('tEvent') is not None else None),
        'queue_page_inject': vals(rr, lambda r: r['tInject'] - r['tPage']),
        'T_page_frame': vals(rr, lambda r: (min(r['frames'].values()) - r['tPage']) if r.get('frames') else None),
        'outcomes': {o: sum(1 for r in rr if r.get('outcome') == o) for o in sorted({r.get('outcome') for r in rr})},
    }

def main():
    e13 = sec_records(sys.argv[1])
    rt = sec_records(sys.argv[2])
    print('| section/metric | E13 | RT-D3 | RT-D3 - E13 median |')
    print('|---|---|---|---|')
    for sec in ('a-idle-menu', 'c-mixer-drag', 'b-play-menu'):
        for kind, name in ((1, 'pointer move'), (2, 'button down (File click)'), (5, 'key down (Esc/Space)')):
            a = summarize(e13.get(sec, []), kind)
            b = summarize(rt.get(sec, []), kind)
            for metric in ('queue_event_inject', 'queue_page_inject', 'T_page_frame'):
                av, bv = a[metric], b[metric]
                diff = (statistics.median(bv) - statistics.median(av)) if av and bv else None
                print(f"| {sec} / {name} / {metric} | {fmt(av)} | {fmt(bv)} | {diff if diff is None else f'{diff:+.2f}'} |")
    # gate
    a = summarize(e13.get('c-mixer-drag', []), 1)['queue_event_inject']
    b = summarize(rt.get('c-mixer-drag', []), 1)['queue_event_inject']
    if a and b:
        print(f"\nGATE G2 (pointermove, queue insertion, median): E13={statistics.median(a):.2f} ms, RT-D3={statistics.median(b):.2f} ms, diff={statistics.median(b)-statistics.median(a):+.2f} ms -> {'PASS (<= +10 ms)' if statistics.median(b)-statistics.median(a) <= 10 else 'FAIL (> +10 ms)'}")

if __name__ == '__main__':
    main()
