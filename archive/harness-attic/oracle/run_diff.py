#!/usr/bin/env python3
"""A5 oracle: function-level bit-exact differential run, native machine code vs lifted C.

For each selected function: N cases (registers, scratch memory fill, stack arguments). Both
drivers get the same case file. Compared per case: all 16 GPRs, XMM0-15 (128 bit each), and the
8-byte-block diff of three regions against their pre-call content (writable image hull,
scratch 0x20000000 + 1 MiB, stack). The verdict is bit-exact only.

Per-case classification
  bit_exact         native returned, lift returned, everything equal
  mismatch          native returned, lift returned and something differs; or native returned
                    and lift faulted / timed out
  lift_unsupported  native returned, lift stopped on fle_unsupported
  lift_stop_other   native returned, lift stopped on another runtime stop (undefined flag,
                    misaligned access, ...) or has no entry for the VA
  native_fault      native did not return (SIGSEGV/SIGBUS/SIGILL/SIGFPE/timeout): not comparable

Negative control: the native driver runs a copy of the PE with one byte XOR 0x01, the lift
driver runs the generated C of the original. A control is detected when at least one case
reports mismatch.
usage: run_diff.py --pe PE --pure pure.json --out DIR [--count 200] [--n 12] [--seed 1]
       [--negctl 24] [--jobs 16]
"""
import argparse
import csv
import json
import os
import random
import shutil
import struct
import subprocess
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
LIFT = os.path.abspath(os.path.join(HERE, "..", ".."))
BIN = os.path.join(LIFT, "out-oracle", "bin")
SCRATCH = 0x20000000
M64 = (1 << 64) - 1

GPR = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"]

BOUNDARY = [0, 1, 2, 3, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF, 0x10000, 0x7FFFFFFF, 0x80000000,
            0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, M64, M64 - 1]
FP64 = [0, 0x8000000000000000, 0x3FF0000000000000, 0xBFF0000000000000, 0x4000000000000000, 0x7FF0000000000000,
        0xFFF0000000000000, 0x7FF8000000000000, 0x7FF4000000000000, 0x0000000000000001, 0x000FFFFFFFFFFFFF,
        0x7FEFFFFFFFFFFFFF, 0x3FE0000000000000, 0x4059000000000000, 0xC059000000000000]
FP32 = [0, 0x80000000, 0x3F800000, 0xBF800000, 0x7F800000, 0xFF800000, 0x7FC00000, 0x00000001, 0x007FFFFF,
        0x7F7FFFFF, 0x3F000000, 0x42C80000]


def ptr(r):
    return SCRATCH + 0x1000 + (r.randrange(0, 0x70000) & ~7)


def gen_cases(fn_va, n, seed):
    r = random.Random((seed << 32) ^ fn_va)
    cases = []
    for k in range(n):
        gpr = [r.getrandbits(64) for _ in range(16)]
        xmm = [[r.getrandbits(64), r.getrandbits(64)] for _ in range(16)]
        fill = 0
        if k == 0:
            fill = 2
            for i in (1, 2, 8, 9):
                gpr[i] = ptr(r)
        elif k == 1:
            fill = 3
            for i in (1, 2, 8, 9):
                gpr[i] = ptr(r)
        elif k == 2:
            fill = 4
            for i in (1, 2, 8, 9):
                gpr[i] = ptr(r)
        elif k == 3:
            fill = 2
            for i in (1, 2, 8, 9):
                gpr[i] = 0
        elif k in (4, 5, 6):
            fill = 1
            gpr[1], gpr[2] = ptr(r), ptr(r)
            gpr[8] = r.choice([0, 1, 4, 8, 16, 17, 32, 64, r.randrange(0, 256)])
            gpr[9] = r.choice([ptr(r), r.choice(BOUNDARY)])
        elif k == 7:
            fill = 1
            for i in (1, 2, 8, 9):
                gpr[i] = ptr(r)
        elif k == 8:
            fill = 3
            for i in (1, 2, 8, 9):
                gpr[i] = M64
        elif k == 11:
            for i, v in zip((1, 2, 8, 9), (0x7FFFFFFF, 0x80000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000)):
                gpr[i] = v
        elif k == 9:
            for j in range(16):
                xmm[j] = [r.choice(FP64), r.choice(FP64)]
            fill = 1
            gpr[1], gpr[2] = ptr(r), ptr(r)
        elif k == 10:
            for j in range(16):
                xmm[j] = [r.choice(FP32) | (r.choice(FP32) << 32), r.choice(FP32) | (r.choice(FP32) << 32)]
            gpr[1], gpr[2] = ptr(r), ptr(r)
        elif k >= 12:
            for i in (1, 2, 8, 9):
                gpr[i] = r.choice([ptr(r), r.choice(BOUNDARY), r.getrandbits(64), r.getrandbits(8)])
            fill = r.choice([0, 1])
        stk = [r.choice([ptr(r), r.choice(BOUNDARY), r.getrandbits(64), r.getrandbits(5)]) for _ in range(4)]
        if k == 3:
            stk = [0] * 4
        elif k == 8:
            stk = [M64] * 4
        cases.append({"id": "%x.%d" % (fn_va, k), "fn": fn_va, "fill": fill,
                      "seed": r.getrandbits(48), "stk": stk, "gpr": gpr, "xmm": xmm})
    return cases


