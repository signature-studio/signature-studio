#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: which instructions of decision 0075 were EXECUTED in bit_exact cases.

A function that contains a 0075 mnemonic is not a test of that mnemonic unless a case ran it. The native single-step run
(ORACLE_TRACE, work/out-ntr-NN.jsonl of run_diff2.py) reports the set of executed rips per case. For every bit_exact case
of a-cases.json this intersects those rips with the addresses of the 26 mnemonics (xmm forms, dsp_pool.classify rules)
inside the function, and counts per mnemonic: instruction sites executed, functions with such a case, bit_exact cases.
The trace keeps at most OR_TRACE_SET (4096) distinct rips per case; a case that hits the cap would undercount (counted).
usage: dsp_exec_mn.py PE RESULTDIR PURE.json OUT.json
"""
import glob
import json
import os
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402
import dsp_pool as dp  # noqa: E402
from capstone import CS_OP_REG  # noqa: E402


def sites(md, code, base, lo, hi):
    out = {}
    for ins in md.disasm(code[lo - base: hi - base], lo):
        mn = ins.mnemonic
        if mn in dp.D0075 and any(op.type == CS_OP_REG and md.reg_name(op.reg).startswith("xmm") for op in ins.operands):
            if mn == "pextrw" and bytes(ins.bytes).find(b"\x0f\xc5") < 0:
                continue
            out[ins.address] = mn
    return out


def main():
    pe, rdir, purej, out = sys.argv[1:5]
    sp.init(pe)
    md, code, base = sp.G["md"], sp.G["code"], sp.G["base"]
    cases = json.load(open(os.path.join(rdir, "a5-cases.json")))
    fns = json.load(open(os.path.join(rdir, "a5-functions.json")))
    pure = {f["va"]: int(f["end"], 16) for f in json.load(open(purej))["functions"]}
    site = {f["fn"]: sites(md, code, base, int(f["fn"], 16), pure[f["fn"]]) for f in fns}
    be = {c["id"]: c["fn"] for c in cases if c["category"] == "bit_exact"}
    rips = {}
    capped = 0
    for fn in sorted(glob.glob(os.path.join(rdir, "work", "out-ntr-*.jsonl"))):
        for ln in open(fn):
            if not ln.strip():
                continue
            o = json.loads(ln)
            if o["id"] in be and o.get("status") == "ok" and "trace" in o:
                rips[o["id"]] = set(o["trace"]["rips"])
                if len(o["trace"]["rips"]) >= 4096:
                    capped += 1
    site_cases = Counter()
    mn_fn = defaultdict(set)
    mn_cases = Counter()
    mn_site_exec = defaultdict(set)
    no_trace = 0
    for cid, fn in be.items():
        r = rips.get(cid)
        if r is None:
            no_trace += 1
            continue
        seen = set()
        for a, mn in site[fn].items():
            if a in r:
                site_cases[a] += 1
                mn_site_exec[mn].add(a)
                seen.add(mn)
        for mn in seen:
            mn_fn[mn].add(fn)
            mn_cases[mn] += 1
    res = {"bit_exact_cases": len(be), "bit_exact_cases_without_trace": no_trace, "traces_at_cap": capped,
           "sites_in_sample": sum(len(s) for s in site.values()),
           "sites_executed_in_bit_exact": len(site_cases),
           "per_mnemonic": {mn: {"sites_executed": len(mn_site_exec[mn]), "functions": len(mn_fn[mn]),
                                 "bit_exact_cases_executing": mn_cases[mn]} for mn in sorted(mn_cases)},
           "mnemonics_in_sample_sites": dict(Counter(mn for s in site.values() for mn in s.values()).most_common()),
           "mnemonics_executed_in_bit_exact": sorted(mn_cases),
           "mnemonics_in_sample_never_executed_bit_exact": sorted({mn for s in site.values() for mn in s.values()} - set(mn_cases)),
           "functions_with_executed_site": len({fn for s in mn_fn.values() for fn in s})}
    with open(out, "x") as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
