#!/usr/bin/env python3
"""alpha1 (decision 0244): the frame series of a fade on the REAL engine (native -O2 build with tools/alpha1_rec.c, G3 scripts of this directory; THE INPUT IS A HARNESS SIMULATION, not a user operation).
Reads the recorder's api.log and the raw crops of one run and checks, frame by frame, the pixel of a fading window against what the contract says:
  composite = src + (1 - srcA / 255) * under      (premultiplied; BLENDFUNCTION, rounded to the nearest, +-1 allowed for the order of operations)
where `src` and `srcA` are what the guest BitBlt'd into the window (the recorder's "pix(x,y)=A.R.G.B" of the BitBlt whose rectangle is SIZE) and `under` is the composite of the same screen pixel after the window is gone.
It also checks that the composite stays between the first frame's value and `under` in every channel (so a fade to transparent does not pass black unless `under` is black).
usage: check-fade.py RUN_DIR --size WxH --px X,Y --crop K --origin OX,OY --cropsize W,H [--tsv OUT.tsv]
  --size: the size of the BitBlt rectangle that carries the fade (206x279 for the File menu after a choice, 113x108 for the Add menu); --px: the sampled point relative to that rectangle (the BitBlt sample point of the run, A1_BBPIX);
  --crop/--origin/--cropsize: the crop (A1_CROP index K, its screen origin and size) whose pixel at --px is the composite of the same point."""
import re, sys, argparse
import numpy as np
ap = argparse.ArgumentParser()
ap.add_argument('run'); ap.add_argument('--size', required=True); ap.add_argument('--px', required=True); ap.add_argument('--crop', type=int, required=True)
ap.add_argument('--origin', required=True); ap.add_argument('--cropsize', required=True); ap.add_argument('--tsv')
a = ap.parse_args()
px, py = map(int, a.px.split(',')); cw, ch = map(int, a.cropsize.split(','))
dumps = {}      # n -> clock
events = []     # (ms, A, r, g, b)
alive = {}      # n -> True if a window with a region of this size is described at the dump
cur = 0
szw, szh = a.size.split('x')
for ln in open(a.run + '/api.log', errors='replace'):
    m = re.match(r'#dump (\d+) clock_ms (\d+)', ln)
    if m: cur = int(m.group(1)); dumps[cur] = int(m.group(2)); continue
    if ln.startswith('D ') and (' %sx%s ' % (szw, szh)) in ln: alive[cur] = True
    m = re.match(r'E (\d+) BitBlt .*\(0,0 %sx%s\).*pix\(%d,%d\)=(\d+)\.(\d+)\.(\d+)\.(\d+)' % (szw, szh, px, py), ln)
    if m: events.append(tuple(int(v) for v in m.groups()))
def crop(n): return np.fromfile('%s/crops/crop-%04d-%d.rgba' % (a.run, n, a.crop), dtype=np.uint8).reshape(ch, cw, 4)[py, px, :3].astype(int)
frames = [n for n in sorted(alive)]
last = frames[-1]
under = crop(last + 1)
rows = []
for n in frames:
    ms = dumps[n]
    ev = [e for e in events if e[0] <= ms]
    if not ev: continue
    _, A, r, g, b = ev[-1]
    comp = crop(n)
    exp = [min(255, int(np.floor(c + (1 - A / 255.0) * u + 0.5))) for c, u in zip((r, g, b), under)]
    rows.append((n, ms, A, r, g, b, tuple(comp), tuple(exp)))
bad = 0
first = rows[0][6]
for n, ms, A, r, g, b, comp, exp in rows:
    if any(abs(c - e) > 1 for c, e in zip(comp, exp)): bad += 1
    for c, f, u in zip(comp, first, under):
        lo, hi = min(f, u) - 1, max(f, u) + 1
        if not lo <= c <= hi: bad += 1
if a.tsv:
    with open(a.tsv, 'w') as f:
        f.write('frame\tclock_ms\tsrcA\tsrc_r\tsrc_g\tsrc_b\tcomposite_r\tcomposite_g\tcomposite_b\texpected_r\texpected_g\texpected_b\n')
        for n, ms, A, r, g, b, comp, exp in rows: f.write('\t'.join(map(str, (n, ms, A, r, g, b) + tuple(comp) + tuple(exp))) + '\n')
        f.write('# under (the same pixel after the window is gone)\t%s\n' % ','.join(map(str, under)))
print('frames %d, under %s, first %s, last %s, violations %d' % (len(rows), tuple(under), first, rows[-1][6], bad))
sys.exit(1 if bad else 0)
