#!/usr/bin/env python3
"""A5 oracle, a5f: add repaired inputs for functions whose native run faults, and judge them like every other case.

For each selected function the same 16 cases run_diff2.gen_cases(fn, 16, seed) makes are regenerated
(seed = the seed of the run in which the function had no comparable case). Each case is run natively
(single-stepped, native_drv2 ORACLE_RING_DIR); a faulting one is repaired by a5f_repair.analyse and run again,
up to --rounds repairs. The cases that end with a native return are new cases (id "<orig id>r"; a case that
needed no repair is an original case and keeps its id). They then go through exactly the checks of
run_diff2.main: native, lift (identical case file), native single-step trace (coverage and consistency),
timeout re-run at 10 s, run_diff2.classify. run_diff2.py itself is not changed and not run here.

usage: a5f_run.py --pe PE --argptr argptr.json --pure pure.json --seedmap seedmap.json --funcs FILE --out DIR
                  [--rounds 40] [--jobs 16] [--limit N] [--negctl N]
  FILE: one function VA (0x...) per line (a leading header line and extra columns are ignored)
  seedmap.json: {"0x...": [[seed, verdict, source], ...]}; the first entry with a native_fault_all verdict
  is used (--seed N overrides for every function)
"""
import argparse
import gzip
import json
import os
import random
import subprocess
import sys
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import a5f_repair as ar  # noqa: E402
import run_diff2 as rd  # noqa: E402

BIN = rd.BIN


