#!/usr/bin/env python3
"""pick1 (decision 0242): the per-operation drawing-path matrix (task A). Joins, for every operation of tools/ops-pick1.tsv:
 native N1 -O2 + FLE_R_FAST_COPY=1 (the shipping runtime flag) CPU ms (plain run), the wasm CPU ms (node, virtual clock) of the bundle named below, the API-call counts of the recorder run (same binary + recorder),
 the pixels written by the runtime's raster primitives, the windows created in the operation, and the module shares of the SIGPROF samples.
usage: mk-path-matrix.py PLAIN_RUN RECORDER_RUN WASM_STDOUT OUT.tsv"""
import re, sys, csv, collections, struct, os
plain, rec, wasm, out = sys.argv[1:5]
here = os.path.dirname(os.path.abspath(__file__))
ops = [l.rstrip('\n').split('\t') for l in open(here + '/ops-pick1.tsv', encoding='utf-8') if l.strip() and not l.startswith('#')]
desc = {o[1]: (o[2], o[3]) for o in ops}
def cpu(path):
    L = [(m.group(1), int(m.group(2)) + int(m.group(3)), int(m.group(4))) for ln in open(path, errors='replace') for m in [re.match(r'g3-cpu: (\S+) user_ms (\d+) sys_ms (\d+) clock_ms (\d+)', ln)] if m]
    d = {}; prev = None
    for n, c, t in L:
        if prev: d[n] = (c - prev[1], t - prev[2], prev[2])
        prev = (n, c, t)
    return d, L
pc, L = cpu(plain + '/stdout.txt')
order = [n for n, _, _ in L][1:]
wasm_ms = {}
for ln in open(wasm).read().splitlines():
    pass
wt = {}
prev = None
cur = None
for ln in open(wasm, errors='replace'):
    m = re.match(r'g3-snap: #\d+ (\S+) ', ln)
    if m: cur = m.group(1)
    m = re.match(r'parity1-maincpu: ([\d.]+) s', ln)
    if m and cur is not None:
        v = float(m.group(1)) * 1000
        if prev is not None: wt[cur] = v - prev
        prev = v
# API counts per op (A lines per dump)
blocks = []; cnt = None
ev = []
for ln in open(rec + '/api.log', errors='replace'):
    if ln.startswith('#dump'): cnt = collections.Counter(); blocks.append(cnt)
    elif ln.startswith('A ') and cnt is not None:
        t = ln.split(); cnt[t[1]] = int(t[2])
    else:
        m = re.match(r'E (\d+) (\S+) (.*)$', ln.rstrip('\n'))
        if m: ev.append((int(m.group(1)), m.group(2), m.group(3)))
rc, _ = cpu(rec + '/stdout.txt')
# pixels
px = collections.defaultdict(int)
d = open(rec + '/blit.bin', 'rb').read() if os.path.exists(rec + '/blit.bin') else b''
recs = [struct.unpack('<IIQ', d[i:i + 16]) for i in range(0, len(d) - 15, 16)]
# windows created per op
lo = {}
Lr = cpu(rec + '/stdout.txt')[1]
for k in range(1, len(Lr)): lo[Lr[k][0]] = (Lr[k - 1][2], Lr[k][2])
mods = {}
for ln in open(rec.replace('runs/', 'tables/').replace('/', '', 0) if False else 'tables/pk-module-share-native-fc.tsv'):
    t = ln.rstrip('\n').split('\t')
    if len(t) >= 3: mods[t[0]] = t[2]
rows = [['op', 'category', 'what', 'virtual_ms', 'N1fc_cpu_ms', 'N1fc_cpu_per_virtual_s', 'wasm_cpu_ms (node, bundle below)', 'wasm_cpu_per_virtual_s', 'api_calls', 'BitBlt', 'BeginPaint', 'CreateDIBSection', 'StretchBlt', 'AlphaBlend', 'GradientFill', 'UpdateLayeredWindow*', 'SetLayeredWindowAttributes', 'DwmExtendFrame', 'windows_created', 'Mpixels_written (raster primitives)', 'module shares (top)']]
for k, name in enumerate(order):
    if name not in desc or name == 'cal_snap': continue
    c = cnt = blocks[k + 1] if k + 1 < len(blocks) else collections.Counter()
    a, b = lo.get(name, (0, 0))
    pxs = sum(r[2] for r in recs if a < r[0] <= b)
    wc = [e for e in ev if a < e[0] <= b and e[1] == 'CreateWindowExW']
    cls = collections.Counter(re.search(r'class="([^"]*)"', e[2]).group(1) for e in wc if re.search(r'class="([^"]*)"', e[2]))
    v = pc[name][1]
    w = wt.get(name)
    rows.append([name, desc[name][0], desc[name][1], v, pc[name][0], '%.3f' % (pc[name][0] / v) if v else '', int(w) if w is not None else '', '%.3f' % (w / v) if (w is not None and v) else '',
                 sum(c.values()), c['BitBlt'], c['BeginPaint'], c['CreateDIBSection'], c['StretchBlt'], c['AlphaBlend'], c['GradientFill'], c['UpdateLayeredWindowIndirect'] + c['UpdateLayeredWindow'],
                 c['SetLayeredWindowAttributes'], c['DwmExtendFrameIntoClientArea'], ' '.join('%s x%d' % kv for kv in cls.items()) or '-', '%.1f' % (pxs / 1e6), mods.get(name, '')[:140]])
with open(out, 'w', newline='') as f:
    csv.writer(f, delimiter='\t').writerows(rows)
print(len(rows) - 1, 'ops')
