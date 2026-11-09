#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: the functions with an MSVC switch (decision 0089/0094), forced into the sample.

The 4 functions that contain the 5 switches the lifter resolved (dsp_stubs.py) each also contain something that makes
select_pure.py exclude them (another indirect jmp the lifter could NOT resolve, a direct jmp target that is not a lifted
function entry, an `in` mnemonic from the linear disassembly, a callee that is not pure). This script runs them anyway,
OUTSIDE the FLEngine criteria: a path that reaches an unresolved jmp ends in lift_unsupported (lift) and a path that reaches
something the native side cannot run ends in native_fault; only the other paths are compared.
Cases: run_diff2.gen_cases (n of them) plus an index sweep: the `small` case with every non-pointer argument set to v for
v in 0..SWEEP-1, which drives a switch index that comes straight from an argument through the table range and past it.
The native single-step run (ORACLE_TRACE) gives the executed rips; for the 5 resolved sites the script reports which of the
switch arms (case targets) a bit_exact case executed.
usage: dsp_run_msvc.py PE BINDIR STUBS.json ARGPTR_FORCE.json OUTDIR [--n 16] [--sweep 26] [--seed 1] [--jobs 8]
OUTDIR must not exist.
"""
import argparse
import copy
import json
import os
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run_diff2 as rd  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pe")
    ap.add_argument("bindir")
    ap.add_argument("stubs")
    ap.add_argument("argptr")
    ap.add_argument("out")
    ap.add_argument("--n", type=int, default=16)
    ap.add_argument("--sweep", type=int, default=26)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=8)
    a = ap.parse_args()
    os.makedirs(a.out)
    work = os.path.join(a.out, "work")
    os.makedirs(work)
    st = json.load(open(a.stubs))
    info = json.load(open(a.argptr))["functions"]
    fns = sorted({m["fn"] for m in st["msvc"]})
    cases = []
    for fn in fns:
        base = rd.gen_cases(fn, a.n, a.seed, info["0x%x" % fn])
        cases += base
        P = set(info["0x%x" % fn]["ptr"])
        for v in range(a.sweep):
            c = copy.deepcopy(base[2])
            c["id"] = "%x.s%d" % (fn, v)
            for j, rn in enumerate(rd.ARGREG):
                if j not in P:
                    c["gpr"][rd.GIDX[rn]] = v
            c["stk"] = [v if (4 + i) not in P else x for i, x in enumerate(c["stk"])]
            cases.append(c)
    byid = {c["id"]: c for c in cases}
    env = dict(os.environ)
    env_tr = dict(env)
    env_tr["ORACLE_TRACE"] = "1"
    chunks = [cases[i::a.jobs] for i in range(a.jobs)]
    nb, lb = os.path.join(a.bindir, "native_drv2"), os.path.join(a.bindir, "lift_drv2")
    nat, _ = rd.run_side(nb, a.pe, chunks, work, "nat", env)
    lif, _ = rd.run_side(lb, a.pe, chunks, work, "lift", env)
    ntr, _ = rd.run_side(nb, a.pe, chunks, work, "ntr", env_tr)
    retry = [c["id"] for c in cases if (nat.get(c["id"]) or {}).get("status") == "timeout" or
             (lif.get(c["id"]) or {}).get("status") == "timeout"]
    if retry:
        e10 = dict(env)
        e10["ORACLE_TIMEOUT"] = "10"
        rc = [byid[i] for i in retry]
        rch = [rc[i::4] for i in range(4)]
        n_r, _ = rd.run_side(nb, a.pe, rch, work, "natr", e10)
        l_r, _ = rd.run_side(lb, a.pe, rch, work, "liftr", e10)
        nat.update(n_r)
        lif.update(l_r)
    rows = []
    for c in cases:
        cat, det = rd.classify(nat.get(c["id"]), lif.get(c["id"]), Counter())
        rows.append({"fn": "0x%x" % c["fn"], "id": c["id"], "category": cat, "detail": det, "retried": c["id"] in retry})
    # arms executed by bit_exact cases
    sites = defaultdict(lambda: {"site_executed_cases": 0, "arms_executed": set(), "bit_exact_site_cases": 0})
    cat_of = {r["id"]: r["category"] for r in rows}
    for c in cases:
        t = ntr.get(c["id"])
        if not t or t.get("status") != "ok" or "trace" not in t:
            continue
        rips = set(t["trace"]["rips"])
        for m in st["msvc"]:
            if m["fn"] != c["fn"] or m["site"] not in rips:
                continue
            s = sites["0x%x" % m["site"]]
            s["site_executed_cases"] += 1
            if cat_of[c["id"]] == "bit_exact":
                s["bit_exact_site_cases"] += 1
                s["arms_executed"] |= {x for x in m["cases"] if x in rips}
    site_out = {}
    for m in st["msvc"]:
        k = "0x%x" % m["site"]
        s = sites[k]
        site_out[k] = {"fn": "0x%x" % m["fn"], "arms": len(m["cases"]), "native_traced_cases_reaching_site": s["site_executed_cases"],
                       "bit_exact_cases_reaching_site": s["bit_exact_site_cases"], "distinct_arms_executed_in_bit_exact": len(s["arms_executed"])}
    per_fn = defaultdict(Counter)
    for r in rows:
        per_fn[r["fn"]][r["category"]] += 1
    summary = {"functions": ["0x%x" % f for f in fns], "cases_total": len(rows),
               "categories": dict(Counter(r["category"] for r in rows)), "per_function": {k: dict(v) for k, v in per_fn.items()},
               "sites": site_out,
               "mismatch_fields": dict(Counter(f for r in rows if r["category"].startswith("mismatch") for f in r["detail"]["fields"])),
               "lift_unsupported_messages": dict(Counter(r["detail"]["msg"] for r in rows if r["category"] == "lift_unsupported")),
               "native_fault_status": dict(Counter(r["detail"]["native"] for r in rows if r["category"] == "native_fault"))}
    for name, obj in (("msvc-summary.json", summary), ("msvc-cases.json", rows)):
        with open(os.path.join(a.out, name), "x") as fh:
            json.dump(obj, fh, indent=1)
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
