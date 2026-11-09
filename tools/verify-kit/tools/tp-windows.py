#!/usr/bin/env python3
"""tp-windows.py RUNDIR TAG [--top N]   (ci1, 2026-11-08, PM 08:18: a per-input window table from an EXISTING trace; no browser)

RUNDIR holds tp.trace.json (CDP Tracing with the v8 cpu_profiler), sync-a.json / sync-b.json (console.timeStamp in the trace + the page clock) and lat-tp-play.json (the page's own latency records), as written by tools/run-tp.sh.

For every File click (button-down, kind 2) that was answered and lies between the two sync events (the same selection as tp-analyze*.py), one row:
  - the input's time: tPage as ms after the first sync event (so rows of different runs are comparable by position in the trace), and the phase boundaries
  - W  = the window [tPage - 200 ms, tPage + 200 ms] (400 ms)
  - C  = phase C of that input = [tInject, first frame]  (inside W)
For the UI Worker (the DedicatedWorker with the most RunTask tasks), over W and over C separately, from the v8 CPU profile samples (weight = the sample's time delta):
  (program) ms, (idle) ms, wasm self ms in total, the sum of the fle_gil_* self ms, and the three largest wasm / named self functions.
Nothing is judged here: the rows are numbers only. A second table gives the same columns summed over all inputs of the run (ms per input).

Method notes (what the columns are, and are not):
  - the time base: trace ts (us, TimeTicks) is mapped to the page clock (epoch ms) by the two console.timeStamp events (offset = mean of the two; they differ by 11-15 us in the runs of 0343-0345).
  - 'wasm self' = samples whose self frame is 'wasm-function[N]' (no name in the name section) or a named wasm function; 'named' = a self frame that has a name (fle_*, g9_hook, poll_waiters, ...); '(program)' = V8 attributes the sample to no JS / wasm function (blocked or native time: it is NOT known which).
  - the other (pthread) Workers are not in this table.
  - windows of neighbouring inputs would overlap if the inputs were < 400 ms apart; in the runs of 0343-0345 the inputs are about 2.1-2.4 s apart, so the windows do not overlap (the sum table counts each input's own window once per input)."""
import sys, json, collections, bisect, statistics