def case_line(c):
    parts = ["CASE", c["id"], "%x" % c["fn"], str(c["fill"]), "%x" % c["seed"], str(len(c["stk"]))]
    parts += ["%x" % v for v in c["stk"]]
    parts += ["%x" % v for v in c["gpr"]]
    for lo, hi in c["xmm"]:
        parts += ["%x" % lo, "%x" % hi]
    return " ".join(parts)


def run_side(binary, pe, chunks, outdir, tag, env):
    procs = []
    outs = []
    for i, ch in enumerate(chunks):
        cf = os.path.join(outdir, "cases-%s-%02d.txt" % (tag, i))
        of = os.path.join(outdir, "out-%s-%02d.jsonl" % (tag, i))
        with open(cf, "w") as fh:
            for c in ch:
                fh.write(case_line(c) + "\n")
        lf = open(os.path.join(outdir, "log-%s-%02d.txt" % (tag, i)), "w")
        procs.append((subprocess.Popen([binary, cf, of, pe], stdout=lf, stderr=subprocess.STDOUT, env=env), lf))
        outs.append(of)
    rc = []
    for p, lf in procs:
        rc.append(p.wait())
        lf.close()
    res = {}
    for of in outs:
        if not os.path.exists(of):
            continue
        with open(of) as fh:
            for line in fh:
                line = line.strip()
                if line:
                    o = json.loads(line)
                    res[o["id"]] = o
    return res, rc


def classify(nat, lif):
    if nat is None or nat["status"] != "ok":
        ns = nat["status"] if nat else "missing"
        same = bool(nat and lif and nat["status"] == "fault" and lif["status"] == "fault" and
                    nat.get("fault_addr") == lif.get("fault_addr"))
        return "native_fault", {"native": ns, "native_msg": (nat or {}).get("msg", ""),
                                "lift": lif["status"] if lif else "missing", "same_fault_addr": same}
    if lif is None:
        return "lift_stop_other", {"reason": "missing"}
    if lif["status"] == "noentry":
        return "lift_stop_other", {"reason": "noentry", "msg": "no entry for the VA in fle_dispatch_table"}
    if lif["status"] == "abort":
        msg = lif.get("msg", "")
        if "unsupported instruction" in msg:
            return "lift_unsupported", {"reason": "unsupported", "msg": msg.strip()}
        return "lift_stop_other", {"reason": "abort", "msg": msg.strip()}
    if lif["status"] != "ok":
        return "mismatch", {"fields": ["lift_" + lif["status"]], "lift_msg": lif.get("msg", ""),
                            "lift_fault_addr": lif.get("fault_addr")}
    fields = []
    for i in range(16):
        if nat["gpr"][i] != lif["gpr"][i]:
            fields.append(GPR[i])
    for i in range(16):
        if nat["xmm"][2 * i] != lif["xmm"][2 * i] or nat["xmm"][2 * i + 1] != lif["xmm"][2 * i + 1]:
            fields.append("xmm%d" % i)
    memd = {}
    for reg in ("image", "scratch", "stack"):
        a, b = nat["mem"][reg], lif["mem"][reg]
        if a["n"] != b["n"] or a["hash"] != b["hash"]:
            fields.append("mem:" + reg)
            na = {x[0]: x for x in a["blocks"]}
            nb = {x[0]: x for x in b["blocks"]}
            first = []
            for addr in sorted(set(na) | set(nb)):
                if na.get(addr) != nb.get(addr):
                    first.append({"addr": addr, "native": na.get(addr), "lift": nb.get(addr)})
                if len(first) >= 4:
                    break
            memd[reg] = {"native_n": a["n"], "lift_n": b["n"], "first": first}
    if fields:
        return "mismatch", {"fields": fields, "mem": memd}
    return "bit_exact", {"wrote_blocks": sum(nat["mem"][r]["n"] for r in ("image", "scratch")),
                         "wrote_stack_blocks": nat["mem"]["stack"]["n"]}


