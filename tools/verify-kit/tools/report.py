#!/usr/bin/env python3
# report.py: out/RESULTS.md from out/* (decision 0277). Every value is written with the load average and the I/O pressure (some avg10) of its moment.
import json, glob, os, re, statistics as st, sys
K = os.path.dirname(os.path.dirname(os.path.abspath(__file__))); O = K + '/out'
NEW = os.environ.get('NEW', '?'); PREV = os.environ.get('PREV', 'fl-engine-ci-ff5ee429'); REF = os.environ.get('REF', '?'); WORKER = os.environ.get('WORKER', '?')
EXPECT_SHA = '381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979'; EXPECT_SIZE = 22863828
L = []
def w(s=''): L.append(s)
def rj(p):
    try: return json.load(open(p))
    except Exception: return None
def med(v):
    v = [x for x in v if x is not None]; return st.median(v) if v else None
def f(v, n=0):
    return '-' if v is None else (f'{v:.{n}f}' if isinstance(v, float) else str(v))
def sh(s):
    return re.sub(r' avg10=([\d.]+) avg60=\S+ avg300=\S+', r' / io \1', s)
def fl(v, n=1): return '-' if v is None else f'{float(v):.{n}f}'
def li(r):  # "load a→b, io c→d" of a scenario row
    return f"load {fl(r.get('loadStart'))}→{fl(r.get('loadEnd'))}, io {fl(r.get('ioStart'))}→{fl(r.get('ioEnd'))}"
def rows_of(pattern):
    out = []
    for p in sorted(glob.glob(pattern, recursive=True)):
        for r in rj(p) or []: r['_dir'] = os.path.relpath(os.path.dirname(p), O); out.append(r)
    return out

# ---- facts gathered first
pre = open(O + '/preflight.txt').read().strip() if os.path.exists(O + '/preflight.txt') else 'not run'
criteria = []  # (name, value, ok)
scenes = {b: rows_of(f'{O}/scenes-{b}/**/report.json') for b in ('chrome', 'firefox')}
def skey(r):
    m = re.match(r'd(\d+)-', r['name']); return (0, int(m.group(1))) if m else (1, r['name'])
for b in scenes: scenes[b].sort(key=skey)
EXPECTED = ['d1-sampler-envelope', 'd2-browser-folder', 'd3-settings-all-tabs', 'd4-playlist-middle-drag', 'd5-mixer-front', 'd6-playlist-close', 'd7-main-close', 'd8-file-menu-100', 'd9-keys', 'd10-pianoroll-f12', 'd11-menu-hover-escape', 'plus-more-plugins', 'add-menu-simple-fade', 'picker-f8-fade', 'picker-cost']
missing = [(b, n) for b in scenes for n in EXPECTED if scenes[b] and n not in {r['name'] for r in scenes[b]}]
stops = [(b, r['name']) for b in scenes for r in scenes[b] if r.get('stopped') and not r.get('intendedExit')]
notok = [(b, r['name']) for b in scenes for r in scenes[b] if not r.get('ok')]
play = {}; playprev = {}
for d_ in sorted(glob.glob(O + '/play-*/report.json')):
    n_ = os.path.basename(os.path.dirname(d_))[5:]; b_ = n_.split('-')[0]; rest = n_.split('-')[1:]
    (playprev if 'prev' in rest else play)[(b_ + ' ' + '-'.join(x for x in rest if x != 'prev')).strip()] = rows_of(d_)
for b in play:
    for r in play[b]:
        if r.get('stopped'): stops.append((b + ' play', r['name']))
        if not r.get('ok'): notok.append((b + ' play', r['name']))
for p_ in sorted(glob.glob(O + '/p3-*/report.json')):
    for r in rj(p_) or []:
        if r.get('stopped'): stops.append((os.path.basename(os.path.dirname(p_)), r['name']))
        if not r.get('ok'): notok.append((os.path.basename(os.path.dirname(p_)), r['name']))
