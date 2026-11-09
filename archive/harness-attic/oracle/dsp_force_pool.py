#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: pure.json plus the functions that contain a resolved MSVC switch (forced, see
dsp_run_msvc.py), in the pure.json format, so that argptr.py can give them pointer information.
usage: dsp_force_pool.py PE PURE.json STUBS.json OUT.json     (OUT.json must not exist)
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402


def main():
    pe, pure_json, stubs_json, out = sys.argv[1:5]
    pure = json.load(open(pure_json))
    st = json.load(open(stubs_json))
    d, base, dd, secs = sp.load_pe(pe)
    ends = dict(sp.pdata_functions(d, base, dd, secs))
    sp.init(pe)
    have = {f["va"] for f in pure["functions"]}
    add = []
    for fn in sorted({m["fn"] for m in st["msvc"]}):
        if "0x%x" % fn in have:
            continue
        lo, hi, n, bad, tg = sp.analyze((fn, ends[fn]))
        add.append({"va": "0x%x" % lo, "end": "0x%x" % hi, "insns": n, "calls": ["0x%x" % t for t in tg],
                    "forced": True, "first_exclusion_reason": bad})
    res = dict(pure)
    res["functions"] = pure["functions"] + add
    res["counts"] = dict(pure["counts"], forced_added=len(add))
    with open(out, "x") as fh:
        json.dump(res, fh, indent=1)
    print("added", [(f["va"], f["end"], f["insns"], f["first_exclusion_reason"]) for f in add])


if __name__ == "__main__":
    main()
