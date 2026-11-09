#!/usr/bin/env python3
"""A5 oracle, a5f: native-fault-guided input repair (adds inputs; the existing cases are not touched).

A case on which the native machine code faults (null / wild / non-canonical pointer, misaligned SSE
operand, divide by zero) says nothing about the lifted C. This module re-runs such a case natively
single-stepped with native_drv2 ORACLE_RING_DIR (the last 16384 states: rip + the 16 GPRs before each
instruction), finds with a backward slice over that trace which INPUT made the faulting address (an input
register, a stack argument, an 8-byte word of the scratch area or of a writable image section that the
function loaded from), and builds a new case in which only that input holds a valid value. The case is
run again; the loop ends when the native call returns (or a repair is impossible / --rounds is used up).
Both sides then run the new case with identical inputs (a5f_run.py). Nothing about the lift side is looked
at while repairing (lift_drv2 is not run), and no result is ever ignored: a repaired case is judged exactly
as every other case (run_diff2.classify).

Backward slice (the value of register R before step k = source * S + K, mod 2^64):
  mov R,R2 / movzx|movsx|movsxd R,R2 / lea R,[B+d] / add|sub R,imm / inc / dec   keep following, K updated
  lea R,[B+I*s+d] / add R,R2 / shl R,imm / imul R,R2,imm / neg                  follow one operand, S or K updated
  add|sub R,[M]                                                                  follow R (M counted as the
                                                                                  constant it was), else the word M
  mov R,[M] / pop R / movsxd R,[M]                                               memory word M: if the function
                                                                                  stored it earlier (mov [M],R2
                                                                                  or add|sub|inc|dec [M],imm) follow
                                                                                  that, else it is an input word
  nothing wrote R before step 0                                                  R is an input register
  anything else (xor R,R, mov R,imm, and, shr ...)                               not repairable ("why" is recorded)
Input word -> poke (8 bytes, scratch or writable image section), stack argument slot -> stk[], input
register -> gpr[]. A5F_FEATURES (comma list, default image,arith,addmem) switches the steps off again for
the stage-by-stage measurement.
"""
import glob
import os
import random
import struct
import sys
import zlib
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run_diff2 as rd  # noqa: E402
from argptr import CANON  # noqa: E402
from capstone import CS_AC_WRITE, CS_OP_IMM, CS_OP_MEM, CS_OP_REG  # noqa: E402
from capstone.x86 import X86_REG_RIP  # noqa: E402

M64 = rd.M64
# A5F_FEATURES (comma list, default all) switches repair steps on for the stage-by-stage measurement:
#   image  : a pointer loaded from a writable image global is repaired by a poke into the image
#   arith  : scaled / indexed / read-modify-write / narrow-load / multi-operand cases (S in value = source*S + K)
#   addmem : add/sub reg,[mem] follows the register first, the memory word second
FEAT = set(os.environ.get("A5F_FEATURES", "image,arith,addmem").split(","))
IMAGE, ARITH, ADDMEM = "image" in FEAT, "arith" in FEAT, "addmem" in FEAT
SCRATCH, SCRATCH_SZ = rd.SCRATCH, 0x100000
STACK, STACK_SZ = rd.STACK, rd.STACK_SZ
STACK_TOP = STACK + 0xC0000
POKE_MAX = 64
GIDX = rd.GIDX
ENT = struct.Struct("<17Q")
ALIGNED_SSE = ("movaps", "movapd", "movdqa", "movntdq", "movntps", "movntpd", "andps", "andpd", "andnps", "andnpd",
               "orps", "orpd", "xorps", "xorpd", "addps", "addpd", "subps", "subpd", "mulps", "mulpd", "divps",
               "divpd", "pand", "pandn", "por", "pxor", "paddd", "paddq", "psubd", "psubq", "pcmpeqd", "punpckldq",
               "punpcklqdq", "punpckhqdq", "unpcklps", "unpcklpd", "cmpps", "cmppd", "minps", "maxps", "minpd", "maxpd")