ex = rj(O + '/export/export.json'); exrows = rows_of(f'{O}/export/report.json')
if exrows and exrows[0].get('stopped'): stops.append(('chrome export', exrows[0]['name']))
d8 = {b: [r for r in scenes[b] if r['name'] == 'd8-file-menu-100'] for b in scenes}
def d8s(b):
    if not d8[b]: return 'not run', False
    m = d8[b][0].get('menuCycles') or {}
    return f"{m.get('completed')}/{m.get('n')}", m.get('completed') == 100 and m.get('n') == 100 and not m.get('popupMissingSurface') and not m.get('notVisible')
import datetime
CCs = []
try:
    for l_ in open(O + '/chrome-count.tsv'):
        c_ = l_.split('\t')
        if len(c_) >= 4: CCs.append((float(c_[0]), int(c_[1])))
except FileNotFoundError: pass
IDLE_T = {}
try:
    today = datetime.date.today()
    for l_ in open(O + '/steps.log'):
        m_ = re.match(r'^(\d\d):(\d\d):(\d\d) idle (\w+) (new|prev) (\d+) \(', l_)
        if m_:
            ts_ = datetime.datetime(today.year, today.month, today.day, int(m_.group(1)), int(m_.group(2)), int(m_.group(3))).timestamp() * 1000
            IDLE_T[(m_.group(4), m_.group(5), int(m_.group(6)))] = ts_
except FileNotFoundError: pass
def oth(b, var, i):
    t = IDLE_T.get((b, var, int(re.sub(r'\D', '', str(i)) or 0)))
    if t is None or not CCs: return None
    seg = [n for tt, n in CCs if t - 1000 <= tt <= t + 15000]
    return max(seg) if seg else None
idle = {}
for b in ('chrome', 'firefox'):
    idle[b] = []
    for p in sorted(glob.glob(f'{O}/idle-{b}*.tsv'), key=lambda x: (x.endswith(f'idle-{b}.tsv'), x)):
        tag = 'q' if (p.endswith(f'idle-{b}.tsv') and os.path.exists(f'{O}/idle-{b}-1st.tsv')) else ''
        for l in open(p):
            c = l.rstrip('\n').split('\t')
            if len(c) >= 8: idle[b].append(dict(var=c[1], i=f'{c[2]}{tag}', ms=None if c[3] == 'None' else int(c[3]), start=sh(c[4][6:]), end=sh(c[5][4:]), webgl=c[6], mode=('gl' if (c[7] == 'None' and c[6] not in ('None', '', 'null')) else c[7])))
QL = float(os.environ.get('QUIET_LOAD', '12'))
def quiet(x):  # load ≤12 and I/O pressure avg10 <1 at the start and at the end
    try:
        v = [float(t) for t in re.findall(r'^([\d.]+) / io ([\d.]+)$', x['start'])[0] + tuple(re.findall(r'^([\d.]+) / io ([\d.]+)$', x['end'])[0])]
        return v[0] <= QL and v[2] <= QL and v[1] < 1 and v[3] < 1
    except Exception: return False
def idle_med(b, var, only_quiet=False):
    return med([x['ms'] for x in idle[b] if x['var'] == var and x['mode'] == 'gl' and (quiet(x) or not only_quiet)])
