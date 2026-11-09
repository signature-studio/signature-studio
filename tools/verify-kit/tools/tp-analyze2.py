#!/usr/bin/env python3
"""tp-analyze2.py RUNDIR TAG (ci1, 2026-11-08, PM 05:45): second pass of the G2 playback-T split.
 1. thread CPU fraction inside the windows (thread time `tts` against wall `ts` of the events of every renderer thread): tells a blocked thread (Atomics.wait / futex: V8 reports it as '(program)') from a computing one
 2. the UI worker (the DedicatedWorker with the most RunTask) in phase C = tInject .. firstFrame of every File click / Escape: ms per input by class (idle / program / wasm / js / gc), self and inclusive time by function (names from the name section where the build has one)
 3. the same ms/input for the other Workers (the mixer / pthread Workers), phase C and the 400 ms window
Sample weight = the time delta of the sample (us). Output = markdown on stdout."""
import sys, json, collections, bisect, statistics

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
            prof[e['id']] = dict(tid=e['tid'], start=e['args']['data']['startTime'], nodes={}, parent={}, samples=[], times=[], dts=[])
    last = {}
    for e in t:
        if e.get('name') != 'ProfileChunk' or e['pid'] != rpid: continue
        p = prof.get(e['id'])
        if not p: continue
        cp = e['args']['data']['cpuProfile']
        for n in cp.get('nodes', []):
            p['nodes'][n['id']] = n['callFrame']
            if 'parent' in n: p['parent'][n['id']] = n['parent']
            for ch in n.get('children', []): p['parent'][ch] = n['id']
        s = cp.get('samples', []); td = e['args']['data'].get('timeDeltas', [])
        cur = last.get(e['id'], p['start'])
        for sid, d in zip(s, td):
            cur += d; p['samples'].append(sid); p['times'].append(cur); p['dts'].append(d)
        last[e['id']] = cur
    return prof

def fname(cf):
    fn = cf.get('functionName') or ''
    url = cf.get('url') or ''
    if fn in ('(idle)', '(program)', '(garbage collector)', '(root)'): return fn
    if not fn: return '(anonymous)' + (' ' + url.rsplit('/', 1)[-1] if url else '')
    return fn

def cls(cf):
    fn = cf.get('functionName') or ''; url = cf.get('url') or ''
    if fn == '(idle)': return 'idle'
    if fn == '(program)': return 'program'
    if fn == '(garbage collector)': return 'gc'
    if fn.startswith('wasm-function') or url.startswith('wasm') or 'wasm' in url: return 'wasm'
    if cf.get('codeType') == 'JS': return 'js'
    return 'other'

