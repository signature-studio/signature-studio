#!/usr/bin/env python3
# pick1 (decision 0242): the "@@ N: step" rendering of win_act.c output that crates/fe-lift/tests/win2_act.rs pins (blocks() + render() in Python), for the lines of one scenario.
import sys
out = []; sc = 0; cur = None
for raw in open(sys.argv[1], encoding='utf-8').read().split('\n'):
    ln = raw.rstrip('\r')
    if ln.startswith('#### scenario '):
        sc = int(ln[len('#### scenario '):].split(':')[0].strip()); cur = None; continue
    if ln and not ln.startswith(' ') and not ln.startswith('=') and ln.endswith(':'):
        t = ln[:-1]
        if t in ('(cleanup)', '(setup)'): cur = None
        else: out.append(['%d: %s' % (sc, t), []]); cur = len(out) - 1
        continue
    if ln.startswith('  ?') or 'WM_NCHITTEST' in ln: continue
    if cur is not None: out[cur][1].append(ln)
for k, v in out:
    print('@@ ' + k)
    for l in v: print(l)