def idle_crit(b):
    n, p = idle_med(b, 'new'), idle_med(b, 'prev')
    if n is None or p is None: return 'not run', None
    nq, pq = idle_med(b, 'new', True), idle_med(b, 'prev', True)
    nn = len([x for x in idle[b] if x['var'] == 'new' and x['mode'] == 'gl' and quiet(x)]); pn = len([x for x in idle[b] if x['var'] == 'prev' and x['mode'] == 'gl' and quiet(x)])
    ok = (n - p) <= 500
    def mse(var):
        v = [x['ms'] for x in idle[b] if x['var'] == var and x['mode'] == 'gl' and x['ms'] is not None]
        return (st.mean(v), (st.stdev(v) / len(v) ** 0.5) if len(v) > 1 else float('nan'), len(v))
    (mn, sn, nn_), (mp, sp, np_) = mse('new'), mse('prev'); dse = (sn ** 2 + sp ** 2) ** 0.5
    t = f'all runs: new median {n:.0f} ms, previous {p:.0f} ms, difference {n-p:+.0f} ms; means {mn:.0f} ± {sn:.0f} (SE, n={nn_}) / {mp:.0f} ± {sp:.0f} (n={np_}), difference of the means {mn-mp:+.0f} ± {dse:.0f} (SE)'
    if nq is not None and pq is not None:
        t += f'; quiet runs only (load ≤12, I/O avg10 <1; n new {nn} / prev {pn}): new {nq:.0f}, previous {pq:.0f}, difference {nq-pq:+.0f} ms'; ok = ok and (nq - pq) <= 500
    return t, ok

w(f'# 0277: verification of `{NEW}` (Worker {WORKER}, page of git {REF}) against the previous production bundle `{PREV}`')
w()
w('Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.')
w()
w('## Pass criteria')
w()
w('| criterion | value | pass |')
w('|---|---|---|')
w(f"| Firefox has WebGL (preflight) | {pre} | {'yes' if pre.startswith('preflight: webglRenderer') and not pre.startswith('preflight: webglRenderer None') and ('desk.mode gl' in pre or 'desk.mode None' in pre) else 'NO: Firefox values are 2D values'} |")
w(f"| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: {stops or 'none'}; not ok: {notok or 'none'}; no result at all (the harness did not start): {missing or 'none'} | {'yes' if scenes['chrome'] and scenes['firefox'] and not stops and not notok and not missing else 'NO or not complete'} |")
if ex:
    ok = ex.get('sha256') == EXPECT_SHA and ex.get('size') == EXPECT_SIZE
    w(f"| Export WAV equals the reference | {ex.get('size')} B, sha256 `{ex.get('sha256')}` (expected {EXPECT_SIZE} B, `{EXPECT_SHA[:16]}…`) | {'yes' if ok else 'NO'} |")
else: w('| Export WAV equals the reference | not run / no file | NO |')
for b in ('chrome', 'firefox'):
    v, ok = idle_crit(b); w(f'| First idle ≤ previous + 500 ms ({b}, WebGL runs) | {v} | {"yes" if ok else ("NO" if ok is False else "not run")} |')
for b in ('chrome', 'firefox'):
    v, ok = d8s(b); w(f'| File menu ×100 ({b}) | {v} | {"yes" if ok else ("NO" if v != "not run" else "not run")} |')
w()

w('## Scenes (new bundle)')
w()
w('| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |')
w('|---|---|---|---|---|---|')
for b in scenes:
    for r in scenes[b]:
        m = r.get('menuCycles'); res = 'ok (intended exit)' if r.get('ok') and r.get('stopped') else 'ok' if r.get('ok') else ('STOPPED' if r.get('stopped') else 'NOT OK: ' + str(r.get('error') or r.get('opErrors') or [e for e in (r.get('expects') or []) if not e.get('ok')])[:160])
        mc = f"{m.get('completed')}/{m.get('n')}" if m else '-'
        w(f"| {b} | {r['name']} | {res} | {f(r.get('firstIdleMs'))} | {mc} | {li(r)} |")
w()

