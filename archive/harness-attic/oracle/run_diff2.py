#!/usr/bin/env python3
"""A5 oracle v2: function-level differential run, native machine code vs lifted C (decision 0036).

Differences from run_diff.py:
  * inputs: arguments that argptr.py finds dereferenced get valid pointers into the scratch area
    (identical address and content on both sides); structures that hold pointers are pointer-rich
    (scratch fill mode 5). Each case is tagged ptr_input (function has pointer args) / no_ptr.
  * compared state: GPR, XMM0-15, memory diff (as v1) plus EFLAGS status flags at return, MXCSR, x87
    control/status/tag (fnstenv image; tag word compared as the set of empty registers, decision 0028).
    Flags the lifter models as undefined (FLE_UNDEF) are counted separately from the defined ones.
  * native_fault cases get a cause from the fault signal, the faulting instruction's effective
    address and a backward scan of the register's definition.
  * negative control: one byte (opcode, immediate or displacement byte of an instruction that a
    bit-exact case executed) XOR 0x01, native only (ORACLE_PATCH), 200+ functions; the single-step
    trace (ORACLE_TRACE) of the unpatched and the patched native run classify the undetected ones.
Per-case categories
  bit_exact       native and lift returned, everything compared equal
  mismatch_core   GPR / XMM / memory differ (or native returned and lift faulted / timed out)
  mismatch_aux    core equal, EFLAGS (defined flags) / MXCSR / x87 differ
  lift_unsupported, lift_stop_other, native_fault  as v1
usage: run_diff2.py --pe PE --pure pure.json --argptr argptr.json --out DIR [--count 1000] [--n 16]
       [--seed 1] [--negctl 200] [--jobs 16] [--exclude exclude-stats.json]
"""
import argparse
import csv
import json
import os
import random
import struct
import subprocess
import sys
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import select_pure as sp  # noqa: E402
from argptr import CANON  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_OP_MEM, CS_OP_REG  # noqa: E402
from capstone.x86 import X86_REG_RIP  # noqa: E402

LIFT = os.path.abspath(os.path.join(HERE, "..", ".."))
BIN = os.environ.get("ORACLE_BIN") or os.path.join(LIFT, "out-oracle", "bin")  # ORACLE_BIN: run5 (decision 0038)
SCRATCH = 0x20000000
STACK = 0x30000000
STACK_SZ = 0x100000
M64 = (1 << 64) - 1
GPR = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"]
GIDX = {n: i for i, n in enumerate(GPR)}
ARGREG = ["rcx", "rdx", "r8", "r9"]
FLAGS = [("cf", 0), ("pf", 2), ("af", 4), ("zf", 6), ("sf", 7), ("of", 11)]
STATUS_MASK = 0x8D5

BOUNDARY = [0, 1, 2, 3, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF, 0x10000, 0x7FFFFFFF, 0x80000000,
            0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, M64, M64 - 1]
FP64 = [0, 0x8000000000000000, 0x3FF0000000000000, 0xBFF0000000000000, 0x4000000000000000, 0x7FF0000000000000,
        0xFFF0000000000000, 0x7FF8000000000000, 0x7FF4000000000000, 0x0000000000000001, 0x000FFFFFFFFFFFFF,
        0x7FEFFFFFFFFFFFFF, 0x3FE0000000000000, 0x4059000000000000, 0xC059000000000000]
FP32 = [0, 0x80000000, 0x3F800000, 0xBF800000, 0x7F800000, 0xFF800000, 0x7FC00000, 0x00000001, 0x007FFFFF,
        0x7F7FFFFF, 0x3F000000, 0x42C80000]


class Img:
    def __init__(self, pe):
        d, base, dd, secs = sp.load_pe(pe)
        size = max(va + max(vs, rs) for _n, va, vs, rs, _rp, _c in secs)
        img = bytearray(size)
        for _n, va, vs, rs, rp, _c in secs:
            img[va: va + rs] = d[rp: rp + rs]
        self.base, self.img, self.size = base, bytes(img), size
        self.wr = [(base + va, base + va + max(vs, rs)) for _n, va, vs, rs, _rp, c in secs if c & 0x80000000]
        self.md = Cs(CS_ARCH_X86, CS_MODE_64)
        self.md.detail = True

    def get(self, va, n):
        off = va - self.base
        return self.img[off: off + n] if 0 <= off < self.size else b""

    def one(self, va):
        for ins in self.md.disasm(self.get(va, 16), va):
            return ins
        return None

    def writable(self, va):
        return any(a <= va < b for a, b in self.wr)


# ----------------------------------------------------------------------------- inputs

def vptr(r, align=16):
    return SCRATCH + 0x20000 + (r.randrange(0, 0x40000) & ~(align - 1))


def int_arg(r, kind=None):
    kind = kind or r.choice(["small", "small", "small", "boundary", "rand32", "rand64", "zero"])
    if kind == "small":
        return r.randrange(0, 65)
    if kind == "boundary":
        return r.choice(BOUNDARY)
    if kind == "rand32":
        return r.getrandbits(32)
    if kind == "zero":
        return 0
    return r.getrandbits(64)


