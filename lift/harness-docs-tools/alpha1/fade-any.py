#!/usr/bin/env python3
"""alpha1: for a run recorded with a crop (A1_CROP index 0 = 0,0,W,H of the screen), finds the dumps in which a window with a window region is described (the fade of a chosen row) and prints the composite colour of one pixel of the region
(centre row, 40 px from the left of the window) over the dumps, with the colour after the window is gone (`under`). usage: fade-any.py RUN CROPW CROPH CROPX CROPY"""
import re, sys
import numpy as np
run, cw, ch, ox, oy = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
cur = 0; rect = None; frames = []
for ln in open(run + '/api.log', errors='replace'):
    m = re.match(r'#dump (\d+) ', ln)
    if m: cur = int(m.group(1)); continue
    m = re.match(r'D lid \d+ at \((-?\d+), (-?\d+)\) (\d+)x(\d+) .*region\((\d+),(\d+),(\d+),(\d+)\) .*class "TQuickPopupMenuWindow"', ln)
    if m:
        x, y, w, h, l, t, r, b = map(int, m.groups())
        if rect is None: rect = (x, y, w, h, l, t, r, b)
        frames.append(cur)
x, y, w, h, l, t, r, b = rect
px, py = x + min(40, (r - l) // 2) + l, y + (t + b) // 2
def pix(n): return np.fromfile('%s/crops/crop-%04d-0.rgba' % (run, n), dtype=np.uint8).reshape(ch, cw, 4)[py - oy, px - ox, :3].astype(int)
under = pix(frames[-1] + 1)
series = [tuple(pix(n)) for n in frames]
print(run, 'window', rect[:4], 'region', rect[4:], 'pixel', (px, py), 'frames', len(frames), 'under', tuple(under))
print('  first', series[0], 'mid', series[len(series) // 2], 'last', series[-1])
lo = [min(f, u) for f, u in zip(series[0], under)]; hi = [max(f, u) for f, u in zip(series[0], under)]
out = [i for i, s in enumerate(series) if any(not (lo[k] - 1 <= s[k] <= hi[k] + 1) for k in range(3))]
print('  frames outside [first, under] (+-1):', len(out), ' black frames (all 0):', sum(1 for s in series if s == (0, 0, 0)))