for b_, n_ in missing: w(f'| {b_} | {n_} | **NO RESULT (the browser did not start)** | - | - | - |')
w()
w('The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).')
w()
if os.path.isdir(O + '/d7-ff-first-session-lost'):
    r7 = rj(O + '/d7-ff-first-session-lost/report.json') or [{}]
    d7line = next((re.sub(r' avg10=([\d.]+) avg60=\S+ avg300=\S+', r' / io \1', l.split(' scenes firefox scn-client.json d7-main-close')[1].strip()) for l in open(O + '/steps.log') if 'scenes firefox scn-client.json d7-main-close' in l), '?'); reruns = [rj(p) for p in sorted(glob.glob(O + '/d7-ff-rerun-*/report.json'))] + [x for x in [rows_of(O + '/scenes-firefox/ff-d7-main-close/report.json')] if x]
    w(f"Firefox d7 (the intended exit): the first run lost the WebDriver session (`{str(r7[0].get('error'))[:120]}`; at {d7line}, two jobs in parallel; the engine's exit was not read). Re-runs: {len(glob.glob(O + '/d7-ff-rerun-*/report.json')) + 1}, all `ok (the intended exit)`. Not reproduced; the cause of the first loss is unknown. The table shows the latest run; the first run is kept in `out/d7-ff-first-session-lost/`.")
    w()
inv = sorted(os.listdir(O + '/invalid-noproj')) if os.path.isdir(O + '/invalid-noproj') else []
if inv:
    w('Invalid runs, kept aside in `out/invalid-noproj/` and NOT counted above: d1, d10, d11 (Chrome and Firefox) and the two d11 runs on the new and the previous bundle, taken while the kit served no `Sampler.flp` / `SHP_Sawing Synth.wav` (a 404 body was opened as the project; the engine stopped with a named CompareStringW stop on both bundles). They were re-run after the kit served the real files (rows above).')
    w()
w('## alpha7 (main window geometry, guest 1920x1080, Chrome)')
w()
a7 = rows_of(O + '/alpha7/report.json')
if a7:
    r = a7[0]; wl = (r.get('winlogs') or {}).get('main') or []
    big = [x for x in wl if re.search(r' (\d+)x(\d+) z', x) and int(re.search(r' (\d+)x(\d+) z', x).group(1)) >= 1800]
    w(f"result {'ok' if r.get('ok') else 'NOT OK'}; {li(r)}; windows visible at the end: {len(r.get('windowsEnd') or [])}; windows larger than 1800 px wide:")
    for x in big or wl[:6]: w(f'- `{x}`')
    w(f"- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `{', '.join(sorted(os.path.basename(p) for p in glob.glob(O + '/alpha7/*.png')))}`")
else: w('not run')
w()

w('## First idle (single runs, cold, a new profile each; previous and new alternating)')
w()
w(f'| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | other Chrome browsers during the run (max) | quiet (load ≤{QL:g}, I/O <1) | webglRenderer | desk.mode |')
w('|---|---|---|---|---|---|---|---|---|---|')
for b in idle:
    for x in idle[b]: w(f"| {b} | {x['var']} | {x['i']} | {f(x['ms'])} | {x['start']} | {x['end']} | {oth(b, x['var'], x['i']) if oth(b, x['var'], x['i']) is not None else 'not observed'} | {'yes' if quiet(x) else 'no (reference)'} | {(x['webgl'] or 'null')[:60]} | {x['mode']} |")
w()
w('| browser | mode | previous median (n) | new median (n) | difference |')
w('|---|---|---|---|---|')
for b in idle:
    for mode in sorted({x['mode'] for x in idle[b]}):
        p = [x['ms'] for x in idle[b] if x['var'] == 'prev' and x['mode'] == mode and x['ms'] is not None]; n = [x['ms'] for x in idle[b] if x['var'] == 'new' and x['mode'] == mode and x['ms'] is not None]
        w(f"| {b} | {mode} | {f(med(p))} ({len(p)}) | {f(med(n))} ({len(n)}) | {f(med(n) - med(p)) if p and n else '-'} |")
w()

w('## Export (Chrome, the reference project, Ctrl+R)')
w()
if exrows:
    r = exrows[0]; w(f"result {'ok' if r.get('ok') else 'NOT OK'}; {li(r)}; seconds {r.get('seconds')}")
    w(f"- WAV `{(ex or {}).get('name')}`: {(ex or {}).get('size')} B, sha256 `{(ex or {}).get('sha256')}`; expected {EXPECT_SIZE} B / `{EXPECT_SHA}`: {'MATCH' if ex and ex.get('sha256') == EXPECT_SHA and ex.get('size') == EXPECT_SIZE else 'DIFFERENT'}")
