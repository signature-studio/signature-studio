#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: how many sampled `.pdata` starts are not function entries.

select_pure.py takes every non-chained `.pdata` start as a function. In the DLL a start whose UNWIND_INFO has SizeOfProlog == 0
is typically an epilogue or cold fragment (e.g. `add rsp, 0x48; ret`): called as a function, its `ret` takes a random return
address from the oracle's stack and native faults, while lift's `ret` just returns. This script counts, per result directory,
the sampled functions and cases with SizeOfProlog == 0 and what the cases ended as. It reads the UNWIND_INFO byte 1 of the PE.
Only a heuristic: SizeOfProlog == 0 does not prove a fragment (a leaf function without a prologue also has 0).
usage: dsp_fragments.py PE RESULTDIR [RESULTDIR...]
"""
import json
import os
import struct
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402


def main():
    pe = sys.argv[1]
    d, base, dd, secs = sp.load_pe(pe)
    prva, psz = dd[3]
    po = sp.rva_off(secs, prva)
    prolog = {}
    for i in range(psz // 12):
        b, e, u = struct.unpack_from("<III", d, po + 12 * i)
        uo = sp.rva_off(secs, u & ~1)
        prolog[base + b] = d[uo + 1]
    for rdir in sys.argv[2:]:
        cases = json.load(open(os.path.join(rdir, "a5-cases.json")))
        fns = json.load(open(os.path.join(rdir, "a5-functions.json")))
        nf = json.load(open(os.path.join(rdir, "a5-native-faults.json")))
        z = {f["fn"] for f in fns if prolog[int(f["fn"], 16)] == 0}
        cz, cn = Counter(), Counter()
        for c in cases:
            (cz if c["fn"] in z else cn)[c["category"]] += 1
        ret = {r["fn"] for r in nf if r["cause"] == "gp_other" and r["extra"] == "ret"}
        vd = Counter(f["verdict"] for f in fns if f["fn"] in z)
        vn = Counter(f["verdict"] for f in fns if f["fn"] not in z)
        print(json.dumps({"dir": rdir, "functions": len(fns), "sizeofprolog0_functions": len(z),
                          "cases_sizeofprolog0": dict(cz), "cases_sizeofprolog_gt0": dict(cn),
                          "function_verdicts_sizeofprolog0": dict(vd), "function_verdicts_sizeofprolog_gt0": dict(vn),
                          "functions_with_native_fault_at_ret": len(ret), "of_which_sizeofprolog0": len(ret & z),
                          "native_fault_at_ret_cases": sum(1 for r in nf if r["cause"] == "gp_other" and r["extra"] == "ret")}))


if __name__ == "__main__":
    main()
