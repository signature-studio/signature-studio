#!/usr/bin/env python3
"""tp-analyze4.py RUNDIR TAG [WASM] (ci1, 2026-11-08, PM 05:45): fourth pass.
 (a) coverage: the profile's sampled ms per input in phase C against the phase C span itself (tInject..firstFrame mean / median)
 (b) thread CPU (d tts) against wall (d ts) inside phase C for the UI worker and every other Worker: separates blocked time ('(program)' with CPU ~ 0) from computing
 (c) names of the top self wasm-function[N] of the UI worker in phase C from the wasm name section of WASM (only when the build has a name section: RT-F)
Output markdown on stdout."""
import sys, json, collections, bisect, statistics
exec(open(__file__.rsplit('/', 1)[0] + '/tp-analyze2.py').read().split('def main():')[0])

def name_map(path):
    f = open(path, 'rb'); f.read(8)
    while True:
        h = f.read(1)
        if not h: return {}
        size = 0; s = 0
        while True:
            x = f.read(1)[0]; size |= (x & 0x7f) << s; s += 7
            if not x & 0x80: break
        pos = f.tell()
        if h[0] == 0:
            b = f.read(min(size, 64)); l = b[0]
            if b[1:1 + l] == b'name':
                f.seek(pos); data = f.read(size); break
        f.seek(pos + size)
    def leb(b, i):
        r = 0; s = 0
        while True:
            x = b[i]; i += 1; r |= (x & 0x7f) << s; s += 7
            if not x & 0x80: return r, i
    l, i = leb(data, 0); i += l
    out = {}
    while i < len(data):
        sid = data[i]; i += 1; sz, i = leb(data, i); end = i + sz
        if sid == 1:
            n, j = leb(data, i)
            for _ in range(n):
                idx, j = leb(data, j); ln, j = leb(data, j); out[idx] = data[j:j + ln].decode('utf8', 'replace'); j += ln
        i = end
    return out

def main():
    d = sys.argv[1]; tag = sys.argv[2]; wasm = sys.argv[3] if len(sys.argv) > 3 else None
    nm = name_map(wasm) if wasm else {}
    t, off, rpid, names = load(d)
    prof = profiles(t, rpid)
    cnt = collections.Counter(e['tid'] for e in t if e.get('pid') == rpid and e.get('ph') == 'X' and e.get('name') == 'RunTask')
    wk = [tid for tid in cnt if names.get((rpid, tid)) == 'DedicatedWorker thread']
    ui = max(wk, key=lambda x: cnt[x])
    j = json.load(open(d + '/lat-tp-play.json'))
    recs = [r for r in j['records'] if r.get('frames') and json.load(open(d + '/sync-a.json'))['epochMs'] <= r['tPage'] <= json.load(open(d + '/sync-b.json'))['epochMs']]   # only inputs inside the trace window (the clicks of closeWizard / openZipProject precede traceStart)
    to_us = lambda ms: ms * 1000 - off
    evs = collections.defaultdict(list)
    for e in t:
        if e.get('pid') == rpid and 'tts' in e and 'ts' in e and e.get('ph') in ('X', 'I', 'B', 'P'):
            evs[e['tid']].append((e['ts'], e['tts']))
    for v in evs.values(): v.sort()
    print(f'## {tag}: names resolved from the wasm name section: {len(nm)}')
    for kind, nmk in ((2, 'File click'), (5, 'Escape')):
        rr = [r for r in recs if r['kind'] == kind]
        if not rr: continue
        spans = [min(r['frames'].values()) - r['tInject'] for r in rr]
        p = [q for q in prof.values() if q['tid'] == ui][0]
        cov = 0.0
        selfc = collections.Counter()
        for r in rr:
            lo, hi = to_us(r['tInject']), to_us(min(r['frames'].values()))
            i0, i1 = bisect.bisect_left(p['times'], lo), bisect.bisect_left(p['times'], hi)
            cov += sum(p['dts'][i0:i1])
            for sid, dt in zip(p['samples'][i0:i1], p['dts'][i0:i1]): selfc[fname(p['nodes'].get(sid, {}))] += dt
        print(f'\n### {nmk} (n={len(rr)}): phase C span mean {statistics.mean(spans):.2f} / median {statistics.median(spans):.2f} ms; UI worker profile covers {cov / 1000 / len(rr):.2f} ms/input')
        print('| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |'); print('|---|---|---|---|---|')
        for tid in sorted(evs, key=lambda x: (x != ui, x)):
            n_ = names.get((rpid, tid), '?')
            if n_ not in ('DedicatedWorker thread', 'CrRendererMain', 'Realtime AudioWorklet thread'): continue
            tsl = [a for a, _ in evs[tid]]; cpu = 0.0; wall = 0.0
            for r in rr:
                lo, hi = to_us(r['tInject']), to_us(min(r['frames'].values()))
                i0, i1 = bisect.bisect_left(tsl, lo), bisect.bisect_right(tsl, hi)
                if i1 - i0 >= 2:
                    wall += evs[tid][i1 - 1][0] - evs[tid][i0][0]; cpu += evs[tid][i1 - 1][1] - evs[tid][i0][1]
            role = 'UI worker' if tid == ui else ('Worker' if n_.startswith('Dedicated') else n_)
            print(f'| {n_} {tid} | {role} | {cpu / 1000 / len(rr):.1f} | {wall / 1000 / len(rr):.1f} | {cpu / wall if wall else 0:.2f} |')
        if nm:
            print(f'\n#### UI worker self top 14 in phase C with names from the name section (ms/input)')
            print('| function | ms/input |'); print('|---|---|')
            for k, v in selfc.most_common(14):
                if k.startswith('wasm-function['):
                    idx = int(k[14:-1]); k2 = f'{k} = {nm.get(idx, "(no name in the section)")}'
                else: k2 = k
                print(f'| {k2} | {v / 1000 / len(rr):.2f} |')
main()
