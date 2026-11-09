#!/usr/bin/env python3
"""tp-analyze3.py RUNDIR TAG (ci1, 2026-11-08, PM 05:45): third pass of the G2 playback-T split.
 (1) '(program)' samples of the UI worker in phase C (tInject .. firstFrame): the nearest named caller above '(program)' (what the thread was in when V8 saw native / blocked time)
 (2) the same for the other Workers over the 400 ms window
 (3) self-time top 30 of the UI worker in phase C (ms per input) for File click and Escape, and the sum of the 'gil / wait / poll / idle' named functions
Output markdown on stdout."""
import sys, json, collections, bisect

sys.path.insert(0, __file__.rsplit('/', 1)[0])
exec(open(__file__.rsplit('/', 1)[0] + '/tp-analyze2.py').read().split('def main():')[0])

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
    print(f'## {tag}: UI worker tid {ui}')
    for kind, nmk in ((2, 'File click'), (5, 'Escape')):
        rr = [r for r in recs if r['kind'] == kind]
        if not rr: continue
        for pid, p in sorted(prof.items(), key=lambda kv: (kv[1]['tid'] != ui, kv[1]['tid'])):
            nm = names.get((rpid, p['tid']), '?')
            if nm != 'DedicatedWorker thread': continue
            role = 'UI worker' if p['tid'] == ui else 'Worker'
            if role == 'Worker' and kind != 2: continue
            cache = {}
            def stack(nid):
                if nid in cache: return cache[nid]
                s = []; x = nid; k = 0
                while x is not None and k < 400:
                    s.append(fname(p['nodes'].get(x, {}))); x = p['parent'].get(x); k += 1
                cache[nid] = s; return s
            callers = collections.Counter(); selfc = collections.Counter(); tot = 0; prog = 0
            for r in rr:
                if role == 'UI worker': lo, hi = to_us(r['tInject']), to_us(min(r['frames'].values()))
                else: lo, hi = to_us(r['tPage'] - 200), to_us(r['tPage'] + 200)
                i0, i1 = bisect.bisect_left(p['times'], lo), bisect.bisect_left(p['times'], hi)
                for sid, dt in zip(p['samples'][i0:i1], p['dts'][i0:i1]):
                    st = stack(sid); tot += dt; selfc[st[0]] += dt
                    if st[0] == '(program)':
                        prog += dt
                        c = next((x for x in st[1:] if x not in ('(root)', '(program)') and not x.startswith('wasm-function')), '(unnamed wasm only)')
                        callers[c] += dt
            n = len(rr)
            ph = 'phase C' if role == 'UI worker' else 'window W (400 ms)'
            print(f'\n### {nmk}: {role} {p["tid"]} {ph}: total {tot / 1000 / n:.1f} ms/input, (program) {prog / 1000 / n:.1f} ms/input; nearest named caller of (program):')
            print('| caller | ms/input |'); print('|---|---|')
            for k, v in callers.most_common(8): print(f'| {k} | {v / 1000 / n:.2f} |')
            if role == 'UI worker':
                print(f'\n(self top 30, ms/input): ' + '; '.join(f'{k} {v / 1000 / n:.2f}' for k, v in selfc.most_common(30)))
                grp = collections.Counter()
                for k, v in selfc.items():
                    kl = k.lower()
                    if 'gil' in kl: grp['gil*'] += v
                    elif 'poll' in kl or 'wait' in kl or 'idle' in kl: grp['poll/wait/idle*'] += v
                    elif k == 'g9_hook' or 'hyb' in kl: grp['g9_hook / fle_hyb*'] += v
                    elif 'compose' in kl or 'desk' in kl or 'raster' in kl or 'putimage' in kl.replace('_', ''): grp['compose/desk/raster/putImageData'] += v
                print('named groups ms/input: ' + '; '.join(f'{k} {v / 1000 / n:.2f}' for k, v in grp.most_common()))
main()
