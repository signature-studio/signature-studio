"""Measurement aid (investigation only): locals per defined function of a wasm relocatable object, with the function names that
wasm-objdump prints (<name> from the "name"/linking sections). Same section walk as locals_detail.py.
usage: locals_by_name.py FILE.o [PREFIX]      only functions whose name starts with PREFIX (default: all)
output: one line per function: name locals groups body-bytes"""
import mmap
import re
import subprocess
import sys

path = sys.argv[1]
prefix = sys.argv[2] if len(sys.argv) > 2 else ""
f = open(path, "rb")
d = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)


def uleb(p):
    r = 0
    s = 0
    while True:
        b = d[p]
        p += 1
        r |= (b & 0x7F) << s
        s += 7
        if not b & 0x80:
            return r, p


p = 8
code = None
while p < len(d):
    sid = d[p]
    p += 1
    sz, p = uleb(p)
    if sid == 10:
        code = (p, sz)
    p += sz
cp, _ = code
n, q = uleb(cp)
rows = []
for i in range(n):
    l, q2 = uleb(q)
    ng, r = uleb(q2)
    tot = 0
    for g in range(ng):
        c, r = uleb(r)
        r += 1
        tot += c
    rows.append((tot, ng, l))
    q = q2 + l
out = subprocess.run(["wasm-objdump", "-x", "-j", "Code", path], capture_output=True, text=True).stdout
names = re.findall(r"- func\[\d+\] size=\d+(?: <([^>]*)>)?", out)
assert len(names) == n, (len(names), n)
for nm, (tot, ng, l) in zip(names, rows):
    if (nm or "").startswith(prefix):
        print(nm or "?", tot, ng, l)
