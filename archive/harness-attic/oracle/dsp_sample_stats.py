#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: what is in a run_diff2.py sample, and why cases were lift_unsupported.

Inputs: RESULTDIR (run_diff2.py output: a5-cases.json, a5-functions.json), CLASSES.json (dsp_pool.py), CENSUS.json
(dsp_census.py), GEN.c (the generated C the lift driver was built from), LIFTSTDOUT.txt (the fe-lift stdout, whose
`unsupported` section gives the lifter's own reason per mnemonic).
Output (OUT.json, must not exist) and a text summary on stdout:
  composition   functions in the sample that contain: a decision-0075 mnemonic (xmm form), any legacy SSE/SSE2, SSE3+ forms,
                VEX/EVEX, a CPU-dispatch stub (0079), a resolved MSVC switch (0094/0095); fle_unsupported own / reached
  by_mnemonic   per decision-0075 mnemonic: functions in the sample
  lift_unsupported   cases / sites / functions per unsupported mnemonic with the lifter's reason text and the class
                (vex = V*/K* forms and BMI/AVX-only; sse3p = SSE3/SSSE3/SSE4 forms that the runtime CPUID does not claim;
                other)
usage: dsp_sample_stats.py RESULTDIR CLASSES.json CENSUS.json GEN.c LIFTSTDOUT.txt OUT.json
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dsp_pool as dp  # noqa: E402


def klass(mn):
    low = mn.lower()
    if mn.startswith(("V", "K")) or low in ("shlx", "shrx", "sarx", "andn", "bzhi", "pdep", "pext", "mulx", "rorx", "blsi", "blsr", "blsmsk"):
        return "vex"
    if low in dp.SSE3P:
        return "sse3p"
    return "other"


def main():
    rdir, classes, census, gen, stdout, out = sys.argv[1:7]
    cases = json.load(open(os.path.join(rdir, "a5-cases.json")))
    fns = json.load(open(os.path.join(rdir, "a5-functions.json")))
    cl = json.load(open(classes))
    cen = json.load(open(census))["functions"]
    sel = [f["fn"] for f in fns]

    def has(v, k):
        return bool(cl[v]["kinds"].get(k))

    comp = {"functions": len(sel), "d0075": sum(has(v, "d0075") for v in sel), "legacy_sse_any": sum(has(v, "sse") for v in sel),
            "sse3_or_later_forms": sum(has(v, "sse3p") for v in sel), "vex_evex": sum(has(v, "vex") for v in sel),
            "cpu_dispatch_stub_0079": sum(cen[v]["stub"] > 0 for v in sel), "msvc_switch_0094_0095": sum(cen[v]["msvc"] > 0 for v in sel),
            "unsupported_own": sum(cl[v]["unsup_own"] for v in sel), "unsupported_reached": sum(cl[v]["unsup_reach"] for v in sel)}
    mn = Counter()
    for v in sel:
        for m in cl[v]["d0075"]:
            mn[m] += 1
    comp["d0075_mnemonics_present"] = len(mn)
    comp["d0075_mnemonics_absent"] = sorted(dp.D0075 - set(mn))
    percase = defaultdict(Counter)
    for c in cases:
        percase[c["fn"]][c["category"]] += 1
    bykind = defaultdict(Counter)
    for v in sel:
        k = "d0075" if has(v, "d0075") else "vex_only" if has(v, "vex") and not has(v, "sse") else "legacy_sse_no_d0075" if has(v, "sse") else "no_xmm"
        for cat, n in percase[v].items():
            bykind[k][cat] += n
        bykind[k]["_functions"] += 1
    # unsupported
    sites = Counter()
    site_fn = defaultdict(set)
    for c in cases:
        if c["category"] == "lift_unsupported":
            a = int(re.search(r"0x([0-9a-f]+)", c["detail"]["msg"]).group(1), 16)
            sites[a] += 1
            site_fn[a].add(c["fn"])
    want = {"L_%x:" % a: a for a in sites}
    site_mn = {}
    with open(gen) as fh:
        for line in fh:
            if line.startswith("L_"):
                k = line[:line.find(":") + 1]
                if k in want:
                    site_mn[want[k]] = re.search(r"/\* 0x[0-9a-f]+ ([A-Za-z0-9_]+) \*/", line).group(1)
    reasons = {}
    sec = False
    for ln in open(stdout):
        if ln.startswith("unsupported "):
            sec = True
            continue
        if sec:
            m = re.match(r"\s+(\d+)\s+([A-Za-z0-9_]+): (.*)", ln)
            if m:
                reasons.setdefault(m.group(2), m.group(3).strip())
    tab = defaultdict(lambda: {"cases": 0, "sites": 0, "functions": set(), "site_list": []})
    for a, n in sites.items():
        m = site_mn.get(a, "?")
        t = tab[m]
        t["cases"] += n
        t["sites"] += 1
        t["functions"] |= site_fn[a]
        t["site_list"].append(["0x%x" % a, n, sorted(site_fn[a])])
    uns = {}
    for m, t in sorted(tab.items(), key=lambda kv: -kv[1]["cases"]):
        uns[m] = {"class": klass(m), "cases": t["cases"], "sites": t["sites"], "functions": len(t["functions"]),
                  "lifter_reason": reasons.get(m, "(no entry in the stdout unsupported section)"),
                  "site_list": sorted(t["site_list"], key=lambda s: -s[1])}
    byclass = Counter()
    fclass = defaultdict(set)
    for m, u in uns.items():
        byclass[u["class"]] += u["cases"]
    for a, fs in site_fn.items():
        fclass[klass(site_mn.get(a, "?"))] |= fs
    res = {"composition": comp, "d0075_mnemonic_functions": dict(mn.most_common()),
           "cases_by_function_kind": {k: dict(v) for k, v in bykind.items()},
           "lift_unsupported": {"cases": sum(sites.values()), "sites": len(sites), "functions": len({f for s in site_fn.values() for f in s}),
                                "cases_by_class": dict(byclass), "functions_by_class": {k: len(v) for k, v in fclass.items()},
                                "by_mnemonic": uns}}
    with open(out, "x") as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps(comp, indent=1))
    print("by function kind:", json.dumps({k: dict(v) for k, v in bykind.items()}))
    lu = res["lift_unsupported"]
    print("lift_unsupported cases %d sites %d functions %d; by class %s; functions by class %s" % (
        lu["cases"], lu["sites"], lu["functions"], lu["cases_by_class"], lu["functions_by_class"]))
    for m, u in list(uns.items()):
        print("  %-16s %-6s cases %4d sites %3d functions %3d  %s" % (m, u["class"], u["cases"], u["sites"], u["functions"], u["lifter_reason"][:100]))


if __name__ == "__main__":
    main()
