"""Measurement aid (docs/investigations/wasm-two-module-o1.md): move chosen functions of the generated units out into their own
translation units, so that they can be compiled at another optimisation level.
usage: split_o1.py SRC_DIR DST_DIR LIST_FILE
  LIST_FILE: lines "<dll-NN.o>\t<f_va>\t..." (first two tab-separated columns used; the first column is the object name).
For each unit dll-NN.c that has chosen functions it writes
  DST_DIR/dll-NN-rest.c   = prelude (includes, externs, forward declarations) + every function except the chosen ones, same order
  DST_DIR/fn-<va>.c       = prelude + that one function
Units without chosen functions are not written (the original object is used for them). The generated C is not changed: the prelude
and the function text are copied verbatim. Existing files are never overwritten (mode 'x')."""
import os
import re
import sys

src, dst, lst = sys.argv[1:4]
fd = re.compile(r"^void f_([0-9a-f]+)\(void\)$")
want = {}
for line in open(lst):
    c = line.rstrip("\n").split("\t")
    if len(c) >= 2:
        want.setdefault(c[0].replace(".o", ".c"), []).append(c[1])
os.makedirs(dst, exist_ok=True)
for unit in sorted(want):
    lines = open(os.path.join(src, unit)).read().split("\n")
    first = next(i for i, l in enumerate(lines) if fd.match(l))
    prelude = lines[:first]
    funcs = []   # (name, text lines)
    i = first
    while i < len(lines):
        m = fd.match(lines[i])
        if m:
            assert lines[i + 1] == "{"
            j = i + 2
            while lines[j] != "}":
                j += 1
            funcs.append(("f_" + m.group(1), lines[i:j + 1]))
            i = j + 1
        else:
            assert lines[i].strip() == "", (unit, i, lines[i][:60])
            i += 1
    names = [n for n, _ in funcs]
    for w in want[unit]:
        assert w in names, (unit, w)
    rest = [t for n, t in funcs if n not in want[unit]]
    stem = unit[:-2]
    with open(os.path.join(dst, stem + "-rest.c"), "x") as f:
        f.write("\n".join(prelude) + "\n")
        for t in rest:
            f.write("\n".join(t) + "\n")
    for n, t in funcs:
        if n in want[unit]:
            with open(os.path.join(dst, "fn-" + n[2:] + ".c"), "x") as f:
                f.write("\n".join(prelude) + "\n")
                f.write("\n".join(t) + "\n")
    print(unit, "functions", len(funcs), "moved", len(want[unit]), "rest", len(rest), "prelude lines", len(prelude))
