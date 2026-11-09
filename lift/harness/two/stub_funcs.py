"""Measurement aid (not a generator change, not a decision): copy the translation units of a two-module build and replace the BODY of the
listed lifted functions by a named stop  { fle_unsupported(0xVA); }  (the runtime's own stop for "instruction not lifted", the same call the
generator emits for an unsupported instruction). Used only to find out whether a wasm build with the functions that exceed the 50,000-locals
limit of the WebAssembly JS API removed from the way builds, links and runs in node. The replaced functions keep their symbol and their row in
the dispatch table; calling one stops the run with `fle: unsupported instruction at 0xVA`.
usage: stub_funcs.py SRC_DIR DST_DIR LIST.txt     (LIST: one hex VA per line, with or without 0x; DST_DIR must not hold the output files)
Writes DST_DIR/dll-NN.c for every dll-NN.c of SRC_DIR (identical when no listed function is in it) and copies common.c and *-tables.txt.
Prints, per file, the replaced functions."""
import glob
import os
import re
import shutil
import sys

src, dst, lst = sys.argv[1:4]
want = {int(l.strip(), 16) for l in open(lst) if l.strip()}
fd = re.compile(r"^void f_([0-9a-f]+)\(void\)$")
os.makedirs(dst, exist_ok=True)
done = set()
for p in sorted(glob.glob(os.path.join(src, "dll-[0-9][0-9].c"))):
    lines = open(p).read().split("\n")
    out = []
    i = 0
    rep = []
    while i < len(lines):
        m = fd.match(lines[i])
        if m and i + 1 < len(lines) and lines[i + 1] == "{" and int(m.group(1), 16) in want:
            va = int(m.group(1), 16)
            j = i + 2
            while lines[j] != "}":
                j += 1
            out += [lines[i], "{", "L_%x: { /* stub_funcs.py: body of f_%x removed (more than 50,000 wasm locals) */ fle_unsupported(UINT64_C(%#x)); }" % (va, va, va), "}"]
            rep.append((va, j - i + 1))
            done.add(va)
            i = j + 1
            continue
        out.append(lines[i])
        i += 1
    with open(os.path.join(dst, os.path.basename(p)), "x") as f:
        f.write("\n".join(out))
    if rep:
        print(os.path.basename(p), "replaced", [("f_%x" % v, "%d lines" % n) for v, n in rep])
for extra in ("common.c", "dll-tables.txt"):
    q = os.path.join(src, extra)
    if os.path.exists(q) and not os.path.exists(os.path.join(dst, extra)):
        shutil.copyfile(q, os.path.join(dst, extra))
missing = want - done
print("replaced %d of %d listed functions%s" % (len(done), len(want), "" if not missing else "; NOT FOUND: " + ", ".join(hex(v) for v in sorted(missing))))
sys.exit(1 if missing else 0)