STRING_OPS = ("movsb", "movsw", "movsd", "movsq", "stosb", "stosw", "stosd", "stosq", "lodsb", "lodsw", "lodsd",
              "lodsq", "scasb", "scasw", "scasd", "scasq", "cmpsb", "cmpsw", "cmpsd", "cmpsq")


def load_ring(path):
    with open(path, "rb") as fh:
        data = fh.read()
    total, cnt = struct.unpack_from("<2Q", data, 0)
    ents = [ENT.unpack_from(data, 16 + ENT.size * i) for i in range(cnt)]
    return total, ents


class Trace:
    def __init__(self, img, total, ents):
        self.img, self.total, self.e = img, total, ents
        self.wrapped = total > len(ents)
        self._ins = {}
        self._stores = None

    def ins(self, j):
        rip = self.e[j][0]
        r = self._ins.get(rip)
        if r is None:
            ins = self.img.one(rip)
            wset = set()
            if ins is not None:
                try:
                    _rd, wl = ins.regs_access()
                    for w in wl:
                        c = CANON.get(self.img.md.reg_name(w))
                        if c:
                            wset.add(c)
                except Exception:
                    pass
            r = self._ins[rip] = (ins, wset)
        return r

    def g(self, j, reg):
        return self.e[j][1 + GIDX[reg]]

    def ea(self, j, op):
        ins = self.ins(j)[0]
        m = op.mem
        if m.base == X86_REG_RIP:
            return (ins.address + ins.size + m.disp) & M64
        ea = m.disp
        if m.base:
            ea += self.g(j, CANON[self.img.md.reg_name(m.base)])
        if m.index:
            ea += self.g(j, CANON[self.img.md.reg_name(m.index)]) * m.scale
        return ea & M64

    def stores(self):
        """[(step, ea, size, src)] ascending; src = ("reg", canon) | ("const",) | ("rmw", delta) | ("other", mnemonic)"""
        if self._stores is not None:
            return self._stores
        out = []
        for j in range(len(self.e)):
            ins = self.ins(j)[0]
            if ins is None:
                continue
            mn = ins.mnemonic
            if mn in ("push", "call"):
                src = ("const",)
                if mn == "push" and ins.operands and ins.operands[0].type == CS_OP_REG:
                    src = ("reg", CANON.get(self.img.md.reg_name(ins.operands[0].reg)))
                elif mn == "push" and ins.operands and ins.operands[0].type == CS_OP_MEM:
                    src = ("other", "push_mem")
                out.append((j, (self.g(j, "rsp") - 8) & M64, 8, src))
                continue
            if mn in STRING_OPS or mn.startswith(("rep", "movs", "stos")):
                if mn.startswith(("movs", "stos")) and mn[-1] in "bwdq":
                    sz = {"b": 1, "w": 2, "d": 4, "q": 8}[mn[-1]]
                    out.append((j, self.g(j, "rdi"), sz, ("other", mn)))
                continue
            if mn in ("lea", "nop") or mn.startswith("prefetch"):
                continue
            for op in ins.operands:
                if op.type == CS_OP_MEM and (op.access & CS_AC_WRITE):
                    src = ("other", mn)
                    if mn == "mov" and len(ins.operands) == 2:
                        o1 = ins.operands[1]
                        if o1.type == CS_OP_REG:
                            c = CANON.get(self.img.md.reg_name(o1.reg))
                            if c and o1.size == op.size:
                                src = ("reg", c)
                        elif o1.type == CS_OP_IMM:
                            src = ("const",)
                    elif ARITH and mn in ("add", "sub") and len(ins.operands) == 2 and ins.operands[1].type == CS_OP_IMM:
                        d = ins.operands[1].imm
                        src = ("rmw", d if mn == "add" else -d)
                    elif ARITH and mn in ("inc", "dec"):
                        src = ("rmw", 1 if mn == "inc" else -1)
                    out.append((j, self.ea(j, op), op.size, src))
        self._stores = out
        return out

    def last_store(self, j, ea, size):
        for s in reversed(self.stores()):
            if s[0] >= j:
                continue
            if s[1] < ea + size and ea < s[1] + s[2]:
                return s
        return None

    def follow(self, reg, k, K, S=1, small=False, chain=None, depth=0):
        """where did the value of `reg` before step k come from; value = source * S + K (mod 2^64).
        small: the value is an index/count, so byte/word loads are acceptable sources"""
        chain = chain if chain is not None else []
        if depth > 60:
            return {"kind": "fail", "why": "chain_too_deep", "chain": chain}
        md = self.img.md
        guard = 0
        while True:
            guard += 1
            if guard > 30000:
                return {"kind": "fail", "why": "chain_too_long", "chain": chain}
            j = k - 1
            while j >= 0 and reg not in self.ins(j)[1]:
                j -= 1
            if j < 0:
                if self.wrapped:
                    return {"kind": "fail", "why": "ring_wrapped", "chain": chain}
                return {"kind": "reg", "reg": reg, "K": K, "S": S, "chain": chain}
            ins = self.ins(j)[0]
            mn, ops = ins.mnemonic, ins.operands
            if len(chain) < 40:
                chain.append("%x:%s %s" % (ins.address, mn, ins.op_str))
            dst = ops[0] if ops else None
            full = dst is not None and dst.type == CS_OP_REG and CANON.get(md.reg_name(dst.reg)) == reg
            if mn == "pop" and full:
                return self.load(j, self.g(j, "rsp"), 8, K, S, small, chain, depth)
            if not full:
                return {"kind": "fail", "why": "implicit_or_partial:" + mn, "chain": chain}
            src = ops[1] if len(ops) > 1 else None
            sreg = CANON.get(md.reg_name(src.reg)) if src is not None and src.type == CS_OP_REG else None
            if mn == "mov" and len(ops) == 2:
                if dst.size in (4, 8) and sreg and src.size == dst.size:
                    reg, k = sreg, j
                    continue
                if dst.size in (4, 8) and src.type == CS_OP_MEM:
                    return self.load(j, self.ea(j, src), dst.size, K, S, small, chain, depth)
                return {"kind": "fail", "why": "mov_%s%d" % ("imm" if src.type == CS_OP_IMM else "other", dst.size),
                        "chain": chain}
            if ARITH and mn in ("movzx", "movsx", "movsxd") and len(ops) == 2 and dst.size in (4, 8):
                if sreg:
                    reg, k = sreg, j
                    continue
                if src.type == CS_OP_MEM and (small or mn == "movsxd"):
                    return self.load(j, self.ea(j, src), src.size, K, S, small, chain, depth)
                return {"kind": "fail", "why": "computed:" + mn, "chain": chain}
            if mn == "lea" and dst.size in (4, 8):
                m = src.mem
                if m.base == X86_REG_RIP or (not m.base and not m.index) or (m.index and not ARITH):
                    return {"kind": "fail", "why": "lea_rip_or_absolute" if not m.index else "lea_index_or_rip", "chain": chain}
                B = CANON[md.reg_name(m.base)] if m.base else None
                I = CANON[md.reg_name(m.index)] if m.index else None
                if I is None:
                    reg, k, K = B, j, (K + m.disp * S) & M64
                elif B is None:
                    reg, k, K, S = I, j, (K + m.disp * S) & M64, (S * m.scale) & M64
                elif B == I:
                    reg, k, K, S = B, j, (K + m.disp * S) & M64, (S * (1 + m.scale)) & M64
                else:
                    bv, iv = self.g(j, B), self.g(j, I)
                    if plausible(bv) and not plausible(iv):
                        reg, k, K, S = I, j, (K + (bv + m.disp) * S) & M64, (S * m.scale) & M64
                    else:
                        reg, k, K = B, j, (K + (iv * m.scale + m.disp) * S) & M64
                continue
            if mn in ("add", "sub") and dst.size == 8 and len(ops) == 2:
                if src.type == CS_OP_MEM and j + 1 < len(self.e) and ADDMEM:
                    # first try the destination register as the source (the memory operand counts as the
                    # constant it was observed to be: after - before), then the memory word itself
                    pre, post = self.g(j, reg), self.g(j + 1, reg)
                    mv = (post - pre) & M64 if mn == "add" else (pre - post) & M64
                    K1 = (K + (mv if mn == "add" else -mv) * S) & M64
                    r1 = self.follow(reg, j, K1, S, small, list(chain), depth + 1)
                    if r1["kind"] != "fail":
                        return r1
                    if src.size == 8:
                        if mn == "add":
                            return self.load(j, self.ea(j, src), 8, (K + pre * S) & M64, S, small, chain, depth)
                        return self.load(j, self.ea(j, src), 8, (K + pre * S) & M64, (-S) & M64, small, chain, depth)
                    return r1
                if src.type == CS_OP_IMM:
                    K = (K + (src.imm if mn == "add" else -src.imm) * S) & M64
                    k = j
                    continue
                if sreg and src.size == 8 and ARITH:
                    a, b = self.g(j, reg), self.g(j, sreg)
                    if sreg == reg:
                        if mn == "add":
                            S, k = (S * 2) & M64, j
                            continue
                        return {"kind": "fail", "why": "computed:sub_self", "chain": chain}
                    if mn == "add":
                        if plausible(a) and not plausible(b):
                            reg, k, K = sreg, j, (K + a * S) & M64
                        else:
                            k, K = j, (K + b * S) & M64
                    else:
                        if plausible(a) and not plausible(b):
                            reg, k, K, S = sreg, j, (K + a * S) & M64, (-S) & M64
                        else:
                            k, K = j, (K - b * S) & M64
                    continue
                return {"kind": "fail", "why": "computed:" + mn, "chain": chain}
            if mn in ("inc", "dec") and dst.size == 8:
                K = (K + (S if mn == "inc" else -S)) & M64
                k = j
                continue
            if ARITH and mn in ("shl", "sal") and dst.size == 8 and src is not None and src.type == CS_OP_IMM and 0 <= src.imm < 16:
                S, k = (S << src.imm) & M64, j
                continue
            if ARITH and mn == "neg" and dst.size == 8:
                S, k = (-S) & M64, j
                continue
            if ARITH and mn == "imul" and dst.size == 8:
                if len(ops) == 3 and ops[2].type == CS_OP_IMM and sreg:
                    S, reg, k = (S * ops[2].imm) & M64, sreg, j
                    continue
                if len(ops) == 3 and ops[2].type == CS_OP_IMM and ops[1].type == CS_OP_MEM:
                    return {"kind": "fail", "why": "computed:imul_mem", "chain": chain}
                if len(ops) == 2 and sreg and sreg != reg:
                    S, k = (S * self.g(j, sreg)) & M64, j
                    continue
                if len(ops) == 2 and src.type == CS_OP_IMM:
                    S, k = (S * src.imm) & M64, j
                    continue
            return {"kind": "fail", "why": "computed:" + mn, "chain": chain}

    def load(self, j, ea, size, K, S, small, chain, depth=0):
        n = 0
        while True:
            s = self.last_store(j, ea, size)
            if s is None:
                break
            if s[1] == ea and s[2] >= size and s[3][0] == "rmw":
                K, j, n = (K + s[3][1] * S) & M64, s[0], n + 1
                if n > 3000:
                    return {"kind": "fail", "why": "rmw_chain_too_long", "chain": chain}
                continue
            if s[1] == ea and s[2] >= size and s[3][0] == "reg":
                chain.append("store@%d" % s[0])
                if depth > 40:
                    return {"kind": "fail", "why": "store_chain_too_deep", "chain": chain}
                return self.follow(s[3][1], s[0], K, S, small, chain, depth + 1)
            return {"kind": "fail", "why": "stored_by_function:" + s[3][0] + (":" + s[3][1] if len(s[3]) > 1 and isinstance(s[3][1], str) else ""),
                    "chain": chain}
        if size < 4 and not small:
            return {"kind": "fail", "why": "narrow_load_for_pointer", "chain": chain}
        return {"kind": "mem", "ea": ea, "size": size, "K": K, "S": S, "chain": chain}


