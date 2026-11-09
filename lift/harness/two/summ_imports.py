"""Summarise a gdb_imports.py log: collapse runs of the same API into 'name xN (first args)'. Read-only.
usage: summ_imports.py LOG"""
import re
import sys

rows = []
for l in open(sys.argv[1]):
    m = re.match(r"\s*(\d+) (fle_imp_\w+) rcx=(\S+) rdx=(\S+) r8=(\S+) r9=(\S+) ret=(\S+)", l)
    if m:
        rows.append(m.groups())
    elif l.strip() and not l.startswith(("breakpoints", "DllMain")) and "fle_imp_" not in l:
        print("      " + l.rstrip())
    elif l.startswith(("DllMain", "breakpoints")):
        print(l.rstrip())
i = 0
while i < len(rows):
    j = i
    while j + 1 < len(rows) and rows[j + 1][1] == rows[i][1]:
        j += 1
    n, name, rcx, rdx, r8, r9, ret = rows[i]
    print("%4s%s %s rcx=%s rdx=%s r8=%s ret=%s" % (n, "-" + rows[j][0] if j > i else "", name[8:] + (" x%d" % (j - i + 1) if j > i else ""), rcx, rdx, r8, ret))
    i = j + 1
