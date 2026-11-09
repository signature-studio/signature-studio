# Read-only probe (no value is changed): print the arguments of every LCMapStringW call and the first source characters.
import os
os.environ.setdefault("CHAIN_MAX", "3")
exec(open(os.path.join(os.path.dirname(os.environ["GDBI_PATCHES"]), "patches_chain.py")).read())


def probe(g):
    lcid, fl, src, cs = g.reg(1), g.reg(2), g.reg(8), g.reg(9)
    sp = g.reg(4)
    dst = g.ld(sp + 0x28, 8)
    cd = g.ld(sp + 0x30, 4)
    n = cs if 0 < cs < 8 else 8
    chars = " ".join("%04x" % g.ld(src + 2 * k, 2) for k in range(n)) if src else "-"
    print("   (probe) LCMapStringW lcid=%#x flags=%#x src=%#x cchSrc=%d dst=%#x cchDest=%d src-units=[%s]" % (lcid, fl, src, cs & 0xffffffff, dst, cd, chars))
    # f_37d75e8 keeps its first argument (the locale name pointer) in rbp (R[5]) across the call to the LocaleNameToLCID fallback
    nm = g.reg(5)
    print("   (probe) rbp=R[5]=%#x (locale name pointer of f_37d75e8; first units: %s)" % (nm, " ".join("%04x" % g.ld(nm + 2 * k, 2) for k in range(4)) if nm else "NULL"))


PATCHES["fle_imp_LCMapStringW"] = probe