def plausible(v):
    return SCRATCH <= v < SCRATCH + SCRATCH_SZ or STACK <= v < STACK + STACK_SZ


def pick_mem_operand(trace, k, ins, fault_addr):
    """the memory operand that faulted: the one whose effective address is not mapped-looking, else the one
    equal to the reported fault address, else the first"""
    ops = [o for o in ins.operands if o.type == CS_OP_MEM]
    if not ops:
        return None
    if not ARITH:
        return ops[0]
    for o in ops:
        if o.mem.base != X86_REG_RIP and not plausible(trace.ea(k, o)):
            return o
    for o in ops:
        if trace.ea(k, o) == fault_addr:
            return o
    return ops[0]


def solve_source(plan, want, rng, mode):
    """source value s with s*S + K == want (mod 2^64); None if not representable"""
    K, S = plan["K"], plan["S"]
    if mode in ("index", "small") and 1 <= S < 4096 and (K < 4096 or K >= M64 - 4096):
        lo = 0 if mode == "index" else 1
        return rng.randrange(lo, 17) if K < 4096 else rng.randrange(16, 33)
    diff = (want - K) & M64
    if S == 1:
        return diff
    if S == M64:
        return (-diff) & M64
    if 1 < S < 4096:
        if diff % S and mode == "ptr":
            diff -= diff % S  # a pointer target may be a few bytes lower; keeps the multiple of S
        return diff // S if diff % S == 0 else None
    return None