def gen_cases(fn_va, n, seed, info):
    P = set(info["ptr"])
    D = set(info["dbl"])
    r = random.Random((seed << 32) ^ fn_va ^ 0x5A5A)
    has_ptr = bool(P)
    cases = []
    for k in range(n):
        gpr = [r.getrandbits(64) for _ in range(16)]
        xmm = [[r.getrandbits(64), r.getrandbits(64)] for _ in range(16)]
        args = [None] * 8
        if k == 0:
            fill, kind = 2, "zero"
        elif k == 1:
            fill, kind = (5 if D else 3), "one"
        elif k == 2:
            fill, kind = (5 if D else 4), "small"
        elif k == 3:
            fill, kind = 6, "small"
        elif k == 11:
            fill, kind = (5 if D else 1), "boundary"
        elif k == 13:
            fill, kind = (5 if D else 0), "rand32"
        else:
            fill, kind = (5 if D and r.random() < 0.8 else r.choice([0, 1, 6])), None
        for j in range(8):
            if j in P:
                args[j] = vptr(r)
            elif kind == "one":
                args[j] = 1
            elif kind == "boundary":
                args[j] = r.choice(BOUNDARY)
            else:
                args[j] = int_arg(r, kind)
        for j in info.get("idx", ()):
            if j not in P and r.random() < 0.85:
                args[j] = r.randrange(0, 17)
        if k == 12 and P:
            p0 = vptr(r)
            for j in P:
                args[j] = p0
        pokes = []
        for j, d in info.get("ptrf", ()):
            if j in P and args[j] is not None:
                pokes.append((args[j] + d, vptr(r)))
        for j, d in info.get("idxf", ()):
            if j in P and args[j] is not None:
                pokes.append((args[j] + d, r.randrange(0, 17)))
        pokes = pokes[:24]
        if k == 9:
            for j in range(16):
                xmm[j] = [r.choice(FP64), r.choice(FP64)]
        elif k == 10:
            for j in range(16):
                xmm[j] = [r.choice(FP32) | (r.choice(FP32) << 32), r.choice(FP32) | (r.choice(FP32) << 32)]
        if k in (14, 15):
            for rn in ("rax", "rbx", "rbp", "rsi", "rdi", "r10", "r11", "r12", "r13", "r14", "r15"):
                if k == 14 or r.random() < 0.5:
                    gpr[GIDX[rn]] = vptr(r)
        for j, rn in enumerate(ARGREG):
            gpr[GIDX[rn]] = args[j]
        cases.append({"id": "%x.%d" % (fn_va, k), "fn": fn_va, "fill": fill, "seed": r.getrandbits(48),
                      "stk": args[4:8], "gpr": gpr, "xmm": xmm, "rflags": r.getrandbits(12) & STATUS_MASK,
                      "ptr_input": has_ptr, "ptr_args": sorted(P), "pokes": pokes})
    return cases


def case_line(c):
    parts = ["CASE", c["id"], "%x" % c["fn"], str(c["fill"]), "%x" % c["seed"], str(len(c["stk"]))]
    parts += ["%x" % v for v in c["stk"]]
    parts += ["%x" % v for v in c["gpr"]]
    for lo, hi in c["xmm"]:
        parts += ["%x" % lo, "%x" % hi]
    parts.append("%x" % c["rflags"])
    pk = c.get("pokes", [])
    parts.append(str(len(pk)))
    for ad, v in pk:
        parts += ["%x" % ad, "%x" % v]
    return " ".join(parts)


def run_side(binary, pe, chunks, outdir, tag, env):
    procs, outs = [], []
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


# ----------------------------------------------------------------------------- compare

def empty_mask(tw):
    return sum(1 << i for i in range(8) if ((tw >> (2 * i)) & 3) == 3)


