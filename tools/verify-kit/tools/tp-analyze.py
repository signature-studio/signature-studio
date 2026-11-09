#!/usr/bin/env python3
"""tp-analyze.py RUNDIR [TAG] (ci1, 2026-11-08, PM 05:45: G2 playback T +23 ms split)

RUNDIR holds tp.trace.json (CDP Tracing: toplevel + devtools.timeline + v8.cpu_profiler of every thread of the renderer), sync-a/b.json (console.timeStamp in the trace + page clock),
lat-tp-play.json (the page's own LatencyTracker records of the File-menu clicks / Escapes during playback).

Per input record (button-down = File click, key-down = Escape): the page clock (epoch ms) is mapped to trace ts (us) with the sync events, then
  window W = [tPage - 200 ms, tPage + 200 ms]  (PM: 200 ms before / after)
  phase segments: A tEvent -> tPage (browser to page listener), B tPage -> tInject (page to engine worker), C tInject -> firstFrame (engine worker processing + frame post),
                  D firstFrame -> raf (page: receive + present until the next animation frame)
For W and for the phases B/C the v8 CPU profile samples of every profiled thread are classed by the self function of each sample: idle, program, wasm (named or wasm-function[N]), JS name, GC, other.
Also from the trace: RunTask time on the renderer main thread and on the Workers inside W, and Atomics.wait / long task events when present.
"""
import sys, json, collections, statistics, re