def read_funcs(path):
    out = []
    for line in open(path):
        t = line.split()
        if t and t[0].startswith("0x"):
            out.append(t[0])
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pe", required=True)
    ap.add_argument("--argptr", required=True)
    ap.add_argument("--pure", required=True)
    ap.add_argument("--seedmap", default="")
    ap.add_argument("--funcs", default="")
    ap.add_argument("--faulting-from", default="", help="a5-cases.json of a run_diff2 run: repair only its native_fault cases "
                    "(needs --seed = the seed of that run; --funcs is then ignored)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--rounds", type=int, default=40)
    ap.add_argument("--jobs", type=int, default=16)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--n", type=int, default=16)
    ap.add_argument("--negctl", type=int, default=0)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    work = os.path.join(a.out, "work")
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ)
    env["TMPDIR"] = os.path.join(rd.LIFT, "out-oracle", "tmp")
    env_tr = dict(env)
    env_tr["ORACLE_TRACE"] = "1"

    pure = {f["va"]: f for f in json.load(open(a.pure))["functions"]}
    ptrinfo = json.load(open(a.argptr))["functions"]
    seedmap = json.load(open(a.seedmap)) if a.seedmap else {}
    only_ids = None
    if a.faulting_from:
        orig_rows = json.load(open(a.faulting_from))
        only_ids = {r["id"] for r in orig_rows if r["category"] == "native_fault"}
        fns = sorted({r["fn"] for r in orig_rows if r["category"] == "native_fault"}, key=lambda x: int(x, 16))
    else:
        fns = read_funcs(a.funcs)
    if a.limit:
        fns = fns[: a.limit]
    img = rd.Img(a.pe)

    base = []
    for f in fns:
        base += [c for c in rd.gen_cases(int(f, 16), a.n, seedmap_seed(seedmap, f, a.seed), ptrinfo[f])
                 if only_ids is None or c["id"] in only_ids]
    byid0 = {c["id"]: c for c in base}
    print("functions %d base cases %d" % (len(fns), len(base)), file=sys.stderr)

    final, report = ar.repair_loop(img, base, a.pe, BIN, work, a.jobs, a.rounds, env, seed=1,
                                   log=lambda m: print(m, file=sys.stderr))
    cases = []
    origin = {}
    for cid, c in final.items():
        repaired = bool(report[cid]["steps"])
        nid = cid + "r" if repaired else cid
        nc = dict(c)
        nc["id"] = nid
        cases.append(nc)
        origin[nid] = {"orig": cid, "repaired": repaired, "rounds": report[cid]["rounds"]}
    cases.sort(key=lambda c: (c["fn"], c["id"]))
    byid = {c["id"]: c for c in cases}
    print("cases that return natively: %d (repaired %d)" % (len(cases), sum(1 for o in origin.values() if o["repaired"])),
          file=sys.stderr)

    chunks = [cases[i :: a.jobs] for i in range(a.jobs)]
    nat, rcn = rd.run_side(os.path.join(BIN, "native_drv2"), a.pe, chunks, work, "nat", env)
    lif, rcl = rd.run_side(os.path.join(BIN, "lift_drv2"), a.pe, chunks, work, "lift", env)
    ntr, rct = rd.run_side(os.path.join(BIN, "native_drv2"), a.pe, chunks, work, "ntr", env_tr)
    print("driver exit codes native %s lift %s native-trace %s" % (sorted(set(rcn)), sorted(set(rcl)), sorted(set(rct))),
          file=sys.stderr)
    retry_ids = [c["id"] for c in cases if (nat.get(c["id"]) or {}).get("status") == "timeout" or
                 (lif.get(c["id"]) or {}).get("status") == "timeout"]
    retry_stat = {"retried": len(retry_ids)}
    if retry_ids:
        e10 = dict(env)
        e10["ORACLE_TIMEOUT"] = "10"
        rch = [[byid[i] for i in retry_ids][x::8] for x in range(8)]
        n_r, _ = rd.run_side(os.path.join(BIN, "native_drv2"), a.pe, rch, work, "natr", e10)
        l_r, _ = rd.run_side(os.path.join(BIN, "lift_drv2"), a.pe, rch, work, "liftr", e10)
        nat.update(n_r)
        lif.update(l_r)
        retry_stat["native_after"] = dict(Counter((nat.get(i) or {}).get("status") for i in retry_ids))
        retry_stat["lift_after"] = dict(Counter((lif.get(i) or {}).get("status") for i in retry_ids))

    st = Counter()
    rows = []
    for c in cases:
        cat, det = rd.classify(nat.get(c["id"]), lif.get(c["id"]), st)
        o = origin[c["id"]]
        row = {"fn": "0x%x" % c["fn"], "id": c["id"], "orig": o["orig"], "repaired": o["repaired"], "rounds": o["rounds"],
               "category": cat, "detail": det}
        if cat == "native_fault":
            cause, extra = rd.fault_cause(img, c, nat[c["id"]], c["fn"]) if nat.get(c["id"]) else ("native_missing", "")
            row["fault_cause"] = cause
        rows.append(row)

    tr_cmp = Counter()
    for c in cases:
        n1, n2 = nat.get(c["id"]), ntr.get(c["id"])
        if not n1 or not n2:
            tr_cmp["missing"] += 1
        elif n1["status"] != n2["status"]:
            tr_cmp["status_differs:%s->%s" % (n1["status"], n2["status"])] += 1
        elif n1["status"] == "ok":
            core, aux, _ = rd.compare(n1, n2, Counter())
            tr_cmp["equal" if not core and not aux else "differs"] += 1
        else:
            tr_cmp["both_" + n1["status"]] += 1

    exec_by_fn = defaultdict(set)
    per_case_cov = defaultdict(list)
    for c in cases:
        t = ntr.get(c["id"])
        if t and t["status"] == "ok" and "trace" in t:
            va = c["fn"]
            end = int(pure["0x%x" % va]["end"], 16)
            own = {r for r in t["trace"]["rips"] if va <= r < end}
            exec_by_fn[va].update(own)
            per_case_cov[va].append(len(own) / max(1, pure["0x%x" % va]["insns"]))

    per_fn = defaultdict(Counter)
    for r in rows:
        per_fn[r["fn"]][r["category"]] += 1
    fn_rows = []
    verdict = Counter()
    for f in fns:
        cc = per_fn[f]
        if cc.get("mismatch_core"):
            v = "has_mismatch_core"
        elif cc.get("mismatch_aux"):
            v = "has_mismatch_aux_only"
        elif cc.get("bit_exact"):
            v = "bit_exact_all_comparable"
        elif cc.get("lift_unsupported") or cc.get("lift_stop_other"):
            v = "lift_not_executable"
        elif sum(cc.values()):
            v = "native_fault_all"
        else:
            v = "no_case_returns_natively"
        verdict[v] += 1
        va = int(f, 16)
        fn_rows.append({"fn": f, "seed": seedmap_seed(seedmap, f, a.seed), "verdict": v, "cases": dict(cc),
                        "base_cases_returning_natively": sum(1 for r in rows if r["fn"] == f),
                        "own_insns_union_executed": len(exec_by_fn.get(va, ())) / max(1, pure[f]["insns"]),
                        "insns": pure[f]["insns"]})
    cnt = Counter(r["category"] for r in rows)
    rep_cnt = Counter(r["category"] for r in rows if r["repaired"])
    res_cnt = Counter(v["result"].split(":")[0] if not v["result"].startswith("unrepairable") else v["result"]
                      for v in report.values())
    covs = sorted(x for v in per_case_cov.values() for x in v)
    summary = {
        "functions": len(fns), "base_cases": len(base), "cases_returning_natively_after_repair": len(cases),
        "repair_result_of_base_cases": dict(res_cnt.most_common()),
        "categories": dict(cnt), "categories_repaired_cases": dict(rep_cnt),
        "function_verdicts": dict(verdict), "trace_consistency": dict(tr_cmp), "timeout_retry": retry_stat,
        "coverage_own_insns_executed_per_case": {"cases": len(covs), "mean": sum(covs) / max(1, len(covs)),
                                                 "median": covs[len(covs) // 2] if covs else None},
        "repair_rounds_histogram": dict(sorted(Counter(o["rounds"] for o in origin.values()).items())),
        "mismatch_fields": dict(Counter(f for r in rows if r["category"] in ("mismatch_core", "mismatch_aux")
                                        for f in r["detail"]["fields"])),
        "flag_stats": {k: v for k, v in sorted(st.items()) if k.startswith("flag_defined_diff")},
    }
    for name, obj in (("a5f-summary.json", summary), ("a5f-functions.json", fn_rows), ("a5f-cases.json", rows),
                      ("a5f-mismatches.json", [r for r in rows if r["category"] in ("mismatch_core", "mismatch_aux",
                                                                                   "lift_unsupported", "lift_stop_other")])):
        with open(os.path.join(a.out, name), "w") as fh:
            json.dump(obj, fh, indent=1)
    with gzip.open(os.path.join(a.out, "a5f-repair-report.json.gz"), "wt") as fh:
        json.dump(ar.trim_report(report), fh)
    with open(os.path.join(a.out, "a5f-cases-input.json"), "w") as fh:
        json.dump({c["id"]: rd.case_line(c) for c in cases}, fh)
    print(json.dumps({k: summary[k] for k in ("functions", "base_cases", "cases_returning_natively_after_repair",
                                              "repair_result_of_base_cases", "categories", "function_verdicts",
                                              "trace_consistency", "mismatch_fields")}, indent=1))

    if a.negctl:
        negctl(a, cases, rows, nat, lif, ntr, work, env_tr, img, pure, ptrinfo)


def seedmap_seed(seedmap, f, override):
    if override:
        return override
    return [s for s, v, src in seedmap[f] if v == "native_fault_all" and src != "seed3plain"][0]


def negctl(a, cases, rows, nat, lif, ntr, work, env_tr, img, pure, ptrinfo):
    """executed-byte negative control on the repaired cases, same method as run_diff2.main: one byte of an
    executed instruction XOR 1 on the native side (ORACLE_PATCH), compare with the unpatched lift result"""
    import struct
    byid = {c["id"]: c for c in cases}
    basis = defaultdict(list)
    for r in rows:
        if r["category"] == "bit_exact":
            t = ntr.get(r["id"])
            if t and t["status"] == "ok" and "trace" in t:
                basis[int(r["fn"], 16)].append(r["id"])
    rr = random.Random(a.seed + 777)
    pool = sorted(basis)
    rr.shuffle(pool)
    controls = []
    for fva in pool:
        if len(controls) >= a.negctl:
            break
        execd = set()
        for cid in basis[fva]:
            execd.update(ntr[cid]["trace"]["rips"])
        cand = []
        for rip in sorted(execd):
            ins = img.one(rip)
            if ins is None:
                continue
            for o, role in rd.opcode_imm_disp_bytes(ins):
                cand.append((rip, o, role, ins))
        if not cand:
            continue
        rip, o, role, ins = rr.choice(cand)
        controls.append({"fn": fva, "patch_va": rip + o, "insn_va": rip, "role": role,
                         "insn": "%s %s" % (ins.mnemonic, ins.op_str), "bytes": ins.bytes.hex(), "byte_index": o})

    def run_control(idx):
        ci = controls[idx]
        cs = [byid[x] for x in basis[ci["fn"]]]
        cf = os.path.join(work, "cases-neg-%03d.txt" % idx)
        of = os.path.join(work, "out-neg-%03d.jsonl" % idx)
        with open(cf, "w") as fh:
            for c in cs:
                fh.write(rd.case_line(c) + "\n")
        e = dict(env_tr)
        e["ORACLE_PATCH"] = "%x:1" % ci["patch_va"]
        subprocess.run([os.path.join(BIN, "native_drv2"), cf, of, a.pe], stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, env=e)
        got = {}
        if os.path.exists(of):
            with open(of) as fh:
                for line in fh:
                    if line.strip():
                        o = json.loads(line)
                        got[o["id"]] = o
        res = Counter()
        sink = Counter()
        for c in cs:
            cid = c["id"]
            m, l, n0, t0 = got.get(cid), lif.get(cid), nat.get(cid), ntr.get(cid)
            if not m or not l or not n0 or n0["status"] != "ok" or l["status"] != "ok":
                res["no_baseline"] += 1
                continue
            if m["status"] != "ok":
                res["fault_diff:" + m["status"]] += 1
                continue
            core, aux, _ = rd.compare(m, l, sink)
            if "mxcsr_sticky" in aux:
                aux.remove("mxcsr_sticky")
            if core:
                res["core"] += 1
            elif aux:
                res["aux"] += 1
            else:
                res["equal"] += 1
                if t0 and t0["status"] == "ok" and "trace" in m and (m["trace"]["digest"] != t0["trace"]["digest"] or
                                                                      m["trace"]["steps"] != t0["trace"]["steps"]):
                    res["trace_differs"] += 1
        out = dict(ci)
        out["patch_va"] = "0x%x" % ci["patch_va"]
        out["insn_va"] = "0x%x" % ci["insn_va"]
        out["fn"] = "0x%x" % ci["fn"]
        out["cases"] = dict(res)
        faultn = sum(v for k, v in res.items() if k.startswith("fault_diff"))
        if res["core"]:
            out["outcome"] = "detected_core"
        elif res["aux"]:
            out["outcome"] = "detected_aux_only"
        elif faultn:
            out["outcome"] = "detected_by_native_fault_only"
        elif res["trace_differs"]:
            out["outcome"] = "undetected_state_changed_but_masked"
        else:
            out["outcome"] = "undetected_equivalent_no_state_change"
        return out

    with ThreadPoolExecutor(a.jobs) as ex:
        ctl = list(ex.map(run_control, range(len(controls))))
    oc = Counter(c["outcome"] for c in ctl)
    neg = {"controls_total": len(ctl), "outcomes": dict(oc),
           "detected_output_difference": oc["detected_core"] + oc["detected_aux_only"],
           "detected_incl_native_fault": oc["detected_core"] + oc["detected_aux_only"] + oc["detected_by_native_fault_only"],
           "controls": ctl}
    with open(os.path.join(a.out, "a5f-negctl.json"), "w") as fh:
        json.dump(neg, fh, indent=1)
    print("negative control on repaired cases:", json.dumps({k: v for k, v in neg.items() if k != "controls"}))


if __name__ == "__main__":
    main()
