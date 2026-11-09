# TEMPORARY values for the measurement chain of docs/investigations/stage23-two-module-real-dll.md.
# NONE of these values is a decision and none is runtime behaviour: each is put in by gdb at a named stop of the runtime only to see where
# the next stop is. Used with gdb_imports.py:  GDBI_PATCHES=this file  CHAIN_MAX=<n>  (patches of stage 1..n are active; stage k covers the stop
# that is reached after stage k-1).
import os

import gdb

CHAIN_MAX = int(os.environ.get("CHAIN_MAX", "0"))
LOGP = print
# Every patch below acts ONLY on an input for which the runtime stops (the guard is written next to each patch); any other call is left to the
# runtime, so the measurement does not change what the runtime would answer.
ENV_SBCS = (1252, 437)          # fle_emulated_locale.acp / .oemcp (decisions 0016, 0042); read again from the binary below


def _emu():
    return int(gdb.parse_and_eval("fle_emulated_locale.lcid")), int(gdb.parse_and_eval("fle_emulated_locale.acp")), int(gdb.parse_and_eval("fle_emulated_locale.oemcp"))


def stage1_getstartupinfow(g):
    # Stop A: fle_imp_GetStartupInfoW stops because the caller's cb ([rcx]) is not 0x68. The CRT passes an uninitialised stack buffer.
    cb = g.ld(g.reg(1), 4)
    if cb == 0x68:
        return None
    LOGP("   (stage 1) GetStartupInfoW: [rcx]=%#x before the patch, rcx=%#x; writing cb=0x68 so that the runtime's own answer is given" % (cb, g.reg(1)))
    g.st(g.reg(1), 4, 0x68)


def stage2_mbtwc(g):
    # Stop B: fle_imp_MultiByteToWideChar stops on a byte of 0x80 or above in a single-byte code page (no table on the MSDN pages).
    # Guard: code page is an environment single-byte page and the source (cb bytes, or NUL-terminated for cb = -1) holds a byte >= 0x80.
    # TEMPORARY: convert every byte b to the code unit U+00bb (identity, i.e. Latin-1) and return the count. This is NOT the cp1252 table.
    cp, fl, src, cb = g.reg(1) & 0xffffffff, g.reg(2), g.reg(8), g.reg(9)
    cb = cb - (1 << 32) if cb >= 1 << 31 else cb
    sp = g.reg(4)
    dst = g.ld(sp + 0x28, 8)
    cch = g.ld(sp + 0x30, 4)
    _, acp, oem = _emu()
    if cp not in (acp, oem) or not src:
        return None
    n = cb
    if cb == -1:
        n = 0
        while g.ld(src + n, 1) != 0:
            n += 1
        n += 1
    if n <= 0 or not any(g.ld(src + k, 1) >= 0x80 for k in range(n)):
        return None
    LOGP("   (stage 2) MultiByteToWideChar cp=%d flags=%#x src=%#x cb=%d dst=%#x cch=%d first-bytes=%s" % (
        cp, fl, src, cb, dst, cch, " ".join("%02x" % g.ld(src + k, 1) for k in range(min(n, 4)))))
    if cch == 0:                        # count query: one unit per byte (single-byte code page)
        g.setreg(0, n)
        return "skip"
    if dst and cch >= n:
        for k in range(n):
            g.st(dst + 2 * k, 2, g.ld(src + k, 1))
        g.setreg(0, n)
        return "skip"
    LOGP("   (stage 2) buffer too small: not patched")


def stage3_getstringtypew(g):
    # Stop C (if reached): GetStringTypeW is a named stop (decision 0078 section 3: the class of a character is Unicode data).
    # TEMPORARY: report success with type 0 for every unit.
    ctype, src, cch, dst = g.reg(1), g.reg(2), g.reg(8), g.reg(9)
    n = cch if cch < 0x80000000 else 0
    LOGP("   (stage 3) GetStringTypeW ctype=%#x src=%#x cch=%d dst=%#x: writing type 0 for each unit, returning 1" % (ctype, src, cch, dst))
    for k in range(n):
        g.st(dst + 2 * k, 2, 0)
    g.setreg(0, 1)
    return "skip"


def stage4_lcmapstringw(g):
    # Stop D: fle_imp_LCMapStringW stops on lcid 0 (the CRT's LocaleNameToLCID(NULL) fallback returns 0; fle_locale_resolve has no rule for 0).
    # TEMPORARY: treat the call as the emulated locale: ASCII case map, other units unchanged; return the count of units.
    # Guard: the locale is one that fle_locale_resolve does not know (0x400, 0x800, the emulated LCID, 0x7f are known), or the source holds a
    # unit >= 0x80 (the runtime stops on that too), and the flags are a single case flag (the runtime stops on others: not patched).
    lcid, fl, src, cs = g.reg(1) & 0xffffffff, g.reg(2) & 0xffffffff, g.reg(8), g.reg(9) & 0xffffffff
    sp = g.reg(4)
    dst = g.ld(sp + 0x28, 8)
    cd = g.ld(sp + 0x30, 4)
    n = cs
    known = {0x400, 0x800, 0x7f, _emu()[0]}
    if src == 0 or cs == 0 or cs >= 1 << 31:
        return None
    if lcid in known and not any(g.ld(src + 2 * k, 2) >= 0x80 for k in range(n)):
        return None
    if fl & ~0x300 and fl not in (0x100, 0x200):
        LOGP("   (stage 4) LCMapStringW flags=%#x is not a single case flag: not patched" % fl)
        return None
    LOGP("   (stage 4) LCMapStringW lcid=%#x flags=%#x cchSrc=%d cchDest=%d: ASCII case map, others unchanged" % (lcid, fl, cs, cd))
    if cd == 0:
        g.setreg(0, n)
        return "skip"
    for k in range(n):
        c = g.ld(src + 2 * k, 2)
        if fl & 0x200 and 0x61 <= c <= 0x7a:
            c -= 0x20
        if fl & 0x100 and 0x41 <= c <= 0x5a:
            c += 0x20
        g.st(dst + 2 * k, 2, c)
    g.setreg(0, n)
    return "skip"