def analyse(img, case, nat, trace, rng):
    """returns (new_case or None, info dict). One repair step for one faulting native result."""
    sig = nat.get("sig")
    rip = int(nat["fault_rip"], 16) if "fault_rip" in nat else 0
    code = nat.get("code", 0)
    info = {"sig": sig, "code": code, "rip": rip}
    if nat["status"] == "timeout":
        info["why"] = "timeout"
        return None, info
    if nat["status"] != "fault":
        info["why"] = "status_" + nat["status"]
        return None, info
    if not trace.e or trace.e[-1][0] != rip:
        info["why"] = "no_trace_at_fault_rip"
        return None, info
    k = len(trace.e) - 1
    ins = trace.ins(k)[0]
    md = img.md
    if ins is None:
        info["why"] = "undecodable"
        return None, info
    info["insn"] = "%s %s" % (ins.mnemonic, ins.op_str)
    mn = ins.mnemonic
    target_reg, mode, mem_plan = None, None, None
    ea_f = None
    fault_addr = int(nat.get("fault_addr", "0x0"), 16)
    if sig == 8:
        op0 = ins.operands[0] if ins.operands and mn in ("div", "idiv") else None
        if op0 is not None and op0.type == CS_OP_REG:
            target_reg, mode = CANON[md.reg_name(op0.reg)], "small"
        elif ARITH and op0 is not None and op0.type == CS_OP_MEM:
            mode = "small"
            mem_plan = trace.load(k, trace.ea(k, op0), op0.size, 0, 1, True, [])
        else:
            info["why"] = "sigfpe_other"
            return None, info
    else:
        mop = pick_mem_operand(trace, k, ins, fault_addr) if mn != "lea" else None
        if mop is None and (mn in STRING_OPS or mn.startswith(("rep", "movs", "stos", "lods", "cmps", "scas"))):
            rsi, rdi = trace.g(k, "rsi"), trace.g(k, "rdi")
            if not plausible(rdi) and mn.startswith(("stos", "movs", "scas", "cmps")):
                target_reg, mode = "rdi", "ptr"
            elif not plausible(rsi) and mn.startswith(("movs", "lods", "cmps")):
                target_reg, mode = "rsi", "ptr"
            else:
                info["why"] = "string_op_other"
                return None, info
            ea_f = trace.g(k, target_reg)
        elif mop is None:
            info["why"] = "no_memory_operand:" + mn
            return None, info
        else:
            m = mop.mem
            if m.base == X86_REG_RIP or (not m.base and not m.index):
                info["why"] = "absolute_or_rip_address"
                return None, info
            ea_f = trace.ea(k, mop)
            base = CANON[md.reg_name(m.base)] if m.base else None
            idx = CANON[md.reg_name(m.index)] if m.index else None
            bv = trace.g(k, base) if base else 0
            xv = trace.g(k, idx) if idx else 0
            info["ea"] = ea_f
            aligned_op = mn in ALIGNED_SSE or mn.startswith(("movdq", "movap"))
            if code == 0x80 and plausible(ea_f) and ea_f & 15 and aligned_op:
                target_reg, mode = base, "align"
                if not base:
                    info["why"] = "misaligned_without_base"
                    return None, info
            elif base and base != "rsp" and not plausible(bv):
                target_reg, mode = base, "ptr"
            elif idx and not (0 <= xv * m.scale < 0x80000 or 0 <= (-xv * m.scale) & M64 < 0x80000):
                target_reg, mode = idx, "index"
            elif base and base != "rsp":
                target_reg, mode = base, "ptr"
            else:
                info["why"] = "stack_or_unknown_base"
                return None, info
    info["blame"] = target_reg
    info["mode"] = mode
    plan = mem_plan if mem_plan is not None else trace.follow(target_reg, k, 0, 1, mode in ("index", "small"))
    info["plan"] = {x: v for x, v in plan.items() if x != "chain"}
    info["chain"] = plan.get("chain", [])
    if plan["kind"] == "fail":
        info["why"] = plan["why"]
        return None, info
    cur = trace.g(k, target_reg) if target_reg else 0
    if mode in ("small", "index"):
        want = rng.randrange(0, 17)
    elif mode == "align":
        want = (cur + ((-ea_f) & 15)) & M64
    else:
        ea_new = SCRATCH + 0x20000 + (rng.randrange(0, 0x40000) & ~15)
        want = (ea_new - ((ea_f - cur) & M64)) & M64
    src_new = solve_source(plan, want, rng, mode)
    if src_new is None:
        info["why"] = "scale_not_divisible"
        return None, info
    nc = dict(case)
    nc["gpr"] = list(case["gpr"])
    nc["stk"] = list(case["stk"])
    nc["pokes"] = list(case["pokes"])
    if plan["kind"] == "reg":
        i = GIDX[plan["reg"]]
        if i == 4:
            info["why"] = "input_rsp"
            return None, info
        nc["gpr"][i] = src_new
        info["fix"] = "reg:%s=%x" % (plan["reg"], src_new)
        return nc, info
    ea, size = plan["ea"], plan["size"]
    if size == 4 and src_new >= 1 << 32 or size == 2 and src_new >= 1 << 16 or size == 1 and src_new >= 256:
        info["why"] = "narrow_load_needs_wide_value"
        return None, info
    in_scratch = SCRATCH <= ea and ea + 8 <= SCRATCH + SCRATCH_SZ
    in_image = IMAGE and img.writable(ea) and img.writable(ea + 7)
    if in_scratch or in_image:
        pk = [p for p in nc["pokes"] if p[0] != ea]
        if len(pk) >= POKE_MAX:
            info["why"] = "poke_limit"
            return None, info
        pk.append((ea, src_new))
        nc["pokes"] = pk
        info["fix"] = "poke:%x=%x%s" % (ea, src_new, "(image)" if in_image else "")
        return nc, info
    if STACK_TOP + 32 <= ea < STACK_TOP + 32 + 64 and (ea - STACK_TOP - 32) % 8 == 0 and size == 8:
        i = (ea - STACK_TOP - 32) // 8
        stk = nc["stk"] + [rng.getrandbits(64) for _ in range(max(0, i + 1 - len(nc["stk"])))]
        stk[i] = src_new
        nc["stk"] = stk
        info["fix"] = "stk%d=%x" % (i, src_new)
        return nc, info
    if STACK <= ea < STACK + STACK_SZ:
        info["why"] = "input_stack_garbage_slot"
    elif img.base <= ea < img.base + img.size:
        info["why"] = ("image_global_writable" if img.writable(ea) else "image_constant")
    else:
        info["why"] = "input_memory_outside_scratch"
    return None, info


