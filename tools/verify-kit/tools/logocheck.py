#!/usr/bin/env python3
# logocheck.py: the windows just after the first idle with the welcome wizard open (wiz0), 2.5 s later and after closeWizard, for out/logo-{new,prev}-N/ (E9 check item 6: lid 8, the logo window, may be 326x460 = time driven)
import json, glob, os, re
K = os.path.dirname(os.path.dirname(os.path.abspath(__file__))); O = K + '/out'
rows = ['| 実行 | bundle | 時点 | 可視の窓（lid class rect） | lid 8 の大きさ | load / I/O 圧（開始） |', '|---|---|---|---|---|---|']
for var in ('new', 'prev'):
    for d in sorted(glob.glob(f'{O}/logo-{var}-*/')):
        p = d + 'report.json'
        if not os.path.exists(p): continue
        r = json.load(open(p))[0]; wl = r.get('winlogs') or {}
        for k, name in (('wiz0-a', 'wiz0'), ('wiz0-b-2.5s-later', 'wiz0 の 2.5 s 後'), ('after-closeWizard', 'closeWizard の後')):
            ls = wl.get(k) or []
            vis = '; '.join(re.sub(r"^(\d+) (\S+) '[^']*' @(\S+) (\S+) z\d+", r'\1 \2 @\3 \4', l) for l in ls) or '(なし)'
            l8 = [re.search(r' @\S+ (\d+x\d+)', l).group(1) for l in ls if l.startswith('8 ')]
            rows.append(f"| {os.path.basename(d.rstrip('/'))} | {var} | {name} | {vis} | {l8[0] if l8 else '-'} | {r.get('loadStart')} / {r.get('ioStart')} |")
out = '\n'.join(rows); open(O + '/LOGO.md', 'w').write(out + '\n'); print(out)
