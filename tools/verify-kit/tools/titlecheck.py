#!/usr/bin/env python3
# titlecheck.py: the minimize -> restore title question (decision 0277, E8/E9) from the pictures of out/title/: for each variant, is the playlist header crumb the same as the known '(none)' crop (out/minrestore/3-after-restore.png) or as the start picture, and where do the restored picture and the start picture differ (bounding box, rows)
import sys, glob, json, os, numpy as np
src = open(__file__.rsplit('/', 1)[0] + '/pngdiff.py').read().split("A = dec(sys.argv[1])")[0]
sys.argv = ['x']; exec(src)
K = __file__.rsplit('/', 2)[0]; O = K + '/out'
start = dec(O + '/title/0-start.png'); none = dec(O + '/minrestore/3-after-restore.png' if os.path.exists(O + '/minrestore/3-after-restore.png') else K + '/out-e8main/minrestore/3-after-restore.png'); box = (1125, 88, 1400, 106)
crop = lambda a: a[box[1]:box[3], box[0]:box[2]]
same = lambda a, b: int((np.abs(crop(a) - crop(b)).max(axis=2) > 0).sum())
rows = ['| 変種 | 復元後のプレイリスト見出し（crop 1125-1400 x 88-106）の start との差 | 既知の『(none)』との差 | 復元後と起動直後の画で違う画素（全体 / 行の範囲） | load / I/O 圧（開始） |', '|---|---|---|---|---|']
rep = {r['name']: r for r in json.load(open(O + '/title/report.json'))}
for f in sorted(glob.glob(O + '/title/scenario-*.png')):
    n = f.rsplit('scenario-', 1)[1][:-4]; a = dec(f); m = (np.abs(start - a).max(axis=2) > 0); ys = np.where(m.any(axis=1))[0]
    r = rep.get(n, {})
    rows.append(f"| {n} | {same(start, a)} 画素 | {same(none, a)} 画素 | {int(m.sum())} / y {ys.min()}..{ys.max()} | {r.get('loadStart')} / {r.get('ioStart')}")
out = '\n'.join(r if r.endswith('|') else r + ' |' for r in rows)
open(O + '/title/TITLE.md', 'w').write(out + '\n'); print(out)