def flip_pe(src, dst, off, bit=0x01):
    shutil.copyfile(src, dst)
    with open(dst, "r+b") as fh:
        fh.seek(off)
        b = fh.read(1)
        fh.seek(off)
        fh.write(bytes([b[0] ^ bit]))


def file_off_of_va(pe_bytes, va):
    lf = struct.unpack_from("<I", pe_bytes, 0x3C)[0]
    nsec = struct.unpack_from("<H", pe_bytes, lf + 6)[0]
    osz = struct.unpack_from("<H", pe_bytes, lf + 20)[0]
    base = struct.unpack_from("<Q", pe_bytes, lf + 24 + 24)[0]
    rva = va - base
    for i in range(nsec):
        s = lf + 24 + osz + 40 * i
        vs, sva, rs, rp = struct.unpack_from("<IIII", pe_bytes, s + 8)
        if sva <= rva < sva + rs:
            return rp + rva - sva
    raise ValueError(hex(va))


def find_const_returners(data, funcs, limit):
    """functions that start with `mov eax, imm32; ret` (b8 imm32 c3). Flipping an imm byte must show in rax."""
    out = []
    for f in funcs:
        off = file_off_of_va(data, int(f["va"], 16))
        if data[off] == 0xB8 and data[off + 5] == 0xC3:
            out.append((f, off + 1))
            if len(out) >= limit:
                break
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pe", required=True)
    ap.add_argument("--pure", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--count", type=int, default=200)
    ap.add_argument("--n", type=int, default=12)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--negctl", type=int, default=24)
    ap.add_argument("--jobs", type=int, default=16)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    work = os.path.join(a.out, "work")
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ)
    env["TMPDIR"] = os.path.join(LIFT, "out-oracle", "tmp")

    pure = json.load(open(a.pure))
    funcs = pure["functions"]
    data = open(a.pe, "rb").read()
    rnd = random.Random(a.seed)
    sel = rnd.sample(funcs, min(a.count, len(funcs)))
    sel.sort(key=lambda f: int(f["va"], 16))
    sel_vas = {f["va"] for f in sel}

    ctl = [(f, off, "const_returner_imm") for f, off in find_const_returners(data, funcs, 3)]
    rr = random.Random(a.seed + 777)

    cases = []
    for f in sel:
        cases += gen_cases(int(f["va"], 16), a.n, a.seed)
    extra = []
    for f, _off, _k in ctl:
        if f["va"] not in sel_vas:
            sel_vas.add(f["va"])
            extra += gen_cases(int(f["va"], 16), a.n, a.seed)
    allc = cases + extra
    chunks = [allc[i::a.jobs] for i in range(a.jobs)]

    nat, rcn = run_side(os.path.join(BIN, "native_drv"), a.pe, chunks, work, "nat", env)
    lif, rcl = run_side(os.path.join(BIN, "lift_drv"), a.pe, chunks, work, "lift", env)
    print("driver exit codes native %s lift %s" % (sorted(set(rcn)), sorted(set(rcl))), file=sys.stderr)

    rows = []
    for c in cases:
        cat, det = classify(nat.get(c["id"]), lif.get(c["id"]))
        rows.append({"fn": "0x%x" % c["fn"], "id": c["id"], "category": cat, "detail": det})

    # random-byte controls: only functions whose cases are all bit_exact in this run, one flipped
    # byte inside the function body. A flip on a path the cases never execute is not detected.
    per_fn_cats = defaultdict(Counter)
    for r in rows:
        per_fn_cats[r["fn"]][r["category"]] += 1
    comparable = {fn for fn, c in per_fn_cats.items() if c.get("bit_exact") == a.n}
    pool = [f for f in sel if f["va"] in comparable]
    rr.shuffle(pool)
    for f in pool:
        if len(ctl) >= a.negctl:
            break
        va, end = int(f["va"], 16), int(f["end"], 16)
        ctl.append((f, file_off_of_va(data, va) + rr.randrange(0, end - va), "random_byte_in_body"))

    neg = {"controls": []}
    flipped = os.path.join(work, "flipped.dll")
    for idx, (f, off, kind) in enumerate(ctl):
        flip_pe(a.pe, flipped, off, 0x01)
        va = int(f["va"], 16)
        ccases = gen_cases(va, a.n, a.seed)
        n2, _ = run_side(os.path.join(BIN, "native_drv"), flipped, [ccases], work, "neg", env)
        cats = Counter()
        for c in ccases:
            cats[classify(n2.get(c["id"]), lif.get(c["id"]))[0]] += 1
        neg["controls"].append({"fn": f["va"], "kind": kind, "file_offset": "0x%x" % off, "xor": "0x01",
                                "cases": dict(cats), "detected": cats.get("mismatch", 0) > 0})
    neg["detected_total"] = sum(1 for c in neg["controls"] if c["detected"])
    neg["controls_total"] = len(neg["controls"])
    neg["const_returner_detected"] = [c["detected"] for c in neg["controls"] if c["kind"] == "const_returner_imm"]

    cnt = Counter(r["category"] for r in rows)
    nf = Counter()
    unsup = Counter()
    other = Counter()
    fieldc = Counter()
    per_fn = defaultdict(Counter)
    for r in rows:
        per_fn[r["fn"]][r["category"]] += 1
        d = r["detail"]
        if r["category"] == "native_fault":
            nf["%s/lift:%s" % (d["native"], d["lift"])] += 1
            if d["same_fault_addr"]:
                nf["same_fault_addr"] += 1
        elif r["category"] == "lift_unsupported":
            unsup[d["msg"]] += 1
        elif r["category"] == "lift_stop_other":
            other[d.get("msg", d.get("reason", ""))] += 1
        elif r["category"] == "mismatch":
            for fl in d["fields"]:
                fieldc[fl] += 1
    fn_verdict = Counter()
    fn_rows = []
    for f in sel:
        c = per_fn[f["va"]]
        if c.get("mismatch"):
            v = "has_mismatch"
        elif c.get("bit_exact"):
            v = "bit_exact_all_comparable"
        elif c.get("lift_unsupported") or c.get("lift_stop_other"):
            v = "lift_not_executable"
        else:
            v = "native_fault_all"
        fn_verdict[v] += 1
        fn_rows.append({"fn": f["va"], "verdict": v, "cases": dict(c)})
    be = [r for r in rows if r["category"] == "bit_exact"]
    summary = {
        "bit_exact_cases_with_image_or_scratch_writes": sum(1 for r in be if r["detail"]["wrote_blocks"] > 0),
        "bit_exact_cases_with_stack_writes_beyond_return_slot": sum(1 for r in be if r["detail"]["wrote_stack_blocks"] > 0),
        "pe": a.pe, "seed": a.seed, "functions_selected": len(sel), "pure_pool": len(funcs),
        "cases_per_function": a.n, "cases_total": len(rows),
        "categories": dict(cnt), "native_fault_breakdown": dict(nf),
        "lift_unsupported_messages": dict(unsup), "lift_stop_other_messages": dict(other.most_common(20)),
        "mismatch_fields": dict(fieldc),
        "function_verdicts": dict(fn_verdict), "negative_control": neg,
    }
    for name, obj in (("a5-summary.json", summary), ("a5-functions.json", fn_rows), ("a5-cases.json", rows)):
        with open(os.path.join(a.out, name), "w") as fh:
            json.dump(obj, fh, indent=1)
    with open(os.path.join(a.out, "a5-cases.csv"), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["fn", "case", "category", "fields_or_reason"])
        for r in rows:
            d = r["detail"]
            w.writerow([r["fn"], r["id"], r["category"],
                        ";".join(d.get("fields", [])) or d.get("reason", "") or d.get("native", "")])
    print(json.dumps({k: summary[k] for k in ("functions_selected", "cases_total", "categories",
                                              "function_verdicts")}, indent=1))
    print("negative control: detected %d / %d (const returners: %s)" %
          (neg["detected_total"], neg["controls_total"], neg["const_returner_detected"]))


if __name__ == "__main__":
    main()
