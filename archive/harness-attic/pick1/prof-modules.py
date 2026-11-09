#!/usr/bin/env python3
"""pick1 (decision 0242): module shares of the SIGPROF samples of a native run (parity_prof.c: 16-byte records pc, virtual ms; the first record is the address of parity_prof_anchor).
Each sample's PC is attributed to the object file whose .text input section of the link map contains it, then to a module class:
  FLEngine (fle-NN.o = the lifted FLEngine), dsp (m1-*), Blend2D lifted (m2-*), QFC (m3-*), FreeType (m4-*), ILTools (m5-*), common.o,
  Blend2D upstream (libblend2d.a, b2d_swap.o), runtime GDI/raster (fle_gdi fle_gdc fle_raster fle_gdi*), runtime windows/messages (fle_win fle_input fle_desk fle_layered fle_class fle_cursor ...), runtime other (fle_*), harness (main_two fruity* a4 image trace), recorder (apiw_* pix_wrap parity_prof), libc/libm/libstdc++.
usage: prof-modules.py PROF.bin BINARY MAP [--ranges FROM:TO,FROM:TO,...] [--labels l1,l2,...]  (ranges are virtual ms; one line per range)"""
import sys, re, struct, bisect, subprocess, collections
a = sys.argv[1:]; prof, binary, mapf = a[0], a[1], a[2]
ranges = []; labels = []
if '--ranges' in a: ranges = [tuple(int(v) for v in r.split(':')) for r in a[a.index('--ranges') + 1].split(',')]
if '--labels' in a: labels = a[a.index('--labels') + 1].split(',')
sec = []
lines = open(mapf, errors='replace').read().splitlines()
i = 0
while i < len(lines):
    m = re.match(r'^ (\.text\S*)\s*$', lines[i]) or re.match(r'^ (\.text\S*)\s+(0x[0-9a-f]+)\s+(0x[0-9a-f]+)\s+(\S.*)$', lines[i])
    if m:
        if m.lastindex == 1 and i + 1 < len(lines):   # name on its own line, values on the next
            n = re.match(r'^\s+(0x[0-9a-f]+)\s+(0x[0-9a-f]+)\s+(\S.*)$', lines[i + 1])
            if n: sec.append((int(n.group(1), 16), int(n.group(2), 16), n.group(3).strip())); i += 1
        elif m.lastindex == 4:
            sec.append((int(m.group(2), 16), int(m.group(3), 16), m.group(4).strip()))
    i += 1
sec = [s for s in sec if s[1] > 0]; sec.sort()
starts = [s[0] for s in sec]
def klass(path):
    p = path.split('(')[0]; b = p.rsplit('/', 1)[-1]; arch = path.split('(')[1] if '(' in path else ''
    if 'libblend2d' in path or b == 'b2d_swap.o': return 'Blend2D upstream (native C++)'
    if re.match(r'fle-\d+\.o$', b): return 'FLEngine (lifted)'
    if re.match(r'm1-', b): return 'dsp/IPP (lifted)'
    if re.match(r'm2-', b): return 'Blend2D module (lifted)'
    if re.match(r'm3-', b): return 'QFC (lifted)'
    if re.match(r'm4-', b): return 'FreeType (lifted)'
    if re.match(r'm5-', b): return 'ILTools (lifted)'
    if b == 'common.o': return 'lifted common'
    if re.match(r'fle_(gdi|gdc|raster|visrgn|clip|blit|region)', b): return 'runtime: GDI/raster'
    if re.match(r'fle_(win|input|desk|layered|class|cursor|menu|timerq|event|thread|appview|dwm|kbd)', b): return 'runtime: windows/messages/threads'
    if b.startswith('fle_'): return 'runtime: other'
    if re.match(r'(apiw_|pix_wrap|parity_prof)', b): return 'recorder (this tool)'
    if re.match(r'(main_two|fruity|a4|image|trace|realmod)', b): return 'harness'
    return 'libc/libm/libstdc++/other'
d = open(prof, 'rb').read(); recs = [struct.unpack('<QQ', d[i:i + 16]) for i in range(0, len(d) - 15, 16)]
anchor = recs[0][0]; recs = recs[1:]
sym = None
for ln in subprocess.run(['nm', binary], capture_output=True, text=True).stdout.splitlines():
    t = ln.split()
    if len(t) == 3 and t[2] == 'parity_prof_anchor': sym = int(t[0], 16)
base = anchor - sym
def attribute(pc):
    k = bisect.bisect_right(starts, pc - base) - 1
    if k < 0: return 'unknown'
    s = sec[k]
    return klass(s[2]) if s[0] <= pc - base < s[0] + s[1] else 'libc/libm/libstdc++/other'
if not ranges: ranges = [(0, 1 << 62)]
for ri, (lo, hi) in enumerate(ranges):
    c = collections.Counter(); n = 0
    for pc, ms in recs:
        if lo < ms <= hi: c[attribute(pc)] += 1; n += 1
    print('%s\tsamples=%d\t%s' % (labels[ri] if ri < len(labels) else '%d:%d' % (lo, hi), n, ' '.join('%s=%.1f%%' % (k, 100.0 * v / n) for k, v in c.most_common(9)) if n else ''))
