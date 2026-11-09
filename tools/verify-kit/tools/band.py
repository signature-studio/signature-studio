#!/usr/bin/env python3
"""band.py RUNDIR: the pictures p1..p5 of a f9-band run (raw RGBA 1920x1080, the compositor's picture): the Mixer was dragged to the right and back (p1), F9 hid it (p2), F9 showed it (p3), then two clicks on the main window's maximize button (p4, p5).
Differing pixels between steps in the band x1903-1912 y548-1019 (the right edge of the Mixer) and the Mixer's lower edge x0-1919 y1022-1040. Pairs: p1 vs p3 (what F9 changes), p3 vs p5 (the picture after the page's own repaint-causing clicks)."""
import sys, numpy as np
W, H = 1920, 1080
def load(d, n): return np.fromfile(f'{d}/{n}.rgba', dtype=np.uint8).reshape(H, W, 4)[:, :, :3].astype(int)
def cnt(a, b, x0, y0, x1, y1): return int((np.abs(a[y0:y1, x0:x1] - b[y0:y1, x0:x1]).max(axis=2) > 0).sum())
d = sys.argv[1]
p = {n: load(d, n) for n in ['p1-dragged-back', 'p2-f9-hidden', 'p3-f9-shown', 'p5-maximized-again']}
print('pair                                   band(1903-1912,548-1019) | lower edge(y1022-1040) | whole screen')
for a, b in [('p1-dragged-back', 'p3-f9-shown'), ('p3-f9-shown', 'p5-maximized-again'), ('p1-dragged-back', 'p5-maximized-again')]:
    print(f'{a:18s} vs {b:20s} {cnt(p[a], p[b], 1903, 548, 1913, 1020):8d} | {cnt(p[a], p[b], 0, 1022, W, 1041):8d} | {cnt(p[a], p[b], 0, 0, W, H):8d}')
