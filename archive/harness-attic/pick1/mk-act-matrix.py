#!/usr/bin/env python3
# pick1 (decision 0242 addendum): the activation / show matrix of harness/api2/win_act.c scenario 14, Wine 10.0 against the runtime, cell by cell.
# usage: mk-act-matrix.py WINE.blocks RUNTIME.blocks OUT.tsv      (the .blocks files: tools/mk-act-blocks.py of the output of `SCEN=14 build.sh act`)
import sys, re, collections
def rd(p):
    out = collections.OrderedDict(); k = None
    for l in open(p, encoding='utf-8').read().split('\n'):
        if l.startswith('@@ '): k = l[3:]; out[k] = []
        elif k is not None and l != '': out[k].append(l)
    return out
w = rd(sys.argv[1]); n = rd(sys.argv[2])
ACT = re.compile(r'WM_(ACTIVATE|NCACTIVATE|ACTIVATEAPP|SETFOCUS|KILLFOCUS|SHOWWINDOW|ENABLE|CAPTURECHANGED)\b')
def split(b):
    msgs = [x for x in b if x.startswith('  ')]; st = [x for x in b if x.startswith('= ')]
    z = [x for x in st if x.startswith('= z')]; ac = [x for x in st if x.startswith('= active')]; vi = [x for x in st if x.startswith('= vis')]
    return msgs, z, ac, vi
rows = []; cnt = collections.Counter(); ccnt = collections.Counter()
for k, wb in w.items():
    nb = n.get(k)
    st, call = k.split(': ', 1)[1].split(' | ', 1)
    if nb is None: rows.append((st, call, 'MISSING', '', '')); cnt['MISSING'] += 1; continue
    if wb == nb: rows.append((st, call, 'same', '', '')); cnt['same'] += 1; continue
    wm, wz, wa, wv = split(wb); nm, nz, na, nv = split(nb)
    cl = []; det = ''
    if wa != na: cl.append('ACTIVE/FOCUS'); det = 'wine %s | runtime %s' % (wa[0][2:] if wa else '', na[0][2:] if na else '')
    if wv != nv: cl.append('VISIBILITY'); det = det or 'wine %s | runtime %s' % (wv[0][2:], nv[0][2:])
    wact = [x for x in wm if ACT.search(x)]; nact = [x for x in nm if ACT.search(x)]
    if wact != nact: cl.append('ACTIVATION-MSGS'); det = det or 'wine %s | runtime %s' % ('; '.join(x.strip() for x in wact)[:110], '; '.join(x.strip() for x in nact)[:110])
    wpos = [x for x in wm if 'WINDOWPOS' in x]; npos = [x for x in nm if 'WINDOWPOS' in x]
    if wpos != npos: cl.append('WINDOWPOS-MSGS'); det = det or 'wine %s | runtime %s' % ('; '.join(x.strip().replace('WM_WINDOWPOS', '') for x in wpos)[:110], '; '.join(x.strip().replace('WM_WINDOWPOS', '') for x in npos)[:110])
    if wz != nz: cl.append('Z-ORDER'); det = det or 'wine %s | runtime %s' % (wz[0][2:] if wz else '', nz[0][2:] if nz else '')
    other_w = [x for x in wm if not ACT.search(x) and 'WINDOWPOS' not in x]; other_n = [x for x in nm if not ACT.search(x) and 'WINDOWPOS' not in x]
    if other_w != other_n: cl.append('OTHER-MSGS'); det = det or 'wine %s | runtime %s' % ('; '.join(x.strip() for x in other_w)[:110], '; '.join(x.strip() for x in other_n)[:110])
    key = '+'.join(cl) or 'DIFF'
    cat = ''
    if 'ACTIVE/FOCUS' in cl:
        mm = st.startswith(('S5', 'S6')) or any(c in call for c in ('SW_SHOWMINIMIZED', 'SW_MINIMIZE', 'SW_SHOWMINNOACTIVE', 'SW_SHOWMAXIMIZED'))
        if mm: cat = 'M: minimize / maximize state (not examined)'
        elif st.startswith(('S7', 'S8', 'S9')): cat = 'E: WS_EX_NOACTIVATE / disabled: the runtime follows Learn (known class E of win-act-known-divergence.txt)'
        elif st.startswith(('S0', 'S1')): cat = 'V: hidden T: Wine activates it; the runtime activates visible windows only (decision 0222 reading)'
        else: cat = 'OTHER (to examine)'
    rows.append((st, call, key, det, cat)); cnt[key] += 1; ccnt[cat] += 1
with open(sys.argv[3], 'w') as o:
    o.write('# pick1 decision 0242 addendum: harness/api2/win_act.c scenario 14, the activation / show matrix: Wine 10.0 (headless, run on this machine) against the runtime window layer, one row per cell (state of T x call on T). class = what differs (ACTIVE/FOCUS = the active / foreground / focus window after the call; VISIBILITY; ACTIVATION-MSGS = WM_ACTIVATE / NCACTIVATE / ACTIVATEAPP / SETFOCUS / KILLFOCUS / SHOWWINDOW / ENABLE / CAPTURECHANGED; WINDOWPOS-MSGS; Z-ORDER; OTHER-MSGS)\n')
    o.write('state of T\tcall\tclass\tdetail (first difference)\tcategory of an ACTIVE/FOCUS difference\n')
    for r in rows: o.write('\t'.join(r) + '\n')
tot = len(rows)
print('cells', tot)
for k, v in sorted(cnt.items(), key=lambda x: -x[1]): print('%5d  %s' % (v, k))

print('ACTIVE/FOCUS differences by category:')
for k, v in sorted(ccnt.items(), key=lambda x: -x[1]):
    if k: print('%5d  %s' % (v, k))
