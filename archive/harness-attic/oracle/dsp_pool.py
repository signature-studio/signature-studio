#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: classify the pure-compute functions by instruction kind and build sample pools.

Instruction kinds are counted from the binary (capstone, the same disassembler select_pure.py uses) and cross-checked
against the mnemonic comments of the generated C (dsp_census.py):
  d0075   one of the 26 mnemonics of decision 0075 (sse_legacy.rs) in its xmm form (capstone operand is an xmm register;
          PEXTRW only with the 0F C5 opcode, which is the 66 0F C5 form that sse_legacy.rs lifts)
  sse     any legacy (non-VEX) SSE/SSE2 instruction with an xmm operand (superset of d0075; includes the SSE3 forms
          that the lifter does not support, listed separately as sse3p)
  vex     any VEX/EVEX instruction (capstone mnemonic starting with 'v' on an xmm/ymm/zmm operand, or k-register ops)
  stub    the CPU-dispatch stub of decision 0079 (`jmp [r10+rax*8]` = 41 FF 24 C2 preceded by lea r10,[rip+d])
  msvc    a function that contains an MSVC switch resolved by the lifter (decision 0089/0094)
Static facts from the generated C: unsup_own (a line became fle_unsupported), unsup_reach (own or reached through direct
calls/jumps).
usage: dsp_pool.py PE PURE.json CENSUS.json OUTDIR [--seed N --count N]
"""
import argparse
import json
import os
import random
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402
from capstone.x86 import X86_REG_XMM0, X86_REG_XMM31  # noqa: E402
from capstone import CS_OP_REG  # noqa: E402

D0075 = {"orps", "andnps", "pcmpeqb", "pcmpeqw", "pcmpgtb", "pcmpgtw", "paddb", "psubb", "psubq", "pmaxub", "pminub",
         "pmaddwd", "pmuludq", "psrldq", "pslldq", "pmovmskb", "movmskps", "movmskpd", "pextrw", "pinsrw", "movntps",
         "movntpd", "movntdq", "movnti", "rsqrtps", "rsqrtss"}
# SSE3 / SSSE3 / SSE4 mnemonics (not claimed by the runtime's CPUID, decision 0075 section 1)
SSE3P = {"lddqu", "movddup", "addsubps", "addsubpd", "movshdup", "movsldup", "haddps", "haddpd", "hsubps", "hsubpd",
         "pshufb", "phaddw", "phaddd", "phsubw", "phsubd", "pmaddubsw", "pmulhrsw", "psignb", "psignw", "psignd",
         "pabsb", "pabsw", "pabsd", "palignr", "pblendw", "pblendvb", "blendps", "blendpd", "blendvps", "blendvpd",
         "pextrb", "pextrd", "pextrq", "pinsrb", "pinsrd", "pinsrq", "pmaxsd", "pminsd", "pmaxsb", "pminsb", "pmaxud",
         "pminud", "pmaxuw", "pminuw", "pmulld", "pmuldq", "pmovsxbw", "pmovzxbw", "pmovsxwd", "pmovzxwd", "pmovsxdq",
         "pmovzxdq", "pmovsxbd", "pmovzxbd", "pmovsxbq", "pmovzxbq", "pmovsxwq", "pmovzxwq", "ptest", "pcmpeqq",
         "pcmpgtq", "packusdw", "roundps", "roundpd", "roundss", "roundsd", "dpps", "dppd", "insertps", "extractps",
         "mpsadbw", "phminposuw", "pcmpestri", "pcmpestrm", "pcmpistri", "pcmpistrm", "movntdqa", "crc32", "popcnt"}


def is_xmm(md, op):
    return op.type == CS_OP_REG and md.reg_name(op.reg).startswith("xmm")


def classify_fn(lo, hi):
    md, code, base = sp.G["md"], sp.G["code"], sp.G["base"]
    kinds = Counter()
    d75 = Counter()
    for ins in md.disasm(code[lo - base: hi - base], lo):
        mn = ins.mnemonic
        if mn.startswith("v") and any(op.type == CS_OP_REG and md.reg_name(op.reg)[:3] in ("xmm", "ymm", "zmm") for op in ins.operands):
            kinds["vex"] += 1
            continue
        if mn.startswith("k") and any(op.type == CS_OP_REG and md.reg_name(op.reg).startswith("k") for op in ins.operands):
            kinds["vex"] += 1
            continue
        has_x = any(is_xmm(md, op) for op in ins.operands)
        if not has_x:
            continue
        if mn in D0075:
            if mn == "pextrw" and not bytes(ins.bytes).find(b"\x0f\xc5") >= 0:
                kinds["sse3p"] += 1
                continue
            kinds["d0075"] += 1
            d75[mn] += 1
            kinds["sse"] += 1
        elif mn in SSE3P:
            kinds["sse3p"] += 1
        else:
            kinds["sse"] += 1
    return lo, dict(kinds), dict(d75)


def init(pe):
    sp.init(pe)


def run_one(fn):
    return classify_fn(*fn)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pe")
    ap.add_argument("pure")
    ap.add_argument("census")
    ap.add_argument("outdir")
    ap.add_argument("--seed", type=int, default=2)
    ap.add_argument("--count", type=int, default=1000)
    a = ap.parse_args()
    import multiprocessing as mp
    pure = json.load(open(a.pure))
    cen = json.load(open(a.census))["functions"]
    funcs = pure["functions"]
    fend = {f["va"]: int(f["end"], 16) for f in funcs}
    with mp.Pool(4, initializer=init, initargs=(a.pe,)) as pool:
        rows = pool.map(run_one, [(int(f["va"], 16), int(f["end"], 16)) for f in funcs], chunksize=64)
    cls = {"0x%x" % lo: (k, d) for lo, k, d in rows}

    # unsupported reach over the pure functions' direct-call graph
    pset = {f["va"] for f in funcs}
    unsup_own = {v for v in pset if cen.get(v, {}).get("unsup")}
    reach = set(unsup_own)
    rev = {}
    for v in pset:
        for c in cen.get(v, {}).get("callees", []):
            rev.setdefault(c, []).append(v)
    work = list(reach)
    while work:
        f = work.pop()
        for c in rev.get(f, []):
            if c not in reach:
                reach.add(c)
                work.append(c)

    out = {"pure": len(funcs), "unsup_own": len(unsup_own), "unsup_reach": len(reach)}
    for k in ("d0075", "sse", "sse3p", "vex"):
        out["has_" + k] = sum(1 for v in pset if cls[v][0].get(k))
    out["has_d0075_and_no_unsup_reach"] = sum(1 for v in pset if cls[v][0].get("d0075") and v not in reach)
    out["has_sse_and_no_unsup_reach"] = sum(1 for v in pset if cls[v][0].get("sse") and v not in reach)
    out["no_sse_no_vex_no_unsup"] = sum(1 for v in pset if not cls[v][0].get("sse") and not cls[v][0].get("vex") and v not in reach)
    mn_func = Counter()
    for v in pset:
        for m in cls[v][1]:
            mn_func[m] += 1
    out["d0075_mnemonics_functions"] = dict(sorted(mn_func.items(), key=lambda kv: -kv[1]))
    out["d0075_mnemonics_missing_in_pure"] = sorted(D0075 - set(mn_func))
    out["stub_in_pure"] = sum(1 for v in pset if cen.get(v, {}).get("stub"))
    out["msvc_in_pure"] = sum(1 for v in pset if cen.get(v, {}).get("msvc"))
    out["stub_functions_total"] = sum(1 for v, f in cen.items() if f["stub"])
    out["msvc_functions_total"] = sum(1 for v, f in cen.items() if f["msvc"])

    # cross-check with the generated C comments
    cc = Counter()
    for v in pset:
        f = cen.get(v)
        if not f:
            continue
        mset = {m.lower() for m in f["mn"]}
        if mset & D0075:
            cc["gen_comment_has_d0075"] += 1
        if cls[v][0].get("d0075"):
            cc["capstone_has_d0075"] += 1
        if (mset & D0075) and cls[v][0].get("d0075"):
            cc["both"] += 1
    out["crosscheck_d0075"] = dict(cc)

    os.makedirs(a.outdir, exist_ok=True)
    with open(os.path.join(a.outdir, "classes.json"), "x") as fh:
        json.dump({v: {"kinds": cls[v][0], "d0075": cls[v][1], "unsup_own": v in unsup_own, "unsup_reach": v in reach}
                   for v in sorted(pset)}, fh)

    # seed-2 pool: functions with a d0075 mnemonic first; filled with other legacy sse functions to a.count
    r = random.Random(a.seed)
    A = [f for f in funcs if cls[f["va"]][0].get("d0075")]
    B = [f for f in funcs if cls[f["va"]][0].get("sse") and not cls[f["va"]][0].get("d0075")]
    r.shuffle(A)
    r.shuffle(B)
    pool = A[:]
    if len(pool) < a.count:
        pool += B[: a.count - len(pool)]
    pool.sort(key=lambda f: int(f["va"], 16))
    out["pool_sse_d0075_functions"] = len(A)
    out["pool_sse_other_functions"] = len(B)
    out["pool_sse_size"] = len(pool)
    pj = dict(pure)
    pj["functions"] = pool
    pj["counts"] = dict(pure["counts"], pure=len(pool), pool="d0075 first, then other legacy sse (dsp_pool.py)")
    with open(os.path.join(a.outdir, "pool-sse.json"), "x") as fh:
        json.dump(pj, fh, indent=1)
    with open(os.path.join(a.outdir, "pool-summary.json"), "x") as fh:
        json.dump(out, fh, indent=1)
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