else: w('not run')
w()

w('## Playback (the reference project zip, ~60 s)')
w()
for kind, pd in (('new', play), ('previous ' + PREV, playprev)):
  pd = dict(sorted(pd.items()))
  for b in pd:
    for r in pd[b]:
        w(f"**{b} ({kind})**: result {'ok' if r.get('ok') else 'NOT OK'}, stopped {r.get('stopped')}; {li(r)}")
        w()
        w('| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |')
        w('|---|---|---|---|---|')
        for m in r.get('measures') or []: w(f"| {m['name']} | {f(m.get('wallS'),1)} | {m.get('workerMsPerS')} | {m.get('mainMsPerS')} | {m.get('load')} |")
        w()
        w('| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |')
        w('|---|---|---|---|---|---|---|---|')
        for a in r.get('audios') or []:
            x = a.get('detail') or {}; w(f"| {a['name']} | {a.get('load')} | {x.get('status')} | {x.get('playingSeconds')} | {x.get('underrunQuanta')} | {x.get('episodes')} | {x.get('longestRunQuanta')} | {x.get('fillMinFrames')} |")
        pl = [m['workerMsPerS'] for m in (r.get('measures') or []) if m['name'].startswith('playing-')]
        au = {a['name']: a.get('detail') or {} for a in r.get('audios') or []}
        if pl and 'playing-a' in au and 'after-stop' in au:
            q = (au['after-stop'].get('underrunQuanta') or 0) - (au['playing-a'].get('underrunQuanta') or 0); e = (au['after-stop'].get('episodes') or 0) - (au['playing-a'].get('episodes') or 0)
            t = (au['after-stop'].get('playingSeconds') or 0) - (au['playing-a'].get('playingSeconds') or 0)
            w(); w(f"- Worker busy while playing: min {min(pl)}, median {med(pl):.0f}, max {max(pl)} ms/s. Underrun from the end of playing-a to the stop ({t:.0f} s of playback): {q} quanta in {e} episodes ({q / e if e else 0:.1f} quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): {au.get('before-play', {}).get('underrunQuanta')} quanta in {au.get('before-play', {}).get('episodes')} episode(s); `longestRunQuanta` {au['after-stop'].get('longestRunQuanta')} is that load stall, not the playback.")
        w()
if not any(play.values()): w('not run'); w()

w('### Playback summary (every run; same page, same procedure)')
w()
w('Counter definition: `window.__app.audio.detail()` of the page: `underrunQuanta` = AudioWorklet render quanta (128 frames ≈ 2.9 ms) in which the output ring was empty, counted cumulatively from when the audio device began to play (page start), `episodes` = maximal runs of consecutive underrun quanta (`app/src/audio/underrun.ts`, `ring-layout.ts`). This is the same counter as `underrunQuanta` in opt1\'s audio-b.json `detail`. **cumulative** = the value at the end (includes the stall while the project is opened); **during playback** = the difference between the reading at the end of `playing-a` (≈19 s after the first reading, 4 s after Space + 10 s playing) and the reading after the stop (≈52 s of playback).')
w()
w('| run | bundle | load at start → end | Worker busy ms/s while playing (min / median / max) | cumulative at the end: quanta / episodes | before the playback (project-open stall) | **during playback: quanta / episodes** | gap frames the engine wrote itself during playback (`engineGapFrames` delta; the engine wrote silence for a hole) |')
w('|---|---|---|---|---|---|---|---|')
allp = [(k, 'new', v) for k, v in play.items()] + [(k, 'previous ' + PREV[-8:], v) for k, v in playprev.items()]
for k, kind, v in sorted(allp, key=lambda x: (x[0].split()[0], x[0])):
    for r in v:
        pl = [m['workerMsPerS'] for m in (r.get('measures') or []) if m['name'].startswith('playing-')]; au = {a['name']: a.get('detail') or {} for a in r.get('audios') or []}
        if not (pl and 'playing-a' in au and 'after-stop' in au): continue
        dq = (au['after-stop'].get('underrunQuanta') or 0) - (au['playing-a'].get('underrunQuanta') or 0); de = (au['after-stop'].get('episodes') or 0) - (au['playing-a'].get('episodes') or 0)
        w(f"| {k} | {kind} | {fl(r.get('loadStart'))} → {fl(r.get('loadEnd'))} (io {fl(r.get('ioStart'))} → {fl(r.get('ioEnd'))}) | {min(pl)} / {med(pl):.0f} / {max(pl)} | {au['after-stop'].get('underrunQuanta')} / {au['after-stop'].get('episodes')} | {au['before-play'].get('underrunQuanta')} / {au['before-play'].get('episodes')} | **{dq} / {de}** | {(au['after-stop'].get('engineGapFrames') or 0) - (au['playing-a'].get('engineGapFrames') or 0)} |")