def repair_loop(img, cases, pe, bindir, work, jobs, rounds, env, seed=0, log=print):
    """cases: list of case dicts. Every case is run natively (single-stepped, ring on); a faulting one is
    repaired and run again, up to `rounds` repairs. returns (final: {id: case that ran natively ok}, report)"""
    ringdir = os.path.join(work, "ring")
    os.makedirs(ringdir, exist_ok=True)
    e_tr = dict(env)
    e_tr["ORACLE_TRACE"] = "1"
    e_tr["ORACLE_RING_DIR"] = ringdir
    cur = {c["id"]: c for c in cases}
    report = {c["id"]: {"rounds": 0, "steps": [], "result": None} for c in cases}
    active = list(cur)
    final = {}
    for rnd in range(rounds + 1):
        if not active:
            break
        chunks = [[cur[i] for i in active[x::jobs]] for x in range(jobs)]
        res, rc = rd.run_side(os.path.join(bindir, "native_drv2"), pe, chunks, work, "rp%d" % rnd, e_tr)
        nxt = []
        stat = Counter()
        for cid in active:
            n = res.get(cid)
            rep = report[cid]
            if n is None:
                rep["result"] = "native_died"
                stat["native_died"] += 1
                continue
            if rnd == 0 and n["status"] != "ok":
                try:
                    rep["cause0"] = rd.fault_cause(img, cur[cid], n, cur[cid]["fn"])[0]
                except Exception as exc:  # classification only; never blocks the repair
                    rep["cause0"] = "classify_error:" + type(exc).__name__
            if n["status"] == "ok":
                rep["result"] = "ok"
                final[cid] = cur[cid]
                stat["ok"] += 1
                continue
            if rnd == rounds:
                rep["result"] = "rounds_exhausted"
                stat["rounds_exhausted"] += 1
                continue
            rp = os.path.join(ringdir, cid + ".ring")
            if not os.path.exists(rp):
                rep["result"] = "no_ring"
                stat["no_ring"] += 1
                continue
            total, ents = load_ring(rp)
            os.remove(rp)
            tr = Trace(img, total, ents)
            rng = random.Random((seed << 24) ^ zlib.crc32(cid.encode()) ^ (rnd << 8))
            nc, info = analyse(img, cur[cid], n, tr, rng)
            info["round"] = rnd
            info["chain"] = info.get("chain", [])[:6]
            rep["steps"].append(info)
            rep["rounds"] = rnd + 1
            if nc is None:
                rep["result"] = "unrepairable:" + info["why"]
                stat["unrepairable:" + info["why"]] += 1
                continue
            sigs = [(s.get("rip"), s.get("fix")) for s in rep["steps"][:-1]]
            if (info["rip"], info.get("fix")) in sigs:
                rep["result"] = "no_progress"
                stat["no_progress"] += 1
                continue
            cur[cid] = nc
            nxt.append(cid)
        log("repair round %d: ran %d cases: %s" % (rnd, len(active), dict(stat)))
        for f in glob.glob(os.path.join(work, "*-rp%d-*" % rnd)):
            os.remove(f)
        active = nxt
    return final, report


def trim_report(report):
    out = {}
    for cid, r in report.items():
        out[cid] = {"rounds": r["rounds"], "result": r["result"], "cause0": r.get("cause0"),
                    "steps": [{k: (hex(v) if k in ("rip", "ea") and isinstance(v, int) else v)
                               for k, v in s.items() if k != "plan"} for s in r["steps"]]}
    return out