def stage5_wctmb(g):
    # Stop E: fle_imp_WideCharToMultiByte stops on a UTF-16 unit of 0x80 or above for a single-byte code page (no table on the MSDN pages).
    # TEMPORARY: unit u <= 0xff -> byte u (identity, i.e. Latin-1, NOT cp1252); a unit above 0xff -> '?' (0x3f). Returns the count.
    # Guard: an environment single-byte code page and a source unit >= 0x80 (the case in which the runtime stops).
    cp, fl, src, cch = g.reg(1) & 0xffffffff, g.reg(2), g.reg(8), g.reg(9) & 0xffffffff
    sp = g.reg(4)
    dst = g.ld(sp + 0x28, 8)
    cb = g.ld(sp + 0x30, 4)
    _, acp, oem = _emu()
    if cp not in (acp, oem) or not src or cch == 0 or (cch >> 31 and cch != 0xffffffff):
        return None
    nn = cch
    if cch == 0xffffffff:
        nn = 0
        while g.ld(src + 2 * nn, 2) != 0:
            nn += 1
        nn += 1
    if not any(g.ld(src + 2 * k, 2) >= 0x80 for k in range(nn)):
        return None
    LOGP("   (stage 5) WideCharToMultiByte cp=%d flags=%#x src=%#x cchWide=%d dst=%#x cbMulti=%d" % (cp, fl, src, cch, dst, cb))
    if cch == 0xffffffff:
        cch = nn
    if cb == 0:
        g.setreg(0, cch)
        return "skip"
    for k in range(cch):
        u = g.ld(src + 2 * k, 2)
        g.st(dst + k, 1, u if u <= 0xff else 0x3f)
    g.setreg(0, cch)
    return "skip"


def _wname(g, p):
    s = ""
    for k in range(64):
        c = g.ld(p + 2 * k, 2)
        if c == 0:
            break
        s += chr(c) if 32 <= c < 127 else "?"
    return s


LLW_ABSENT = set(os.environ.get("CHAIN_ABSENT", "avrt.dll").lower().split(","))


def stage6_loadlibraryw(g):
    # Stop F: LoadLibraryW of an external module that is neither a virtual module nor in fle_load_absent[] (decision 0029: avrt, PowrProf,
    # PSAPI, RICHED20, URLMON, WebView2Loader stop; the decision names adding a name to fle_load_absent[] as a PM decision).
    # TEMPORARY: answer NULL (the guest's own code checks every LoadLibraryW result: external-modules-usage.md 2.1) and last error 126 for
    # the names in CHAIN_ABSENT (default avrt.dll); every other name is left to the runtime.
    # The guest's name has no extension ("avrt"): the runtime appends ".dll" (normalize()); do the same for the comparison.
    name = _wname(g, g.reg(1)).lower()
    if not name.endswith(".dll"):
        name += ".dll"
    if name in LLW_ABSENT:
        LOGP("   (stage 6) LoadLibraryW(%r): TEMPORARY answer NULL (R[0]=0); last error not set by this patch" % name)
        g.setreg(0, 0)
        return "skip"
    LOGP("   (stage 6) LoadLibraryW(%r): left to the runtime" % name)


STAGES = {
    1: ("fle_imp_GetStartupInfoW", stage1_getstartupinfow),
    2: ("fle_imp_MultiByteToWideChar", stage2_mbtwc),
    3: ("fle_imp_GetStringTypeW", stage3_getstringtypew),
    4: ("fle_imp_LCMapStringW", stage4_lcmapstringw),
    5: ("fle_imp_WideCharToMultiByte", stage5_wctmb),
    6: ("fle_imp_LoadLibraryW", stage6_loadlibraryw),
}
EXTRA = {}
try:
    exec(open(os.environ["CHAIN_EXTRA"]).read(), {"STAGES": STAGES, "LOGP": LOGP}) if os.environ.get("CHAIN_EXTRA") else None
except KeyError:
    pass

PATCHES = {}
for k in sorted(STAGES):
    if k <= CHAIN_MAX:
        name, fn = STAGES[k]
        prev = PATCHES.get(name)
        if prev is None:
            PATCHES[name] = fn
        else:
            PATCHES[name] = (lambda a, b: (lambda g: a(g) or b(g)))(prev, fn)
