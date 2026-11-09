#!/usr/bin/env python3
"""Compare two generated C files function by function (body SHA-256 of `void f_XXXX(void)` .. closing `}`), and list the
functions that differ, optionally intersected with a set of function VAs (a pure.json, a sample).
Used to decide whether a measurement made with one lifter build still holds for another (the other lines of the file, the
prologue with the tables, are compared as a whole and reported separately).
usage: dsp_cmp_gen.py OLD.c NEW.c OUT.json [PURE.json]
"""
import hashlib
import json
import re
import sys

FN = re.compile(rb"^void f_([0-9a-f]+)\(void\)$")


def bodies(path):
    out = {}
    pro = hashlib.sha256()
    cur = None
    h = None
    with open(path, "rb") as fh:
        for line in fh:
            m = FN.match(line.rstrip(b"\n"))
            if m:
                cur = "0x" + m.group(1).decode()
                h = hashlib.sha256()
                h.update(line)
                continue
            if cur is None:
                pro.update(line)
                continue
            h.update(line)
            if line == b"}\n":
                out[cur] = h.hexdigest()
                cur = None
    return out, pro.hexdigest()


def main():
    old, new, out = sys.argv[1:4]
    pure = None
    if len(sys.argv) > 4:
        pure = {f["va"] for f in json.load(open(sys.argv[4]))["functions"]}
    a, pa = bodies(old)
    b, pb = bodies(new)
    only_a = sorted(set(a) - set(b))
    only_b = sorted(set(b) - set(a))
    diff = sorted(v for v in set(a) & set(b) if a[v] != b[v])
    res = {"functions_old": len(a), "functions_new": len(b), "only_old": only_a, "only_new": only_b,
           "bodies_equal": sum(1 for v in set(a) & set(b) if a[v] == b[v]), "bodies_differ": diff,
           "prologue_equal": pa == pb}
    if pure is not None:
        res["differ_in_pure"] = [v for v in diff if v in pure]
    with open(out, "x") as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
