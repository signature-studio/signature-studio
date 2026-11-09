#!/usr/bin/env python3
"""Decision 0155 (provisional). Makes generated C usable with -DFLE_LOWREGS (runtime/fle_lowregs.h): the definitions and the extern declarations of the guest register
file R and of the flags FL are put in `#ifndef FLE_LOWREGS ... #endif`, those of the XMM file X in `#ifndef FLE_LOWREGS_X ... #endif` (the optional fixed-address X, fle_sse.h). Without the macro the file means what it meant before (nothing else is touched).
usage: lowregs.py FILE.c...      (in place; the lines are in the first lines of a file: common.c of merge_tables_n.py, the output of fe-lift, the units of split_bytes.py / split_mod.py)"""
import re
import sys

PAT = re.compile(r'^(?:extern )?(?:uint64_t R\[16\];|fle_eflags FL(?: = \{[^}]*\})?;)$')
PATX = re.compile(r'^(?:extern )?fle_xmm X\[16\];$')
for path in sys.argv[1:]:
    with open(path, 'r') as f:
        text = f.read()
    cut = text.find('\nvoid f_')
    head, tail = (text[:cut], text[cut:]) if cut >= 0 else (text, '')
    if '#ifndef FLE_LOWREGS' in head:
        continue
    out = []
    n = 0
    for line in head.split('\n'):
        if PAT.match(line):
            out.append('#ifndef FLE_LOWREGS\n' + line + '\n#endif')
            n += 1
        elif PATX.match(line):
            out.append('#ifndef FLE_LOWREGS_X\n' + line + '\n#endif')
            n += 1
        else:
            out.append(line)
    if n:
        with open(path, 'w') as f:
            f.write('\n'.join(out) + tail)
    print('%s: %d lines wrapped' % (path, n))
