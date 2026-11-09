#!/usr/bin/env python3
# pick1 (decision 0242 addendum): the activation / focus / show sequence of the second pop-up ("More plugins...") out of a gdb event log (gdbcr/cr.gdb: API = the guest's call, M = a message the runtime sent, ACT = do_activate)
# usage: mk-act-seq.py EV.txt HANDLE LABEL > table.tsv      HANDLE = the pop-up's handle (0x7f990228)
import sys, re
ev = [l.rstrip('\n') for l in open(sys.argv[1], errors='replace')]
h = sys.argv[2]; label = sys.argv[3]
i0 = max(i for i, l in enumerate(ev) if l.startswith('API ShowWindow h=%s cmd=4' % h))
for k in range(i0, 0, -1):
    if ev[k].startswith('API GetActiveWindow'): i0 = k; break
i1 = next((i for i, l in enumerate(ev) if i > i0 and l.startswith('API DestroyWindow h=%s' % h)), len(ev) - 1)
names = {'0x7f990038': 'main', h: 'popup', '0x7f990044': 'main child', '0x7f990180': 'TVectorPanel', '0x7f990238': 'list form', '0x7f99022c': 'list form (first, owner main)'}
msgs = {'0x86': 'WM_NCACTIVATE', '0x6': 'WM_ACTIVATE', '0x1c': 'WM_ACTIVATEAPP', '0x21': 'WM_MOUSEACTIVATE', '0x7': 'WM_SETFOCUS', '0x8': 'WM_KILLFOCUS', '0xa': 'WM_ENABLE', '0x18': 'WM_SHOWWINDOW', '0x2': 'WM_DESTROY', '0x82': 'WM_NCDESTROY', '0x10': 'WM_CLOSE', '0x1f': 'WM_CANCELMODE', '0x215': 'WM_CAPTURECHANGED'}
print('# pick1 decision 0242 addendum: %s. Real engine (native N1, G3 simulated input). API = call of the guest (runtime import); M = message the runtime sent as a result; ACT = the runtime activates a window (do_activate). Windows: %s' % (label, ', '.join('%s=%s' % (k, v) for k, v in names.items())))
print('n\tkind\twhat\twindow\tdetail')
n = 0
def nm(x): return names.get(x, x)
for l in ev[i0:i1 + 1]:
    f = l.split()
    if f[0] == 'M' and ('msg=0x46' in l or 'msg=0x47' in l or 'msg=0x24' in l): continue
    n += 1
    if f[0] == 'API':
        hh = re.search(r'h=(0x[0-9a-f]+)', l)
        print('%d\tAPI\t%s\t%s\t%s' % (n, f[1], nm(hh.group(1)) if hh else '', ' '.join(x for x in f[2:] if not x.startswith('h='))))
    elif f[0] == 'M':
        hh = re.search(r'h=(0x[0-9a-f]+)', l); m = re.search(r'msg=(0x[0-9a-f]+)', l); wp = re.search(r'wp=(0x[0-9a-f]+|0)', l); lp = re.search(r'lp=(0x[0-9a-f]+|0)', l)
        print('%d\tM\t%s (in %s)\t%s\twp=%s lp=%s' % (n, msgs.get(m.group(1), m.group(1)), f[1], nm(hh.group(1)), nm(wp.group(1)) if wp else '', nm(lp.group(1)) if lp else ''))
    elif f[0] == 'ACT':
        hh = re.search(r'h=(0x[0-9a-f]+)', l); print('%d\tACT\tdo_activate (from %s)\t%s\t%s' % (n, f[1], nm(hh.group(1)), f[-1]))
    elif f[0] == 'SKIPACT': print('%d\tEXPERIMENT\tactivation skipped\t%s\t' % (n, nm(re.search(r'h=(0x[0-9a-f]+)', l).group(1))))
    else: print('%d\t%s\t\t\t' % (n, f[0]))