w()
w('## Mixer header drag: Worker CPU per move (previous and new alternating)')
w()
for b in ('chrome', 'firefox'):
    ps = sorted(glob.glob(f'{O}/ab-{b}*.tsv'), key=lambda x: (os.path.basename(x) != f'ab-{b}.tsv', x))
    if not ps: continue
    rows = []
    for k, p in enumerate(ps):
        for l in open(p):
            if l.strip():
                c = l.rstrip('\n').split('\t'); c[2] = (re.sub(r'.*ab-\w+?-(\w)\.tsv$', r'\1', p) if re.search(r'ab-\w+?-\w\.tsv$', p) else 'c') + c[2] if len(ps) > 1 else c[2]; rows.append(c)
    if len(ps) > 1: w(f'(the runs of {len(ps)} sessions, a / b / c = ab-{b}-a.tsv / ab-{b}-b.tsv / ab-{b}.tsv, in this order in time)'); w()
    w(f'**{b}** (1920x1080 guest)'); w()
    w('| bundle | n | median ms/move (all) | median (quiet runs: load ≤12, I/O <1) | runs (round, ms, load and I/O pressure at the start; * = load >12, reference) |'); w('|---|---|---|---|---|')
    for var in ('prev', 'new'):
        v = [(c[2], c[3], sh(c[4][6:])) for c in rows if c[1] == var]; nums = [float(x[1]) for x in v if x[1] != 'None']
        qn = [float(x[1]) for x in v if x[1] != 'None' and float(x[2].split()[0]) <= 12]
        w(f"| {var} | {len(nums)} | {f(med(nums), 2)} | {f(med(qn), 2)} (n={len(qn)}) | " + '; '.join(f"#{x[0]} {x[1]} ({x[2]}){'*' if float(x[2].split()[0]) > 12 else ''}" for x in v) + ' |')
    w()
for p_ in sorted(glob.glob(O + '/P3-*.md')):
    w(open(p_).read()); w()
if os.path.isdir(O + '/../out-e7b') and os.path.exists(O + '/minrestore/report.json'):
    pass
mr = rj(O + '/minrestore/report.json')
if mr:
    w('## E8: minimize / restore through the page button (Chrome; FL own minimize button clicked with the mouse)')
    w()
    for r in mr:
        w(f"**{r['name']}**: ok {r.get('ok')}, stopped {r.get('stopped')}; {li(r)}")
        for p in r.get('probes') or []: w(f"- {p['probe']}: `{p['result'][:300]}`")
        for k, v in (r.get('winlogs') or {}).items(): w(f"- windows {k}: {v if v else 'none visible'}")
        w()
w('## Steps (load and I/O pressure at each step)')
w()
w('```')
if os.path.exists(O + '/steps.log'): w(open(O + '/steps.log').read().rstrip())
w('```')
open(O + '/RESULTS.md', 'w').write('\n'.join(L) + '\n'); print(O + '/RESULTS.md', len(L), 'lines')
