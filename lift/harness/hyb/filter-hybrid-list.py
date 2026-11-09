#!/usr/bin/env python3
"""mt4 stage A: removes from hybrid function lists (lift/harness/seeds/hyb-<set>-<module>.txt: one `0xVA` per line, `#` comments) the functions that contain a LOCK-prefixed or implicitly locked instruction. fe-wasmgen leaves such a
function on the C path (the atomic read-modify-write is fle_cas / fle_xchg / fle_xadd there), and the build's hybn0 check (chk_lowered.py) wants every listed function emitted. The removal list is a TSV with the VA in the first column
(mt4-atomic-functions.tsv); `#` lines and a header line `va` are ignored. The comment lines of a list stay; a comment line `# mt4: removed N functions (LOCK-prefixed instruction)` is added after the first comment.
The REMOVE.tsv files (decisions 0246 and 0243): seeds/mt4-atomic-functions.tsv for the six modules of HEAD (hyb-U-{fle,dsp,b2d,qfc,ft,ilt}.txt and the older sets), seeds/mt4-atomic-functions-m67.tsv for oggio and Elastique
(hyb-U-oggio.txt, hyb-U-elastique.txt). A list that is filtered with the file of another lift keeps its functions: the file names the functions by VA, a VA that is not in the list is ignored.
usage: filter-hybrid-list.py REMOVE.tsv LIST.txt [LIST.txt ...]      (rewrites the lists; prints module list / removed / kept; a list without a `0xVA` per line is refused)
       filter-hybrid-list.py --report REMOVE.tsv LIST.txt ...         (only counts; mt4-hyb-U-filter.tsv: the rows `list<TAB>va`)"""
import sys, re
args = sys.argv[1:]
report = False
if args and args[0] == '--report':
    report = True; args = args[1:]
rem = set()
for l in open(args[0]):
    l = l.strip()
    if not l or l.startswith('#') or l.startswith('va'): continue
    rem.add(int(l.split()[0], 16))
rows = []
for p in args[1:]:
    lines = open(p).read().split('\n')
    keep, gone = [], []
    for l in lines:
        s = l.strip()
        if s and not s.startswith('#'):
            m = re.match(r'^(0x[0-9a-fA-F]+)\b', s)
            if not m: sys.exit('%s: a list line that is not 0xVA: %r' % (p, l))
            if int(m.group(1), 16) in rem:
                gone.append(m.group(1)); continue
        keep.append(l)
    print('%s: %d removed, %d kept' % (p, len(gone), sum(1 for l in keep if l.strip() and not l.strip().startswith('#'))))
    for g in gone: rows.append((p, g))
    if gone and not report:
        i = next((k for k, l in enumerate(keep) if l.startswith('#')), -1) + 1
        keep.insert(i, '# mt4 stage A: removed %d functions that contain a LOCK-prefixed or implicitly locked instruction (they stay on the C path; tools/filter-hybrid-list.py)' % len(gone))
        open(p, 'w').write('\n'.join(keep))
if report:
    for p, g in rows: print('ROW\t%s\t%s' % (p, g))
