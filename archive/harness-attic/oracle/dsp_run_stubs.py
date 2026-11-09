#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: the CPU-dispatch stubs of decision 0079, native vs lift, selector variable fixed.

The stub is `movsxd rax,[rip+V]; lea r10,[rip+T]; jmp [r10+rax*8]` (V = selector variable, initial 0xffffffff in the file,
T = a table of 6, 7 or 9 code pointers). The stub entry is a lifted function but NOT a .pdata start (the .pdata function
starts 16 bytes earlier with the initialisation `call`, which reaches cpuid), so the stub address is run directly.
For every selector value k and every stub whose k-th table entry is itself in the plain pure set (select_pure.py), the stub
is called with inputs generated for that entry (run_diff2.gen_cases with the entry's pointer info). Both drivers patch V := k
in the mapped image before the cases run (ORACLE_PATCH, XOR of the 4 bytes of 0xffffffff), so native executes the real
`jmp [r10+rax*8]` on the real table and lift executes the generated `switch`.
Negative control (--negctl N pairs): the lift side is run with the selector k2 = (k+1) % 6 instead of k (the table entry
then differs) and compared with the native result of k; a comparison that cannot see a wrong table entry would show 0.
usage: dsp_run_stubs.py PE BINDIR STUBS.json PURE.json ARGPTR.json OUTDIR [--n 16] [--seed 1] [--jobs 12] [--negctl 100]
OUTDIR must not exist.
"""
import argparse
import json
import os
import random
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run_diff2 as rd  # noqa: E402


def patch_spec(var, k):
    return ",".join("%x:%x" % (var + i, (0xFF ^ k) if i == 0 else 0xFF) for i in range(4))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pe")
    ap.add_argument("bindir")
    ap.add_argument("stubs")
    ap.add_argument("pure")
    ap.add_argument("argptr")
    ap.add_argument("out")
    ap.add_argument("--n", type=int, default=16)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--negctl", type=int, default=100)
    a = ap.parse_args()
    os.makedirs(a.out)
    work = os.path.join(a.out, "work")
    os.makedirs(work)
    st = json.load(open(a.stubs))
    assert st["sel_init_values"] == [0xFFFFFFFF]
    pure = {f["va"] for f in json.load(open(a.pure))["functions"]}
    ptrinfo = json.load(open(a.argptr))["functions"]
    nat_bin = os.path.join(a.bindir, "native_drv2")
    lift_bin = os.path.join(a.bindir, "lift_drv2")
    base_env = dict(os.environ)

    groups = defaultdict(list)
    pairs_total = 0
    for s in st["stubs"]:
        for k, (_slot, tgt) in enumerate(s["cases"]):
            pairs_total += 1
            if "0x%x" % tgt in pure:
                groups[(s["sel_var"], k)].append((s, tgt))
    rows, cases_by_id, nat_all, lif_all = [], {}, {}, {}
    cnt = Counter()
    for (var, k), items in sorted(groups.items()):
        cases = []
        for s, tgt in items:
            for c in rd.gen_cases(s["fn"], a.n, a.seed, ptrinfo["0x%x" % tgt]):
                c["id"] = "%x.k%d.%d" % (s["fn"], k, int(c["id"].split(".")[1]))
                c["target"] = tgt
                cases.append(c)
        for c in cases:
            cases_by_id[c["id"]] = c
        env = dict(base_env)
        env["ORACLE_PATCH"] = patch_spec(var, k)
        tag = "v%x-k%d" % (var, k)
        chunks = [cases[i::a.jobs] for i in range(a.jobs)]
        nat, _ = rd.run_side(nat_bin, a.pe, chunks, work, "nat-" + tag, env)
        lif, _ = rd.run_side(lift_bin, a.pe, chunks, work, "lift-" + tag, env)
        retry = [c["id"] for c in cases if (nat.get(c["id"]) or {}).get("status") == "timeout" or
                 (lif.get(c["id"]) or {}).get("status") == "timeout"]
        if retry:
            e10 = dict(env)
            e10["ORACLE_TIMEOUT"] = "10"
            rc = [cases_by_id[i] for i in retry]
            rch = [rc[i::8] for i in range(8)]
            n_r, _ = rd.run_side(nat_bin, a.pe, rch, work, "natr-" + tag, e10)
            l_r, _ = rd.run_side(lift_bin, a.pe, rch, work, "liftr-" + tag, e10)
            nat.update(n_r)
            lif.update(l_r)
        nat_all.update(nat)
        lif_all.update(lif)
        for c in cases:
            cat, det = rd.classify(nat.get(c["id"]), lif.get(c["id"]), Counter())
            rows.append({"stub": "0x%x" % c["fn"], "k": k, "var": "0x%x" % var, "target": "0x%x" % c["target"], "id": c["id"],
                         "category": cat, "detail": det, "retried": c["id"] in retry})
            cnt[cat] += 1
        print("group var %#x k %d: stubs %d cases %d -> %s" % (var, k, len(items), len(cases),
              dict(Counter(r["category"] for r in rows[-len(cases):]))), file=sys.stderr, flush=True)

    # negative control: lift with the selector k2 = (k+1)%6 against the native result of k
    neg = {"pairs": 0, "cases_compared": 0, "detected": 0, "target_same": 0, "lift_not_ok": 0, "by_pair": []}
    if a.negctl:
        r = random.Random(a.seed + 99)
        cand = []
        for (var, k), items in groups.items():
            k2 = (k + 1) % 6
            for s, tgt in items:
                if s["cases"][k2][1] != tgt:
                    cand.append((var, k, k2, s, tgt))
        r.shuffle(cand)
        pick = cand[: a.negctl]
        bygrp = defaultdict(list)
        for p in pick:
            bygrp[(p[0], p[1], p[2])].append(p)
        for (var, k, k2), ps in sorted(bygrp.items()):
            cases = [c for (_v, _k, _k2, s, _t) in ps for c in cases_by_id.values()
                     if c["fn"] == s["fn"] and c["id"].split(".")[1] == "k%d" % k]
            env = dict(base_env)
            env["ORACLE_PATCH"] = patch_spec(var, k2)
            chunks = [cases[i::a.jobs] for i in range(a.jobs)]
            lif2, _ = rd.run_side(lift_bin, a.pe, chunks, work, "negl-v%x-k%d-%d" % (var, k, k2), env)
            for (_v, _k, _k2, s, tgt) in ps:
                mine = [c for c in cases if c["fn"] == s["fn"]]
                det = cmpd = nok = 0
                for c in mine:
                    n0 = nat_all.get(c["id"])
                    l2 = lif2.get(c["id"])
                    if not n0 or n0["status"] != "ok" or not l2:
                        continue
                    if l2["status"] != "ok":
                        nok += 1
                        det += 1
                        cmpd += 1
                        continue
                    core, aux, _ = rd.compare(n0, l2, Counter())
                    core = [x for x in core]
                    aux = [x for x in aux if x != "mxcsr_sticky"]
                    cmpd += 1
                    det += bool(core or aux)
                neg["pairs"] += 1
                neg["cases_compared"] += cmpd
                neg["detected"] += det
                neg["lift_not_ok"] += nok
                neg["by_pair"].append({"stub": "0x%x" % s["fn"], "var": "0x%x" % var, "k": k, "k2": k2,
                                       "target": "0x%x" % tgt, "target_k2": "0x%x" % s["cases"][k2][1],
                                       "compared": cmpd, "detected": det})
        neg["pairs_with_any_detection"] = sum(1 for p in neg["by_pair"] if p["detected"])

    bycat_k = defaultdict(Counter)
    for r in rows:
        bycat_k[r["k"]][r["category"]] += 1
    stubs_ok = defaultdict(Counter)
    for r in rows:
        stubs_ok[r["stub"]][r["category"]] += 1
    verdict = Counter()
    for s, c in stubs_ok.items():
        verdict["has_mismatch" if c.get("mismatch_core") or c.get("mismatch_aux") else "bit_exact_some" if c.get("bit_exact") else "none_comparable"] += 1
    summary = {"pe": a.pe, "stubs_total": len(st["stubs"]), "stub_k_pairs_total": pairs_total,
               "stub_k_pairs_target_pure": sum(len(v) for v in groups.values()), "groups": len(groups),
               "cases_total": len(rows), "categories": dict(cnt),
               "by_k": {str(k): dict(v) for k, v in sorted(bycat_k.items())},
               "stub_verdicts(stubs with >=1 pair run)": dict(verdict), "stubs_with_pair_run": len(stubs_ok),
               "stubs_with_bit_exact_case": sum(1 for c in stubs_ok.values() if c.get("bit_exact")),
               "pairs_with_bit_exact_case": len({(r["stub"], r["k"]) for r in rows if r["category"] == "bit_exact"}),
               "mismatch_fields": dict(Counter(f for r in rows if r["category"].startswith("mismatch")
                                               for f in r["detail"]["fields"])),
               "lift_unsupported_messages": dict(Counter(r["detail"]["msg"] for r in rows if r["category"] == "lift_unsupported").most_common(30)),
               "negative_control_wrong_table_entry": {k: v for k, v in neg.items() if k != "by_pair"}}
    for name, obj in (("stubs-summary.json", summary), ("stubs-cases.json", rows), ("stubs-negctl.json", neg),
                      ("stubs-mismatches.json", [r for r in rows if r["category"].startswith("mismatch")])):
        with open(os.path.join(a.out, name), "x") as fh:
            json.dump(obj, fh, indent=1)
    print(json.dumps({k: v for k, v in summary.items() if k != "lift_unsupported_messages"}, indent=1))


if __name__ == "__main__":
    main()
