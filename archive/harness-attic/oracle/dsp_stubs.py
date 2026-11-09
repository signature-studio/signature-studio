#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: the CPU-dispatch stubs (decision 0079) and the MSVC switches (decision 0089/0094).

Reads the generated C (fe-lift lift REBASED.dll GEN.c --pdata) and the rebased PE, and writes, for every function that
contains a `switch (t)` produced from an indirect jmp:
  stub    : the selector variable (the `movsxd rax,[rip+V]` operand), the table address (`lea r10,[rip+T]`), the lifted
            cases (slot VA -> callee) and the PE's own table (the qword at each slot VA, read from the file image).
            `table_matches_pe` compares the two: every lifted case target must be the PE's qword at that slot.
  msvc    : the site address and the lifted case VAs.
The native side of the oracle needs nothing but the PE: the stub is run with the selector variable patched in the mapped
image (ORACLE_PATCH, identical on both sides), so native executes `jmp [r10+rax*8]` on the real table and lift executes
the switch.
usage: dsp_stubs.py PE GEN.c OUT.json
"""
import json
import re
import struct
import sys

sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
import select_pure as sp  # noqa: E402

FN = re.compile(r"^void f_([0-9a-f]+)\(void\)$")
LD = re.compile(r"fle_ld\(UINT64_C\(0x([0-9a-f]+)\), 4\)")
LEA = re.compile(r"R\[10\] = UINT64_C\(0x([0-9a-f]+)\);")
CASE = re.compile(r"case UINT64_C\(0x([0-9a-f]+)\): \{ f_([0-9a-f]+)\(\); return; \}")
CASE_GOTO = re.compile(r"case UINT64_C\(0x([0-9a-f]+)\): goto L_([0-9a-f]+);")
SITE = re.compile(r"/\* 0x([0-9a-f]+) Jmp \*/")


def main():
    pe, gen, out = sys.argv[1:4]
    d, base, dd, secs = sp.load_pe(pe)
    img = bytearray(max(va + max(vs, rs) for _n, va, vs, rs, _rp, _c in secs))
    for _n, va, vs, rs, rp, _c in secs:
        img[va: va + rs] = d[rp: rp + rs]

    def q(va):
        off = va - base
        return struct.unpack_from("<Q", img, off)[0] if 0 <= off <= len(img) - 8 else None

    def dword(va):
        off = va - base
        return struct.unpack_from("<I", img, off)[0]

    stubs, msvc = [], []
    cur = None
    prev2 = []
    with open(gen) as fh:
        for line in fh:
            m = FN.match(line.rstrip("\n"))
            if m:
                cur = int(m.group(1), 16)
                prev2 = []
                continue
            if "switch (t)" not in line:
                if line.startswith("L_"):
                    prev2.append(line)
                    prev2 = prev2[-2:]
                continue
            site = int(SITE.search(line).group(1), 16)
            if "(R[10] + (R[0]*8)); switch (t)" in line:
                sel = LD.search(prev2[0]) if len(prev2) >= 2 else None
                lea = LEA.search(prev2[1]) if len(prev2) >= 2 else None
                cases = [(int(a, 16), int(f, 16)) for a, f in CASE.findall(line)]
                stubs.append({"fn": cur, "site": site, "sel_var": int(sel.group(1), 16) if sel else None,
                              "table": int(lea.group(1), 16) if lea else None, "cases": cases,
                              "n_cases_in_line": line.count("case UINT64_C")})
            else:
                cs = CASE_GOTO.findall(line)
                msvc.append({"fn": cur, "site": site, "cases": [int(a, 16) for a, _ in cs],
                             "n_cases_in_line": line.count("case UINT64_C"), "all_goto": all(a == b for a, b in cs),
                             "reg": re.search(r"uint64_t t = (R\[\d+\]);", line).group(1),
                             "default": "fle_jumptab_miss" in line})
    bad = 0
    for s in stubs:
        s["sel_init"] = dword(s["sel_var"]) if s["sel_var"] else None
        s["pe_targets"] = [q(a) for a, _f in s["cases"]]
        s["table_matches_pe"] = all(q(a) == f for a, f in s["cases"]) and len(s["cases"]) == s["n_cases_in_line"]
        s["slots_contiguous_from_table"] = [a for a, _f in s["cases"]] == [s["table"] + 8 * i for i in range(len(s["cases"]))]
        bad += not (s["table_matches_pe"] and s["slots_contiguous_from_table"])
    res = {"stubs": stubs, "msvc": msvc, "n_stubs": len(stubs), "n_msvc": len(msvc), "stubs_table_mismatch": bad,
           "sel_vars": sorted({s["sel_var"] for s in stubs}), "sel_init_values": sorted({s["sel_init"] for s in stubs})}
    with open(out, "x") as fh:
        json.dump(res, fh)
    print(json.dumps({k: v for k, v in res.items() if k not in ("stubs", "msvc")}, default=hex))
    from collections import Counter
    print("cases per stub", dict(Counter(len(s["cases"]) for s in stubs)))
    print("stubs per selector variable", {hex(v): sum(1 for s in stubs if s["sel_var"] == v) for v in res["sel_vars"]})
    print("msvc", [(hex(m["fn"]), hex(m["site"]), len(m["cases"])) for m in msvc])


if __name__ == "__main__":
    main()
