#!/usr/bin/env python3
"""Which pure functions depend on the value of their return address ([rsp] at entry)?

static : one linear sweep per function, tracking rsp (and an rbp = rsp + c alias) relative to the entry
         rsp; any memory operand that addresses entry_rsp + 0 (the return address slot) is reported.
dynamic: the 16 run_diff2.gen_cases of every function (given seed) run on native_drv2 twice, with the
         return-address stub at 0xdead0000 and at ORACLE_RETADDR=0xbeef0000; a case whose output differs in
         anything but the sentinel-dependent value is reported (status "timeout" pairs are skipped).
usage: a5f_retaddr_scan.py PE pure.json argptr.json OUTDIR [--seed 1] [--chunk 1000] [--jobs 16] [--static-only]
"""
import argparse
import json
import os
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run_diff2 as rd  # noqa: E402
from argptr import CANON  # noqa: E402
from capstone import CS_OP_IMM, CS_OP_MEM, CS_OP_REG  # noqa: E402


def static_scan(img, funcs):
    out = []
    for f in funcs:
        va, end = int(f["va"], 16), int(f["end"], 16)
        d = 0  # rsp - entry_rsp
        bp = None  # rbp - entry_rsp when known
        hits = []
        for ins in img.md.disasm(img.get(va, end - va), va):
            mn, ops = ins.mnemonic, ins.operands
            for op in ops:
                if op.type != CS_OP_MEM:
                    continue
                m = op.mem
                b = CANON.get(img.md.reg_name(m.base)) if m.base else None
                if m.index:
                    continue
                off = None
                if b == "rsp":
                    off = d + m.disp
                elif b == "rbp" and bp is not None:
                    off = bp + m.disp
                if off == 0:
                    hits.append("%x: %s %s" % (ins.address, mn, ins.op_str))
            if mn == "push":
                d -= 8
            elif mn == "pop":
                d += 8
                if ops and ops[0].type == CS_OP_REG and CANON.get(img.md.reg_name(ops[0].reg)) == "rbp":
                    bp = None
            elif mn in ("sub", "add") and ops and ops[0].type == CS_OP_REG and CANON.get(img.md.reg_name(ops[0].reg)) == "rsp" \
                    and ops[1].type == CS_OP_IMM:
                d += -ops[1].imm if mn == "sub" else ops[1].imm
            elif mn == "mov" and len(ops) == 2 and ops[0].type == CS_OP_REG and ops[1].type == CS_OP_REG:
                dn, sn = CANON.get(img.md.reg_name(ops[0].reg)), CANON.get(img.md.reg_name(ops[1].reg))
                if dn == "rbp" and sn == "rsp":
                    bp = d
            elif mn == "lea" and ops[0].type == CS_OP_REG and CANON.get(img.md.reg_name(ops[0].reg)) == "rbp" and \
                    ops[1].mem.base and CANON.get(img.md.reg_name(ops[1].mem.base)) == "rsp" and not ops[1].mem.index:
                bp = d + ops[1].mem.disp
            elif mn == "lea" and ops[0].type == CS_OP_REG and CANON.get(img.md.reg_name(ops[0].reg)) == "rsp":
                d = None
                break
        if hits:
            out.append({"fn": f["va"], "hits": hits[:6]})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pe")
    ap.add_argument("pure")
    ap.add_argument("argptr")
    ap.add_argument("out")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--chunk", type=int, default=1000)
    ap.add_argument("--jobs", type=int, default=16)
    ap.add_argument("--static-only", action="store_true")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    work = os.path.join(a.out, "work")
    os.makedirs(work, exist_ok=True)
    funcs = json.load(open(a.pure))["functions"]
    ptrinfo = json.load(open(a.argptr))["functions"]
    img = rd.Img(a.pe)
    st = static_scan(img, funcs)
    json.dump(st, open(os.path.join(a.out, "retaddr-static.json"), "w"), indent=1)
    print("static: %d of %d functions address entry_rsp+0:" % (len(st), len(funcs)), [x["fn"] for x in st], file=sys.stderr)
    if a.static_only:
        return
    env = dict(os.environ)
    env["TMPDIR"] = os.path.join(rd.LIFT, "out-oracle", "tmp")
    env_b = dict(env)
    env_b["ORACLE_RETADDR"] = "beef0000"
    binp = os.path.join(rd.BIN, "native_drv2")
    dyn, total, compared, skipped = {}, 0, 0, 0
    for i in range(0, len(funcs), a.chunk):
        cases = []
        for f in funcs[i: i + a.chunk]:
            cases += rd.gen_cases(int(f["va"], 16), 16, a.seed, ptrinfo[f["va"]])
        chunks = [cases[x:: a.jobs] for x in range(a.jobs)]
        ra, _ = rd.run_side(binp, a.pe, chunks, work, "ra", env)
        rb, _ = rd.run_side(binp, a.pe, chunks, work, "rb", env_b)
        for c in cases:
            x, y = ra.get(c["id"]), rb.get(c["id"])
            total += 1
            if x is None or y is None or x["status"] == "timeout" or y["status"] == "timeout":
                skipped += 1
                continue
            compared += 1
            x, y = dict(x), dict(y)
            for k in ("side",):
                x.pop(k, None)
                y.pop(k, None)
            if x != y:
                keys = sorted(k for k in set(x) | set(y) if x.get(k) != y.get(k))
                dyn.setdefault(c["fn"], []).append({"id": c["id"], "differs": keys, "status": (x["status"], y["status"])})
        for fn_ in os.listdir(work):
            os.remove(os.path.join(work, fn_))
        print("chunk %d: cases %d, functions with a difference so far: %d" % (i // a.chunk, total, len(dyn)), file=sys.stderr)
    res = {"seed": a.seed, "cases": total, "compared": compared, "skipped_timeout_or_missing": skipped,
           "functions_depending_on_return_address": {"0x%x" % k: v for k, v in sorted(dyn.items())}}
    json.dump(res, open(os.path.join(a.out, "retaddr-dynamic.json"), "w"), indent=1)
    print(json.dumps({k: (v if k != "functions_depending_on_return_address" else sorted(v)) for k, v in res.items()}, indent=1))


if __name__ == "__main__":
    main()
