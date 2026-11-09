import re
import sys
from collections import Counter

IDATA_LO = 0x196F000
IDATA_HI = 0x1975710

pat = re.compile(r"\t(call|jmp)\s+QWORD PTR \[rip[^#]*#\s*0x([0-9a-f]+)")
c = Counter()
with open(sys.argv[1], encoding="utf-8", errors="replace") as f:
    for line in f:
        m = pat.search(line)
        if not m:
            continue
        v = int(m.group(2), 16)
        where = "into_idata" if IDATA_LO <= v < IDATA_HI else "other"
        c[(m.group(1), where)] += 1
for k in sorted(c):
    print(k[0], k[1], c[k])
