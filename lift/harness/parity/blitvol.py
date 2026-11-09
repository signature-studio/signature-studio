#!/usr/bin/env python3
"""parity1: pixels written by the raster primitives (fle_r_blit / fle_r_rop_rect / fle_r_mix_rect) per operation. usage: blitvol.py BLITLOG NATIVE_STDOUT"""
import re, struct, sys
recs = []
d = open(sys.argv[1], 'rb').read()
for i in range(0, len(d) - 15, 16):
    ms, kind, px = struct.unpack('<IIQ', d[i:i + 16]); recs.append((ms, kind, px))
snaps = []
for ln in open(sys.argv[2], errors='replace'):
    m = re.match(r'g3-snap: #\d+ (\S+) clock_ms (\d+)', ln)
    if m: snaps.append((m.group(1), int(m.group(2))))
# the sys clock is the same scale as g3's clock_ms (ticks / 10000)
prev = snaps[0][1] if snaps else 0
print('%-24s %8s %9s %9s %12s %10s' % ('op', 'virt_ms', 'blit_n', 'rop/mix_n', 'pixels', 'px_per_vs'))
for k in range(1, len(snaps)):
    nm, t1 = snaps[k]; t0 = snaps[k - 1][1]
    sel = [r for r in recs if t0 < r[0] <= t1]
    nb = sum(1 for r in sel if r[1] == 1); nr = sum(1 for r in sel if r[1] != 1); px = sum(r[2] for r in sel)
    v = t1 - t0
    print('%-24s %8d %9d %9d %12d %10.0f' % (nm, v, nb, nr, px, px * 1000.0 / v if v else 0))