def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, max(0, -(-p * len(v) // 100) - 1))] if v else None

def load(d):
    t = json.load(open(d + '/tp.trace.json'))['traceEvents']
    sa = json.load(open(d + '/sync-a.json')); sb = json.load(open(d + '/sync-b.json'))
    ts = [e for e in t if e.get('name') == 'TimeStamp']
    ta = [e for e in ts if 'sync-a' in e['args']['data']['message']][0]; tb = [e for e in ts if 'sync-b' in e['args']['data']['message']][0]
    off = (sa['epochMs'] * 1000 - ta['ts'] + sb['epochMs'] * 1000 - tb['ts']) / 2
    names = {(e['pid'], e['tid']): e['args']['name'] for e in t if e.get('ph') == 'M' and e.get('name') == 'thread_name'}
    return t, off, ta['pid'], names

def profiles(t, rpid):
    prof = {}
    for e in t:
        if e.get('name') == 'Profile' and e['pid'] == rpid:
            prof[e['id']] = dict(tid=e['tid'], start=e['args']['data']['startTime'], nodes={}, samples=[], times=[])
    last = {}
    for e in t:
        if e.get('name') != 'ProfileChunk' or e['pid'] != rpid: continue
        p = prof.get(e['id'])
        if not p: continue
        cp = e['args']['data']['cpuProfile']
        for n in cp.get('nodes', []): p['nodes'][n['id']] = n['callFrame']
        s = cp.get('samples', []); td = e['args']['data'].get('timeDeltas', [])
        cur = last.get(e['id'], p['start'])
        for sid, d in zip(s, td):
            cur += d; p['samples'].append(sid); p['times'].append(cur)
        last[e['id']] = cur
    return prof

def klass(cf):
    fn = cf.get('functionName') or ''
    url = cf.get('url') or ''
    if fn == '(idle)': return 'idle'
    if fn == '(program)': return 'program'
    if fn == '(garbage collector)': return 'gc'
    if fn == '(root)': return 'root'
    if cf.get('codeType') == 'other' and not fn: return 'other'
    if url.startswith('wasm') or url == '' and (fn.startswith('wasm-function') or cf.get('codeType') == 'other'): return 'wasm:' + fn
    if 'wasm' in url: return 'wasm:' + fn
    return 'js:' + fn

def bucket(k):
    if k in ('idle', 'program', 'gc', 'root', 'other'): return k
    if k.startswith('wasm:'): return 'wasm'
    if k.startswith('js:'): return 'js'
    return 'other'

def main():
    d = sys.argv[1]; tag = sys.argv[2] if len(sys.argv) > 2 else d
    t, off, rpid, names = load(d)
    prof = profiles(t, rpid)
    pn = {pid: names.get((rpid, p['tid']), '?') for pid, p in prof.items()}
    # thread roles: the Worker with the most RunTask = UI worker (the one that receives the page's input); the others = mixer / pthread workers
    cnt = collections.Counter(e['tid'] for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask')
    wk = [tid for tid in cnt if names.get((rpid, tid)) == 'DedicatedWorker thread']
    j = json.load(open(d + '/lat-tp-play.json'))
    recs = [r for r in j['records'] if r.get('frames') and json.load(open(d + '/sync-a.json'))['epochMs'] <= r['tPage'] <= json.load(open(d + '/sync-b.json'))['epochMs']]   # only inputs inside the trace window (the clicks of closeWizard / openZipProject precede traceStart)
    print(f'## {tag}: {len(recs)} answered records, profiles {len(prof)}')
    for pid, p in prof.items():
        print(f'- profile {pid}: thread {p["tid"]} {pn[pid]} samples {len(p["samples"])}')
    def to_us(ms): return ms * 1000 - off
    out = {}
    for kind, nm in ((2, 'File click (button-down)'), (5, 'Escape (key-down)')):
        rr = [r for r in recs if r['kind'] == kind]
        if not rr: continue
        seg = collections.defaultdict(list)
        for r in rr:
            f = min(r['frames'].values()); a = min(r['answers'].values(), key=lambda x: x['at'])
            seg['A tEvent->tPage'].append(r['tPage'] - r['tEvent']); seg['B tPage->tInject'].append(r['tInject'] - r['tPage'])
            seg['C tInject->firstFrame'].append(f - r['tInject']); seg['D firstFrame->raf'].append(a['raf'] - a['at']); seg['T tPage->firstFrame'].append(f - r['tPage'])
            seg['T2 tPage->raf'].append(a['raf'] - r['tPage'])
        print(f'\n### {nm}: n={len(rr)}')
        print('| segment | median ms | p90 | max |'); print('|---|---|---|---|')
        for k in sorted(seg):
            v = seg[k]; print(f'| {k} | {statistics.median(v):.2f} | {pct(v, 90):.2f} | {max(v):.2f} |')
        # per-thread samples in W and in phases B, C
        for wname, lo_f, hi_f in (('W = tPage-200ms .. tPage+200ms', lambda r, f, a: r['tPage'] - 200, lambda r, f, a: r['tPage'] + 200),
                                  ('B+C = tPage .. firstFrame', lambda r, f, a: r['tPage'], lambda r, f, a: f)):
            agg = collections.defaultdict(lambda: collections.Counter()); tot = collections.defaultdict(int); fn = collections.defaultdict(collections.Counter)
            for r in rr:
                f = min(r['frames'].values()); a = min(r['answers'].values(), key=lambda x: x['at'])
                lo, hi = to_us(lo_f(r, f, a)), to_us(hi_f(r, f, a))
                for pid, p in prof.items():
                    ts_ = p['times']
                    # binary search
                    import bisect
                    i0, i1 = bisect.bisect_left(ts_, lo), bisect.bisect_left(ts_, hi)
                    for sid in p['samples'][i0:i1]:
                        cf = p['nodes'].get(sid, {}); k = klass(cf); agg[pid][bucket(k)] += 1; tot[pid] += 1
                        if bucket(k) in ('wasm', 'js'): fn[pid][k] += 1
            print(f'\n#### samples by self class, {wname} (sum over {len(rr)} inputs)')
            print('| thread (profile) | samples | idle | program | wasm | js | gc | other |'); print('|---|---|---|---|---|---|---|---|')
            for pid in sorted(prof, key=lambda x: -tot[x]):
                n = tot[pid] or 1; c = agg[pid]
                print(f'| {pn[pid]} {prof[pid]["tid"]} | {tot[pid]} | ' + ' | '.join(f'{100 * c[b] / n:.1f}%' for b in ('idle', 'program', 'wasm', 'js', 'gc', 'other')) + ' |')
            print(f'\n#### top self functions (wasm/js), {wname}')
            for pid in sorted(prof, key=lambda x: -tot[x])[:4]:
                if not fn[pid]: continue
                n = tot[pid] or 1
                print(f'- {pn[pid]} {prof[pid]["tid"]}: ' + '; '.join(f'{k} {100 * v / n:.1f}%' for k, v in fn[pid].most_common(8)))
        # RunTask busy time on main and workers inside W (and the longest RunTask)
        busy = collections.defaultdict(float); longest = collections.defaultdict(float); ntask = collections.defaultdict(int)
        for r in rr:
            lo, hi = to_us(r['tPage'] - 200), to_us(r['tPage'] + 200)
            for e in t:
                pass
        print()
    # Single pass for RunTask inside the union of windows
    wins = []
    for kind in (2, 5):
        for r in recs:
            if r['kind'] == kind: wins.append((kind, to_us(r['tPage'] - 200), to_us(r['tPage'] + 200)))
    byt = collections.defaultdict(lambda: collections.defaultdict(lambda: [0.0, 0, 0.0]))
    import bisect
    ev = [e for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask']
    for e in ev:
        for kind, lo, hi in wins:
            if e['ts'] + e['dur'] > lo and e['ts'] < hi:
                s = byt[kind][e['tid']]; s[0] += min(e['ts'] + e['dur'], hi) - max(e['ts'], lo); s[1] += 1; s[2] = max(s[2], e['dur'])
    for kind, nm in ((2, 'File click'), (5, 'Escape')):
        n = sum(1 for r in recs if r['kind'] == kind)
        print(f'\n#### RunTask busy ms per input inside W (+-200 ms = 400 ms), {nm} (n={n})')
        print('| thread | busy ms per input | tasks per input | longest task ms |'); print('|---|---|---|---|')
        for tid, s in sorted(byt[kind].items(), key=lambda kv: -kv[1][0]):
            print(f'| {names.get((rpid, tid), "?")} {tid} | {s[0] / 1000 / max(1, n):.1f} | {s[1] / max(1, n):.1f} | {s[2] / 1000:.2f} |')

main()