def compare(nat, lif, st):
    """returns (core_fields, aux_fields); st accumulates flag/mxcsr/x87 statistics."""
    core, aux = [], []
    for i in range(16):
        if nat["gpr"][i] != lif["gpr"][i]:
            core.append(GPR[i])
    for i in range(16):
        if nat["xmm"][2 * i] != lif["xmm"][2 * i] or nat["xmm"][2 * i + 1] != lif["xmm"][2 * i + 1]:
            core.append("xmm%d" % i)
    memd = {}
    for reg in ("image", "scratch", "stack"):
        a, b = nat["mem"][reg], lif["mem"][reg]
        if a["n"] != b["n"] or a["hash"] != b["hash"]:
            core.append("mem:" + reg)
            na = {x[0]: x for x in a["blocks"]}
            nb = {x[0]: x for x in b["blocks"]}
            first = []
            for addr in sorted(set(na) | set(nb)):
                if na.get(addr) != nb.get(addr):
                    first.append({"addr": addr, "native": na.get(addr), "lift": nb.get(addr)})
                if len(first) >= 4:
                    break
            memd[reg] = {"native_n": a["n"], "lift_n": b["n"], "first": first}
    nf, lf, ud = nat["rflags"], lif["rflags"], lif["flundef"]
    for name, bit in FLAGS:
        if (ud >> bit) & 1:
            st["flag_undef:" + name] += 1
            st["flag_undef_native_is_1:" + name] += (nf >> bit) & 1
        else:
            st["flag_defined:" + name] += 1
            if ((nf ^ lf) >> bit) & 1:
                st["flag_defined_diff:" + name] += 1
                aux.append("eflags:" + name)
    d = nat["mxcsr"] ^ lif["mxcsr"]
    if d & 0xFFC0:
        aux.append("mxcsr_ctrl")
    if d & 0x3F:
        aux.append("mxcsr_sticky")
    st["mxcsr_native_ne_reset"] += nat["mxcsr"] != 0x1F80
    st["mxcsr_lift_ne_reset"] += lif["mxcsr"] != 0x1F80
    ncw, nsw, ntw = nat["x87"]
    lcw, lsw, ltw = lif["x87"]
    if ncw != lcw:
        aux.append("x87_cw")
    if nsw != lsw:
        aux.append("x87_sw")
    if empty_mask(ntw) != empty_mask(ltw):
        aux.append("x87_tw_empty")
    st["x87_tw_raw_differs(informational)"] += ntw != ltw
    st["x87_native_ne_reset"] += (ncw, nsw, ntw) != (0x37F, 0, 0xFFFF)
    st["x87_lift_ne_reset"] += (lcw, lsw, ltw) != (0x37F, 0, 0xFFFF)
    return core, aux, memd


def classify(nat, lif, st):
    if nat is None or nat["status"] != "ok":
        ns = nat["status"] if nat else "missing"
        same = bool(nat and lif and nat["status"] == "fault" and lif["status"] == "fault" and
                    nat.get("fault_addr") == lif.get("fault_addr"))
        return "native_fault", {"native": ns, "lift": lif["status"] if lif else "missing", "same_fault_addr": same}
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
        return "mismatch_core", {"fields": ["lift_" + lif["status"]], "lift_msg": lif.get("msg", ""),
                                 "lift_fault_addr": lif.get("fault_addr")}
    core, aux, memd = compare(nat, lif, st)
    if core:
        return "mismatch_core", {"fields": core + aux, "mem": memd}
    if aux:
        return "mismatch_aux", {"fields": aux}
    return "bit_exact", {"wrote_blocks": sum(nat["mem"][r]["n"] for r in ("image", "scratch")),
                         "wrote_stack_blocks": nat["mem"]["stack"]["n"]}


# ----------------------------------------------------------------------------- native_fault causes

def reg64(fg, rname):
    c = CANON.get(rname)
    return int(fg[GIDX[c]], 16) if c else None


def mem_ea(img, ins, fg):
    for op in ins.operands:
        if op.type == CS_OP_MEM:
            m = op.mem
            if m.base == X86_REG_RIP:
                return ins.address + ins.size + m.disp, None, None, op
            b = img.md.reg_name(m.base) if m.base else None
            x = img.md.reg_name(m.index) if m.index else None
            ea = m.disp
            if b:
                ea += reg64(fg, b) or 0
            if x:
                ea += (reg64(fg, x) or 0) * m.scale
            return ea & M64, b, x, op
    return None, None, None, None


def origin_of(img, fn_lo, rip, rname, c, fg):
    c64 = CANON.get(rname)
    val = reg64(fg, rname)
    for j, rn in enumerate(ARGREG):
        if c["gpr"][GIDX[rn]] == val:
            return "same_value_as_arg%d%s" % (j, "(ptr_given)" if j in c["ptr_args"] else "(int_arg)")
    for j, v in enumerate(c["stk"]):
        if v == val:
            return "same_value_as_stackarg%d" % j
    seq = []
    for ins in img.md.disasm(img.get(fn_lo, rip - fn_lo + 16), fn_lo):
        if ins.address >= rip:
            break
        seq.append(ins)
    for ins in reversed(seq[-40:]):
        try:
            _rd, wl = ins.regs_access()
        except Exception:
            continue
        if not any(CANON.get(img.md.reg_name(w)) == c64 for w in wl):
            continue
        mn = ins.mnemonic
        ops = ins.operands
        if mn == "xor" and len(ops) == 2 and ops[0].type == CS_OP_REG and ops[1].type == CS_OP_REG and ops[0].reg == ops[1].reg:
            return "zeroed_in_function"
        if mn == "mov" and len(ops) == 2 and ops[1].type == CS_OP_MEM:
            ea, b, _x, _op = mem_ea(img, ins, [hex(0)] * 16)
            m = ops[1].mem
            if m.base == X86_REG_RIP:
                return "loaded_from_image_global(writable)" if img.writable(ea) else "loaded_from_image_const"
            bn = CANON.get(img.md.reg_name(m.base)) if m.base else None
            if bn in ARGREG and ARGREG.index(bn) in c["ptr_args"]:
                return "loaded_via_arg_ptr(content-of-given-structure)"
            if bn == "rsp":
                return "loaded_from_stack_slot"
            return "loaded_from_memory(base=%s)" % (bn or "abs")
        if mn == "mov" and len(ops) == 2 and ops[1].type == CS_OP_REG:
            return "copied_from_reg"
        if mn == "mov" and len(ops) == 2:
            return "immediate"
        return "computed(%s)" % mn
    return "no_def_in_function(callee or incoming)"