def main():
    d = sys.argv[1]; tag = sys.argv[2]
    t, off, rpid, names = load(d)
    prof = profiles(t, rpid)
    cnt = collections.Counter(e['tid'] for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask')
    wk = [tid for tid in cnt if names.get((rpid, tid)) == 'DedicatedWorker thread']
    ui = max(wk, key=lambda x: cnt[x])
    j = json.load(open(d + '/lat-tp-play.json'))
    recs = [r for r in j['records'] if r.get('frames') and json.load(open(d + '/sync-a.json'))['epochMs'] <= r['tPage'] <= json.load(open(d + '/sync-b.json'))['epochMs']]   # only inputs inside the trace window (the clicks of closeWizard / openZipProject precede traceStart)
    to_us = lambda ms: ms * 1000 - off
    print(f'## {tag}: UI worker = tid {ui} (most RunTask: {cnt[ui]}); Workers: {sorted(wk)}')
    # 1. thread CPU fraction inside the windows
    print('\n### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread')
    wins = [(to_us(r['tPage'] - 200), to_us(r['tPage'] + 200)) for r in recs]
    evs = collections.defaultdict(list)
    for e in t:
        if e.get('pid') == rpid and 'tts' in e and 'ts' in e and e.get('ph') in ('X', 'I', 'B', 'P'):
            evs[e['tid']].append((e['ts'], e['tts']))
    print('| thread | role | events | wall ms | CPU ms | CPU fraction |'); print('|---|---|---|---|---|---|')
    for tid in sorted(evs, key=lambda x: -len(evs[x])):
        nm = names.get((rpid, tid), '?')
        if nm not in ('DedicatedWorker thread', 'CrRendererMain', 'Realtime AudioWorklet thread', 'Compositor'): continue
        v = sorted(evs[tid]); tsl = [a for a, _ in v]
        wall = 0.0; cpu = 0.0; n = 0
        for lo, hi in wins:
            i0, i1 = bisect.bisect_left(tsl, lo), bisect.bisect_right(tsl, hi)
            if i1 - i0 >= 2:
                wall += v[i1 - 1][0] - v[i0][0]; cpu += v[i1 - 1][1] - v[i0][1]; n += i1 - i0
        role = 'UI worker' if tid == ui else ('Worker' if nm.startswith('Dedicated') else nm)
        print(f'| {nm} {tid} | {role} | {n} | {wall / 1000:.0f} | {cpu / 1000:.0f} | {cpu / wall if wall else 0:.2f} |')
    # 2. per-thread class ms per input in phase C and in W
    for kind, nmk in ((2, 'File click'), (5, 'Escape')):
        rr = [r for r in recs if r['kind'] == kind]
        if not rr: continue
        print(f'\n### {nmk} (n={len(rr)}): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)')
        print('| thread | role | phase | idle | program | wasm | js | gc | total ms |'); print('|---|---|---|---|---|---|---|---|---|')
        for pid, p in sorted(prof.items(), key=lambda kv: (kv[1]['tid'] != ui, kv[1]['tid'])):
            nm = names.get((rpid, p['tid']), '?')
            if nm == 'Realtime AudioWorklet thread': role = 'audio worklet'
            elif nm == 'CrRendererMain': role = 'page main'
            else: role = 'UI worker' if p['tid'] == ui else 'Worker'
            for ph, f in (('C', lambda r: (r['tInject'], min(r['frames'].values()))), ('W', lambda r: (r['tPage'] - 200, r['tPage'] + 200))):
                acc = collections.Counter()
                for r in rr:
                    lo, hi = f(r); lo, hi = to_us(lo), to_us(hi)
                    i0, i1 = bisect.bisect_left(p['times'], lo), bisect.bisect_left(p['times'], hi)
                    for sid, dt in zip(p['samples'][i0:i1], p['dts'][i0:i1]):
                        acc[cls(p['nodes'].get(sid, {}))] += dt
                tot = sum(acc.values())
                print(f'| {nm} {p["tid"]} | {role} | {ph} | ' + ' | '.join(f'{acc[c] / 1000 / len(rr):.1f}' for c in ('idle', 'program', 'wasm', 'js', 'gc')) + f' | {tot / 1000 / len(rr):.1f} |')
        # 3. UI worker functions in phase C: self and inclusive
        p = [q for q in prof.values() if q['tid'] == ui][0]
        selfms = collections.Counter(); incl = collections.Counter(); tot = 0
        cache = {}
        def stack(nid):
            if nid in cache: return cache[nid]
            s = []; x = nid; seen = 0
            while x is not None and seen < 400:
                s.append(fname(p['nodes'].get(x, {}))); x = p['parent'].get(x); seen += 1
            cache[nid] = s; return s
        for r in rr:
            lo, hi = to_us(r['tInject']), to_us(min(r['frames'].values()))
            i0, i1 = bisect.bisect_left(p['times'], lo), bisect.bisect_left(p['times'], hi)
            for sid, dt in zip(p['samples'][i0:i1], p['dts'][i0:i1]):
                st = stack(sid); tot += dt
                selfms[st[0]] += dt
                for nme in set(st):
                    if nme != '(root)': incl[nme] += dt
        print(f'\n#### UI worker functions in phase C, {nmk}: ms per input (total {tot / 1000 / len(rr):.1f} ms)')
        print('| function | self ms | inclusive ms |'); print('|---|---|---|')
        top = sorted(incl, key=lambda k: -incl[k])[:28]
        for k in top: print(f'| {k} | {selfms[k] / 1000 / len(rr):.2f} | {incl[k] / 1000 / len(rr):.2f} |')
        print('\n(self-time top 12): ' + '; '.join(f'{k} {v / 1000 / len(rr):.2f}' for k, v in selfms.most_common(12)))
main()
