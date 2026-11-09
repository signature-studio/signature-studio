#!/usr/bin/env python3
# pick1 (decision 0242 addendum): compare two G3 runs snapshot by snapshot (g3-surface hashes, g3-wintable sha, g3-win lines)
# usage: cmp-g3.py SET BEFORE_STDOUT AFTER_STDOUT TABLE.tsv DETAIL.txt
import sys, re
def snaps(p):
    out = []; cur = None
    for l in open(p, errors='replace'):
        l = l.rstrip('\n')
        if l.startswith('g3-snap:'):
            m = re.match(r'g3-snap: #(\d+) (\S+) clock_ms (\d+) entries (\d+)', l)
            cur = {'n': m.group(1), 'label': m.group(2), 'clock': m.group(3), 'surf': [], 'win': [], 'wt': ''}; out.append(cur)
        elif cur is not None:
            if l.startswith('g3-surface:'): cur['surf'].append(l)
            elif l.startswith('g3-win:'): cur['win'].append(l)
            elif l.startswith('g3-wintable:'): cur['wt'] = l
    return out
name, bp, ap, tsv, det = sys.argv[1:6]
b = snaps(bp); a = snaps(ap)
rows = []; d = open(det, 'a')
for i in range(max(len(a), len(b))):
    if i >= len(a) or i >= len(b): rows.append((name, 'missing #%d' % (i + 1), '', '', '')); continue
    x, y = b[i], a[i]
    sc = sum(1 for s in x['surf'] if s not in y['surf']); wc = x['wt'] != y['wt']; gl = x['win'] != y['win']; clk = x['clock'] != y['clock']
    rows.append((name, x['label'], len(x['surf']), sc, ('changed' if (wc or gl) else 'same')))
    if sc or wc or gl:
        d.write('== %s %s (#%s): surfaces changed %d, wintable %s, g3-win lines %s\n' % (name, x['label'], x['n'], sc, 'changed' if wc else 'same', 'changed' if gl else 'same'))
        for l in x['win']:
            if l not in y['win']: d.write('  - before: %s\n' % l[:230])
        for l in y['win']:
            if l not in x['win']: d.write('  + after : %s\n' % l[:230])
        for l in x['surf']:
            if l not in y['surf']: d.write('  - before: %s\n' % l[:160])
        for l in y['surf']:
            if l not in x['surf']: d.write('  + after : %s\n' % l[:160])
with open(tsv, 'a') as o:
    for r in rows: o.write('\t'.join(map(str, r)) + '\n')
print(name, 'snapshots', len(rows), 'changed', sum(1 for r in rows if r[4] == 'changed' or r[3] not in (0, '')))