def main():
    argv = sys.argv[1:]
    top = 3
    if '--top' in argv:
        i = argv.index('--top'); top = int(argv[i + 1]); del argv[i:i + 2]
    d, tag = argv[0], argv[1]
    t = json.load(open(d + '/tp.trace.json'))['traceEvents']
    sa = json.load(open(d + '/sync-a.json')); sb = json.load(open(d + '/sync-b.json'))
    ts = [e for e in t if e.get('name') == 'TimeStamp']
    ta = [e for e in ts if 'sync-a' in e['args']['data']['message']][0]; tb = [e for e in ts if 'sync-b' in e['args']['data']['message']][0]
    off = (sa['epochMs'] * 1000 - ta['ts'] + sb['epochMs'] * 1000 - tb['ts']) / 2
    rpid = ta['pid']
    names = {(e['pid'], e['tid']): e['args']['name'] for e in t if e.get('ph') == 'M' and e.get('name') == 'thread_name'}
    cnt = collections.Counter(e['tid'] for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask')
    wk = [tid for tid in cnt if names.get((rpid, tid)) == 'DedicatedWorker thread']
    ui = max(wk, key=lambda x: cnt[x])
    # the UI Worker's profile
    prof = None
    for e in t:
        if e.get('name') == 'Profile' and e['pid'] == rpid and e['tid'] == ui:
            prof = dict(start=e['args']['data']['startTime'], id=e['id'], nodes={}, parent={}, samples=[], times=[], dts=[]); break
    cur = prof['start']
    for e in t:
        if e.get('name') != 'ProfileChunk' or e['pid'] != rpid or e['id'] != prof['id']: continue
        cp = e['args']['data']['cpuProfile']
        for n in cp.get('nodes', []):
            prof['nodes'][n['id']] = n['callFrame']
            for ch in n.get('children', []): prof['parent'][ch] = n['id']
            if 'parent' in n: prof['parent'][n['id']] = n['parent']
        for sid, dt in zip(cp.get('samples', []), e['args']['data'].get('timeDeltas', [])):
            cur += dt; prof['samples'].append(sid); prof['times'].append(cur); prof['dts'].append(dt)
    def label(sid):
        cf = prof['nodes'].get(sid, {}); fn = cf.get('functionName') or ''
        if not fn: return '(anonymous)'
        return fn
    cls_cache = {}
    def cls(sid):
        if sid in cls_cache: return cls_cache[sid]
        cf = prof['nodes'].get(sid, {}); fn = cf.get('functionName') or ''; url = cf.get('url') or ''
        if fn == '(idle)': c = 'idle'
        elif fn == '(program)': c = 'program'
        elif fn == '(garbage collector)': c = 'gc'
        elif fn.startswith('fle_gil'): c = 'gil'
        elif fn.startswith('wasm-function') or url.startswith('wasm') or 'wasm' in url: c = 'wasm_anon'
        elif cf.get('codeType') == 'JS': c = 'js'
        else: c = 'named'   # a function with a name from the name section (fle_*, g9_hook, ...) or other
        cls_cache[sid] = c; return c
    a0 = sa['epochMs']; b0 = sb['epochMs']
    j = json.load(open(d + '/lat-tp-play.json'))
    recs = [r for r in j['records'] if r.get('frames') and r['kind'] == 2 and a0 <= r['tPage'] <= b0]
    to_us = lambda ms: ms * 1000 - off
    def span(lo_ms, hi_ms):
        lo, hi = to_us(lo_ms), to_us(hi_ms)
        i0, i1 = bisect.bisect_left(prof['times'], lo), bisect.bisect_left(prof['times'], hi)
        acc = collections.Counter(); fn = collections.Counter(); gil = collections.Counter()
        for sid, dt in zip(prof['samples'][i0:i1], prof['dts'][i0:i1]):
            c = cls(sid); acc[c] += dt; acc['total'] += dt
            if c in ('wasm_anon', 'named', 'gil'): fn[label(sid)] += dt
            if c == 'gil': gil[label(sid)] += dt
        return acc, fn, gil
    def fmt_top(fn, k):
        return '; '.join(f'{n} {v / 1000:.2f}' for n, v in fn.most_common(k))
    print(f'## {tag}: UI Worker tid {ui}; File clicks between the sync events: {len(recs)}; profile samples {len(prof["samples"])}')
    print('\nUnits: ms (not per input) inside each window. W = tPage +-200 ms (400 ms), C = tInject .. first frame.\n')
    print('| # | tPage (ms after sync-a) | A tEvent->tPage | B tPage->tInject | C span (tInject->first frame) | C: total | C: (program) | C: (idle) | C: wasm anon self | C: named self | C: fle_gil_* self | C: top self functions (ms) | W: total | W: (program) | W: (idle) | W: wasm anon self | W: named self | W: fle_gil_* self | W: top self functions (ms) |')
    print('|' + '---|' * 19)
    sumC = collections.Counter(); sumW = collections.Counter(); fnC = collections.Counter(); fnW = collections.Counter(); gC = collections.Counter(); gW = collections.Counter()
    for i, r in enumerate(sorted(recs, key=lambda r: r['tPage']), 1):
        ff = min(r['frames'].values())
        cC, fC, gc_ = span(r['tInject'], ff)
        cW, fW, gw_ = span(r['tPage'] - 200, r['tPage'] + 200)
        sumC.update(cC); sumW.update(cW); fnC.update(fC); fnW.update(fW); gC.update(gc_); gW.update(gw_)
        m = lambda c, k: f'{c[k] / 1000:.2f}'
        print(f'| {i} | {r["tPage"] - a0:.0f} | {r["tPage"] - r["tEvent"]:.2f} | {r["tInject"] - r["tPage"]:.2f} | {ff - r["tInject"]:.2f} | {m(cC, "total")} | {m(cC, "program")} | {m(cC, "idle")} | {m(cC, "wasm_anon")} | {m(cC, "named")} | {m(cC, "gil")} | {fmt_top(fC, top)} | {m(cW, "total")} | {m(cW, "program")} | {m(cW, "idle")} | {m(cW, "wasm_anon")} | {m(cW, "named")} | {m(cW, "gil")} | {fmt_top(fW, top)} |')
    n = len(recs)
    print(f'\n### {tag}: sum over the {n} inputs, ms per input (the same columns)')
    print('| window | total | (program) | (idle) | wasm anon self | named self | fle_gil_* self | js | gc | top self functions, ms per input |')
    print('|---|---|---|---|---|---|---|---|---|---|')
    for nm, c, f in (('C', sumC, fnC), ('W (400 ms)', sumW, fnW)):
        pr = lambda k: f'{c[k] / 1000 / n:.2f}'
        print(f'| {nm} | {pr("total")} | {pr("program")} | {pr("idle")} | {pr("wasm_anon")} | {pr("named")} | {pr("gil")} | {pr("js")} | {pr("gc")} | ' + '; '.join(f'{k} {v / 1000 / n:.2f}' for k, v in f.most_common(6)) + ' |')
    print(f'\nfle_gil_* self, ms per input: C: ' + (', '.join(f'{k} {v / 1000 / n:.2f}' for k, v in gC.most_common()) or 'none') + ' | W: ' + (', '.join(f'{k} {v / 1000 / n:.2f}' for k, v in gW.most_common()) or 'none'))
main()