ALIGN_FREE = {"movups", "movupd", "movdqu", "lddqu", "movlps", "movhps", "movlpd", "movhpd", "movss", "movsd",
              "movd", "movq", "movsldup", "movshdup"}


def fault_cause(img, c, nat, fn_lo):
    st = nat["status"]
    if st == "timeout":
        return "timeout_2s(loop)", ""
    if st == "abort":
        return "native_abort", ""
    if st != "fault":
        return "native_" + st, ""
    sig = nat["sig"]
    addr = int(nat["fault_addr"], 16)
    rip = int(nat["fault_rip"], 16)
    fg = nat["fgpr"]
    code = nat.get("code", 0)
    if sig == 4:
        return "sigill", ""
    if sig == 8:
        return "sigfpe(divide)", ""
    if sig == 7:
        return "sigbus", ""
    if not (img.base <= rip < img.base + img.size):
        return "control_flow_left_image(bad_ret_or_jump)", ""
    ins = img.one(rip)
    mn = ins.mnemonic if ins else "?"
    ea, b, x, op = mem_ea(img, ins, fg) if ins else (None, None, None, None)
    implicit = False
    if ea is None and ins:
        if mn.startswith(("push", "pop", "call", "ret")):
            ea, implicit, b = int(fg[4], 16), True, "rsp"
        elif mn.startswith(("movs", "stos", "lods", "cmps", "scas", "rep")):
            ea, implicit, b = int(fg[7], 16), True, "rdi"
    if code == 0x80:
        if ea is not None and ((ea >> 47) not in (0, 0x1FFFF)):
            reg = b or x
            if reg and not implicit:
                return "gp_noncanonical_address", "%s base=%s origin=%s" % (mn, reg, origin_of(img, fn_lo, rip, reg, c, fg))
            return "gp_noncanonical_address", mn
        if ins and op is not None and ea is not None and ea & 15 and mn not in ALIGN_FREE and \
                any(o.type == CS_OP_REG and img.md.reg_name(o.reg).startswith("xmm") for o in ins.operands):
            return "gp_misaligned_sse_operand", mn
        return "gp_other", mn
    if ea is None:
        return "segv_no_memory_operand_decoded", mn
    det = mn
    if code == 2 and img.base <= addr < img.base + img.size:
        return "write_to_readonly_image", mn
    if abs(addr - (STACK + STACK_SZ // 2)) < 0x4000000 and (STACK - 0x3000000 <= addr < STACK + STACK_SZ + 0x3000000):
        return "stack_range_exceeded", mn
    if addr < 0x10000:
        kind = "null_deref(addr<0x10000)"
    elif SCRATCH - 0x1000000 <= addr < SCRATCH + 0x100000 + 0x1000000:
        kind = "scratch_range_exceeded"
    else:
        kind = "wild_pointer"
    reg = b or x
    if reg and not implicit:
        o = origin_of(img, fn_lo, rip, reg, c, fg)
        return kind, "%s base=%s origin=%s" % (mn, reg, o)
    return kind, det


# ----------------------------------------------------------------------------- negative control

def opcode_imm_disp_bytes(ins):
    """(offset, role) of opcode, immediate and displacement bytes of one instruction"""
    out = []
    npre = sum(1 for p in ins.prefix if p)
    start = npre + (1 if ins.rex else 0)
    ends = [v for v in (ins.modrm_offset if ins.modrm_offset else 0,) if v]
    if not ends:
        ends = [v for v in (ins.disp_offset, ins.imm_offset) if v]
    end_op = min(ends) if ends else ins.size
    for o in range(start, min(end_op, ins.size)):
        out.append((o, "opcode"))
    if ins.disp_size:
        for o in range(ins.disp_offset, ins.disp_offset + ins.disp_size):
            out.append((o, "disp"))
    if ins.imm_size:
        for o in range(ins.imm_offset, ins.imm_offset + ins.imm_size):
            out.append((o, "imm"))
    return [(o, r) for o, r in out if o < ins.size]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pe", required=True)
    ap.add_argument("--pure", required=True)
    ap.add_argument("--argptr", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--exclude", default="")
    ap.add_argument("--count", type=int, default=1000)
    ap.add_argument("--n", type=int, default=16)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--negctl", type=int, default=200)
    ap.add_argument("--jobs", type=int, default=16)
    ap.add_argument("--base200", type=int, default=200, help="size of the v1-identical sample (Random(seed).sample)")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    work = os.path.join(a.out, "work")
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ)
    env["TMPDIR"] = os.path.join(LIFT, "out-oracle", "tmp")
    env_tr = dict(env)
    env_tr["ORACLE_TRACE"] = "1"

    pure = json.load(open(a.pure))
    funcs = pure["functions"]
    ptrinfo = json.load(open(a.argptr))["functions"]
    img = Img(a.pe)
    rnd = random.Random(a.seed)
    base200 = rnd.sample(funcs, min(a.base200, len(funcs)))
    chosen = {f["va"] for f in base200}
    rest = [f for f in funcs if f["va"] not in chosen]
    random.Random(a.seed + 1).shuffle(rest)
    sel = base200 + rest[: max(0, a.count - len(base200))]
    sel.sort(key=lambda f: int(f["va"], 16))
    base_vas = {f["va"] for f in base200}
    fend = {f["va"]: int(f["end"], 16) for f in funcs}

    cases = []
    for f in sel:
        cases += gen_cases(int(f["va"], 16), a.n, a.seed, ptrinfo[f["va"]])
    byid = {c["id"]: c for c in cases}
    chunks = [cases[i::a.jobs] for i in range(a.jobs)]
    nat, rcn = run_side(os.path.join(BIN, "native_drv2"), a.pe, chunks, work, "nat", env)
    lif, rcl = run_side(os.path.join(BIN, "lift_drv2"), a.pe, chunks, work, "lift", env)
    ntr, rct = run_side(os.path.join(BIN, "native_drv2"), a.pe, chunks, work, "ntr", env_tr)
    print("driver exit codes native %s lift %s native-trace %s" % (sorted(set(rcn)), sorted(set(rcl)), sorted(set(rct))),
          file=sys.stderr)

    # a 2 s alarm under 16 parallel jobs can fire on a case that takes milliseconds alone: every case
    # that timed out on either side is re-run on both sides, 8 jobs, 10 s alarm
    retry_ids = [c["id"] for c in cases if (nat.get(c["id"]) or {}).get("status") == "timeout" or
                 (lif.get(c["id"]) or {}).get("status") == "timeout"]
    retry_stat = {"retried": len(retry_ids), "native_before": Counter((nat.get(i) or {}).get("status") for i in retry_ids),
                  "lift_before": Counter((lif.get(i) or {}).get("status") for i in retry_ids)}
    if retry_ids:
        rc_cases = [byid[i] for i in retry_ids]
        e10 = dict(env)
        e10["ORACLE_TIMEOUT"] = "10"
        rch = [rc_cases[i::8] for i in range(8)]
        n_r, _ = run_side(os.path.join(BIN, "native_drv2"), a.pe, rch, work, "natr", e10)
        l_r, _ = run_side(os.path.join(BIN, "lift_drv2"), a.pe, rch, work, "liftr", e10)
        nat.update(n_r)
        lif.update(l_r)
        retry_stat["native_after"] = Counter((nat.get(i) or {}).get("status") for i in retry_ids)
        retry_stat["lift_after"] = Counter((lif.get(i) or {}).get("status") for i in retry_ids)
    retry_stat = {k: dict(v) if isinstance(v, Counter) else v for k, v in retry_stat.items()}

    st = Counter()
    rows = []
    faults = []
    for c in cases:
        cat, det = classify(nat.get(c["id"]), lif.get(c["id"]), st)
        row = {"fn": "0x%x" % c["fn"], "id": c["id"], "kind": "ptr_input" if c["ptr_input"] else "no_ptr",
               "in_v1_sample": "0x%x" % c["fn"] in base_vas, "category": cat, "detail": det}
        if cat == "native_fault":
            cause, extra = fault_cause(img, c, nat[c["id"]], c["fn"])
            row["fault_cause"] = cause
            row["fault_extra"] = extra
            row["lift_status"] = det["lift"]
            n = nat[c["id"]]
            faults.append({"id": c["id"], "fn": row["fn"], "cause": cause, "extra": extra, "sig": n.get("sig"),
                           "code": n.get("code"), "addr": n.get("fault_addr"), "rip": n.get("fault_rip"),
                           "lift": det["lift"], "same_fault_addr": det["same_fault_addr"]})
        rows.append(row)

    # trace consistency: the single-stepped native run must give the same result as the plain run
    tr_cmp = Counter()
    for c in cases:
        n1, n2 = nat.get(c["id"]), ntr.get(c["id"])
        if not n1 or not n2:
            tr_cmp["missing"] += 1
        elif n1["status"] != n2["status"]:
            tr_cmp["status_differs:%s->%s" % (n1["status"], n2["status"])] += 1
        elif n1["status"] == "ok":
            core, aux, _ = compare(n1, n2, Counter())
            tr_cmp["equal" if not core and not aux else "differs"] += 1
        else:
            tr_cmp["both_" + n1["status"]] += 1

    # per-function coverage of the cases (native single-step trace)
    exec_by_fn = defaultdict(set)
    for c in cases:
        t = ntr.get(c["id"])
        if t and t["status"] == "ok" and "trace" in t:
            exec_by_fn[c["fn"]].update(t["trace"]["rips"])
    cov = []
    for f in sel:
        va = int(f["va"], 16)
        own = {r for r in exec_by_fn.get(va, ()) if va <= r < fend[f["va"]]}
        cov.append(len(own) / max(1, f["insns"]))

    # --- negative control
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
    data = open(a.pe, "rb").read()

    def foff(va):
        lf = struct.unpack_from("<I", data, 0x3C)[0]
        nsec = struct.unpack_from("<H", data, lf + 6)[0]
        osz = struct.unpack_from("<H", data, lf + 20)[0]
        rva = va - img.base
        for i in range(nsec):
            s = lf + 24 + osz + 40 * i
            vs, sva, rs, rp = struct.unpack_from("<IIII", data, s + 8)
            if sva <= rva < sva + rs:
                return rp + rva - sva
        return None

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
            for o, role in opcode_imm_disp_bytes(ins):
                cand.append((rip, o, role, ins))
        if not cand:
            continue
        rip, o, role, ins = rr.choice(cand)
        controls.append({"fn": fva, "patch_va": rip + o, "insn_va": rip, "role": role, "insn": "%s %s" % (ins.mnemonic, ins.op_str),
                         "bytes": ins.bytes.hex(), "byte_index": o, "kind": "executed_insn_byte"})
    # three constant returners, kept as a sanity check (mov eax, imm32; ret)
    ncr = 0
    for fobj in funcs:
        if ncr >= 3:
            break
        va = int(fobj["va"], 16)
        b = img.get(va, 6)
        if len(b) == 6 and b[0] == 0xB8 and b[5] == 0xC3 and any(ci["fn"] == va for ci in controls) is False:
            controls.append({"fn": va, "patch_va": va + 1, "insn_va": va, "role": "imm", "insn": "mov eax, imm32",
                             "bytes": b[:5].hex(), "byte_index": 1, "kind": "const_returner_imm"})
            ncr += 1
    for fva in {ci["fn"] for ci in controls} - set(basis) - {int(f["va"], 16) for f in sel}:
        pass

    ctl_cases = {}
    for ci in controls:
        if ci["fn"] in basis:
            ctl_cases[ci["fn"]] = [byid[x] for x in basis[ci["fn"]]]
        else:
            ctl_cases[ci["fn"]] = gen_cases(ci["fn"], a.n, a.seed, ptrinfo.get("0x%x" % ci["fn"], {"ptr": [], "dbl": []}))
    ctl_extra_lif = {}
    extra_fns = [ci["fn"] for ci in controls if ci["fn"] not in basis]
    if extra_fns:
        ex = []
        for fva in extra_fns:
            ex += ctl_cases[fva]
        l2, _ = run_side(os.path.join(BIN, "lift_drv2"), a.pe, [ex[i::a.jobs] for i in range(a.jobs)], work, "liftx", env)
        ctl_extra_lif = l2
        n2, _ = run_side(os.path.join(BIN, "native_drv2"), a.pe, [ex[i::a.jobs] for i in range(a.jobs)], work, "ntrx", env_tr)
        ntr.update(n2)
        n3, _ = run_side(os.path.join(BIN, "native_drv2"), a.pe, [ex[i::a.jobs] for i in range(a.jobs)], work, "natx", env)
        nat.update(n3)

    def run_control(idx):
        ci = controls[idx]
        cs = ctl_cases[ci["fn"]]
        cf = os.path.join(work, "cases-neg-%03d.txt" % idx)
        of = os.path.join(work, "out-neg-%03d.jsonl" % idx)
        with open(cf, "w") as fh:
            for c in cs:
                fh.write(case_line(c) + "\n")
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
        fields = Counter()
        exec_cases = 0
        for c in cs:
            cid = c["id"]
            m = got.get(cid)
            l = lif.get(cid) or ctl_extra_lif.get(cid)
            n0 = nat.get(cid)
            t0 = ntr.get(cid)
            if not m or not l or not n0 or n0["status"] != "ok" or l["status"] != "ok":
                res["no_baseline"] += 1
                continue
            if t0 and t0["status"] == "ok" and ci["insn_va"] in t0["trace"]["rips"]:
                exec_cases += 1
            if m["status"] != "ok":
                res["fault_diff:" + m["status"]] += 1
                continue
            core, aux, _ = compare(m, l, sink)
            if "mxcsr_sticky" in aux:
                aux.remove("mxcsr_sticky")
                res["sticky_diff_ignored"] += 1
            if core:
                res["core"] += 1
                fields.update(core)
            elif aux:
                res["aux"] += 1
                fields.update(aux)
            else:
                res["equal"] += 1
            if t0 and t0["status"] == "ok" and "trace" in m:
                same = m["trace"]["digest"] == t0["trace"]["digest"] and m["trace"]["steps"] == t0["trace"]["steps"]
                res["trace_same" if same else "trace_differs"] += 1
        out = dict(ci)
        out["patch_va"] = "0x%x" % ci["patch_va"]
        out["insn_va"] = "0x%x" % ci["insn_va"]
        out["fn"] = "0x%x" % ci["fn"]
        out["cases"] = dict(res)
        out["executing_cases"] = exec_cases
        out["fields"] = dict(fields)
        faultn = sum(v for k, v in res.items() if k.startswith("fault_diff"))
        if res["core"]:
            out["outcome"] = "detected_core"
        elif res["aux"]:
            out["outcome"] = "detected_aux_only"
        elif faultn:
            out["outcome"] = "detected_by_native_fault_only"
        elif res["trace_differs"]:
            out["outcome"] = "undetected_state_changed_but_masked"
        elif res["no_baseline"] and not res["equal"]:
            out["outcome"] = "no_baseline"
        else:
            out["outcome"] = "undetected_equivalent_no_state_change"
        return out

    with ThreadPoolExecutor(a.jobs) as ex:
        ctl_res = list(ex.map(run_control, range(len(controls))))
    oc = Counter(c["outcome"] for c in ctl_res)
    by_role = defaultdict(Counter)
    for c in ctl_res:
        by_role[c["role"]][c["outcome"]] += 1
    by_kind = defaultdict(Counter)
    for c in ctl_res:
        by_kind[c["kind"]][c["outcome"]] += 1
    exec_one = Counter("exec_cases=1" if c["executing_cases"] == 1 else "exec_cases>=2" if c["executing_cases"] else "exec_cases=0"
                       for c in ctl_res if c["outcome"].startswith("undetected"))
    neg = {"controls_total": len(ctl_res), "outcomes": dict(oc), "by_role": {k: dict(v) for k, v in by_role.items()},
           "by_kind": {k: dict(v) for k, v in by_kind.items()},
           "detected_output_difference": oc["detected_core"] + oc["detected_aux_only"],
           "detected_incl_native_fault": oc["detected_core"] + oc["detected_aux_only"] + oc["detected_by_native_fault_only"],
           "detected_core_only(v1 compare set)": oc["detected_core"],
           "undetected_by_executing_case_count": dict(exec_one), "controls": ctl_res}

    # --- summary
    cnt = Counter(r["category"] for r in rows)
    per_kind = defaultdict(Counter)
    for r in rows:
        per_kind[r["kind"]][r["category"]] += 1
    v1s = Counter(r["category"] for r in rows if r["in_v1_sample"])
    v1_kind = defaultdict(Counter)
    for r in rows:
        if r["in_v1_sample"]:
            v1_kind[r["kind"]][r["category"]] += 1
    cause = Counter(r["fault_cause"] for r in rows if r["category"] == "native_fault")
    cause_x = Counter((r["fault_cause"], r["fault_extra"].split(" origin=")[-1] if "origin=" in r["fault_extra"] else "")
                      for r in rows if r["category"] == "native_fault")
    lift_vs = Counter("%s/%s" % (r["fault_cause"].split("(")[0], r["lift_status"]) for r in rows if r["category"] == "native_fault")
    sameaddr = sum(1 for f in faults if f["same_fault_addr"])
    fieldc = Counter()
    for r in rows:
        if r["category"] in ("mismatch_core", "mismatch_aux"):
            for fl in r["detail"]["fields"]:
                fieldc[fl] += 1
    unsup, other = Counter(), Counter()
    for r in rows:
        if r["category"] == "lift_unsupported":
            unsup[r["detail"]["msg"]] += 1
        elif r["category"] == "lift_stop_other":
            other[r["detail"].get("msg", r["detail"].get("reason", ""))] += 1
    per_fn = defaultdict(Counter)
    for r in rows:
        per_fn[r["fn"]][r["category"]] += 1
    fn_verdict = Counter()
    fn_rows = []
    for f in sel:
        cc = per_fn[f["va"]]
        if cc.get("mismatch_core"):
            v = "has_mismatch_core"
        elif cc.get("mismatch_aux"):
            v = "has_mismatch_aux_only"
        elif cc.get("bit_exact"):
            v = "bit_exact_all_comparable"
        elif cc.get("lift_unsupported") or cc.get("lift_stop_other"):
            v = "lift_not_executable"
        else:
            v = "native_fault_all"
        fn_verdict[v] += 1
        fn_rows.append({"fn": f["va"], "ptr_args": ptrinfo[f["va"]]["ptr"], "verdict": v, "cases": dict(cc)})
    nptrfn = sum(1 for f in sel if ptrinfo[f["va"]]["ptr"])
    fstat = {k: v for k, v in sorted(st.items())}
    aux_rows = [r for r in rows if r["category"] == "mismatch_aux"]
    fstat["mismatch_aux_cases_only_mxcsr_sticky"] = sum(1 for r in aux_rows if set(r["detail"]["fields"]) == {"mxcsr_sticky"})
    fstat["mismatch_aux_functions_only_mxcsr_sticky"] = len({r["fn"] for r in aux_rows if set(r["detail"]["fields"]) == {"mxcsr_sticky"}})
    sticky_bits = Counter()
    for r in rows:
        if r["category"] == "mismatch_aux":
            n_, l_ = nat[r["id"]], lif[r["id"]]
            d_ = n_["mxcsr"] ^ l_["mxcsr"]
            for b_, nm in ((0, "IE"), (1, "DE"), (2, "ZE"), (3, "OE"), (4, "UE"), (5, "PE")):
                if (d_ >> b_) & 1:
                    sticky_bits[nm] += 1
    fstat["mxcsr_sticky_bits_set_by_native_only"] = dict(sticky_bits)
    flag_tab = {}
    for name, _b in FLAGS:
        d = st["flag_defined:" + name]
        u = st["flag_undef:" + name]
        flag_tab[name] = {"defined_compared": d, "defined_equal": d - st["flag_defined_diff:" + name],
                          "defined_diff": st["flag_defined_diff:" + name], "undefined_in_lift_not_compared": u,
                          "undefined_native_value_1": st["flag_undef_native_is_1:" + name]}
    covs = sorted(cov)
    summary = {
        "pe": a.pe, "seed": a.seed, "functions_selected": len(sel), "functions_with_ptr_args": nptrfn,
        "functions_without_ptr_args": len(sel) - nptrfn, "pure_pool": len(funcs),
        "cases_per_function": a.n, "cases_total": len(rows),
        "categories": dict(cnt), "categories_by_input_kind": {k: dict(v) for k, v in per_kind.items()},
        "v1_identical_sample_200": {"functions": len(base_vas), "cases": sum(v1s.values()), "categories": dict(v1s),
                                    "by_input_kind": {k: dict(v) for k, v in v1_kind.items()},
                                    "v1_run3_reference": {"cases": 2400, "native_fault": 943, "bit_exact": 1457}},
        "native_fault_causes": dict(cause.most_common()),
        "native_fault_causes_with_origin": {"%s | %s" % k: v for k, v in cause_x.most_common()},
        "native_fault_vs_lift": dict(lift_vs.most_common()), "native_fault_same_fault_addr": sameaddr,
        "lift_unsupported_messages": dict(unsup), "lift_stop_other_messages": dict(other.most_common(20)),
        "mismatch_fields": dict(fieldc), "function_verdicts": dict(fn_verdict),
        "flag_stats": flag_tab, "state_stats": fstat, "trace_consistency": dict(tr_cmp), "timeout_retry": retry_stat,
        "coverage_own_insns_executed": {"functions": len(covs), "mean": sum(covs) / max(1, len(covs)),
                                        "median": covs[len(covs) // 2] if covs else None,
                                        "functions_full_coverage": sum(1 for x in covs if x >= 0.999),
                                        "functions_zero": sum(1 for x in covs if x == 0)},
        "negative_control": {k: v for k, v in neg.items() if k != "controls"},
    }
    if a.exclude and os.path.exists(a.exclude):
        summary["exclusion_stats"] = json.load(open(a.exclude))
    be = [r for r in rows if r["category"] == "bit_exact"]
    summary["bit_exact_cases_with_image_or_scratch_writes"] = sum(1 for r in be if r["detail"]["wrote_blocks"] > 0)
    for name, obj in (("a5-summary.json", summary), ("a5-functions.json", fn_rows), ("a5-cases.json", rows),
                      ("a5-native-faults.json", faults), ("a5-negctl.json", neg)):
        with open(os.path.join(a.out, name), "w") as fh:
            json.dump(obj, fh, indent=1)
    with open(os.path.join(a.out, "a5-cases.csv"), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["fn", "case", "input_kind", "category", "fields_or_reason", "fault_cause"])
        for r in rows:
            d = r["detail"]
            w.writerow([r["fn"], r["id"], r["kind"], r["category"],
                        ";".join(d.get("fields", [])) or d.get("reason", "") or d.get("native", ""),
                        r.get("fault_cause", "")])
    mm = [r for r in rows if r["category"] in ("mismatch_core", "mismatch_aux")]
    with open(os.path.join(a.out, "a5-mismatches.json"), "w") as fh:
        json.dump(mm, fh, indent=1)
    print(json.dumps({k: summary[k] for k in ("functions_selected", "cases_total", "categories", "categories_by_input_kind",
                                              "v1_identical_sample_200", "function_verdicts")}, indent=1))
    print("native_fault causes:", json.dumps(summary["native_fault_causes"], indent=1))
    print("negative control:", json.dumps({k: v for k, v in summary["negative_control"].items()}, indent=1))


if __name__ == "__main__":
    main()
