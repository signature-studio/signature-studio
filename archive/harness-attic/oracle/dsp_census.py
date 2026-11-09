#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: per-function census of a generated C file (fe-lift lift ... --pdata).

For every `void f_XXXX(void)` body this records, from the generated C only (no disassembly):
  mn       the set of instruction mnemonics from the `/* 0xADDR Mnemonic */` comments of the emitted lines
  unsup    the mnemonics of the lines that became `fle_unsupported(...)` (lifter gaps), with counts
  callees  the functions called or tail-jumped to (`f_XXXX();`)
  stub     1 if the body has a CPU-dispatch stub switch (decision 0079: `switch (t = R[10] + R[0]*8)` after `lea r10`)
  msvc     the number of other `switch (t)` tables (decision 0089/0094: MSVC switch with a proven base)
  lg       the number of lines that call a `fle_lg_*` helper (decision 0075: fle_sse_int2.h legacy SSE/SSE2 forms)
usage: dsp_census.py GEN.c OUT.json
"""
import json
import re
import sys

FN = re.compile(rb"^void f_([0-9a-f]+)\(void\)$")
CMT = re.compile(rb"^L_[0-9a-f]+: \{ /\* 0x([0-9a-f]+) ([A-Za-z0-9_]+)")
CALL = re.compile(rb"\bf_([0-9a-f]+)\(\)")
STUB = b"(R[10] + (R[0]*8)); switch (t)"


def main():
    src, out = sys.argv[1:3]
    funcs = {}
    cur = None
    in_body = False
    with open(src, "rb") as fh:
        for line in fh:
            line = line.rstrip(b"\n")
            m = FN.match(line)
            if m:
                cur = {"mn": {}, "unsup": {}, "callees": set(), "stub": 0, "msvc": 0, "lg": 0, "lines": 0}
                funcs["0x" + m.group(1).decode()] = cur
                in_body = True
                continue
            if cur is None or not in_body:
                continue
            if line == b"}":
                in_body = False
                continue
            if not line.startswith(b"L_"):
                continue
            cur["lines"] += 1
            c = CMT.match(line)
            mn = c.group(2).decode() if c else "?"
            cur["mn"][mn] = cur["mn"].get(mn, 0) + 1
            if b"fle_unsupported(" in line:
                cur["unsup"][mn] = cur["unsup"].get(mn, 0) + 1
            if b"switch (t)" in line:
                if STUB in line:
                    cur["stub"] += 1
                else:
                    cur["msvc"] += 1
            if b"fle_lg_" in line:
                cur["lg"] += 1
            for cm in CALL.finditer(line):
                cur["callees"].add("0x" + cm.group(1).decode())
    for f in funcs.values():
        f["callees"] = sorted(f["callees"])
    with open(out, "x") as fh:
        json.dump({"gen": src, "functions": funcs}, fh)
    n = len(funcs)
    print("functions", n, "with_unsupported", sum(1 for f in funcs.values() if f["unsup"]),
          "stubs", sum(f["stub"] for f in funcs.values()), "msvc_switches", sum(f["msvc"] for f in funcs.values()))


if __name__ == "__main__":
    main()
