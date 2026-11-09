#!/usr/bin/env python3
# beacon.py PERFLOG SKIP [tsv BROWSER VARIANT I START END DIR]: the last first-idle beacon written after line SKIP of the perf log (webglRenderer, desk.mode, first idle ms)
# without 'tsv': prints a line and exits 0 only if webglRenderer is set and desk.mode = gl (1: no beacon, 2: no WebGL / 2D)
# with 'tsv': prints one tab separated row: browser, variant, i, first idle ms (entry-check's final.idle, else the beacon's), start, end, webglRenderer, desk.mode
import json, sys, glob
path, skip = sys.argv[1], int(sys.argv[2])
last = None
try:
    for n, l in enumerate(open(path)):
        if n < skip: continue
        try: o = json.loads(l)
        except Exception: continue
        p = o.get('perf') or {}
        if p.get('reason') == 'first-idle': last = p
except FileNotFoundError:
    pass
w = ((last or {}).get('browser') or {}).get('webglRenderer'); m = ((last or {}).get('desk') or {}).get('mode')
if len(sys.argv) > 3 and sys.argv[3] == 'tsv':
    b, var, i, s, e, d = sys.argv[4:10]
    idle = None
    try: idle = json.load(open(glob.glob(d + '/entry-*.json')[0])).get('final', {}).get('idle')
    except Exception: pass
    if idle is None and last: idle = last['timeline']['idle']
    print('\t'.join(map(str, [b, var, i, round(idle) if idle is not None else None, 'start ' + s, 'end ' + e, w, m])))
    sys.exit(0)
if not last:
    print('preflight: NO BEACON'); sys.exit(1)
print('preflight: webglRenderer', w, 'desk.mode', m, 'first idle', round(last['timeline']['idle']))
# the present Worker P (decision 0311) owns the canvas and the page's beacon has no desk.mode then: a page whose engine says present D with a WebGL renderer is a WebGL run too
present = (((last or {}).get('engine') or {}).get('present'))
sys.exit(0 if w and (m == 'gl' or (m is None and present == 'D')) else 2)
