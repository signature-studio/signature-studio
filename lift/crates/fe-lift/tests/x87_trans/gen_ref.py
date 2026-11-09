#!/usr/bin/env python3
"""Reference table for F2XM1 and FSINCOS (decision 0043), independent of runtime/fle_x87.c.

Usage: python3 gen_ref.py OUT.txt        (deterministic: random.Random with a fixed seed)

Only Python's decimal (240 digits) and fractions (exact rationals) are used. No mpmath, no libm.
  2^x - 1  : Taylor series of exp(y) - 1 with y = x * ln2 (decimal.ln), cross-checked with decimal.exp.
  sin, cos : x is reduced with the SDM's 66-bit Pi (Vol. 1 8.3.8, Pi = 0x3243F6A8885A308D3 * 2^-64) to
             r = x - k * Pi/2 (k the nearest integer, exact rational), then Taylor series of sin r, cos r;
             the quadrant k mod 4 rotates them. The result is rounded to the 64-bit significand by RC
             (nearest even, down, up, toward zero) with the exponent range of f80, denormals included.
The rounding is checked to be decided far from a rounding boundary (a relative window of 1e-200).

One line per case:
  op cw pre fill xm xse | sw tw p0 p1 p2 p3 p4 p5 p6 p7
op: 0 F2XM1, 1 FSINCOS. cw: control word (PC random, RC 0..3, all exceptions masked). pre: C0/C2/C3 bits
loaded into the status word before the instruction. fill: number of registers loaded below the operand
(value k: m = 0x8000000000000000 | ((k+1) * 0x0123456789abcdef & 0x7fff...), se = 0x4000 + k, k=0 first).
After the "|": the status word, the tag word and the 8 physical registers (m:se, '-' when empty) as the
instruction must leave them. TOP before the operand is loaded is 0 (so TOP = (8 - fill - 1) & 7 after).
"""
import random
import sys
from decimal import Decimal, getcontext
from fractions import Fraction

getcontext().prec = 240
getcontext().Emax = 999999
getcontext().Emin = -999999

MASK64 = (1 << 64) - 1
D66 = 0x3243F6A8885A308D3  # Pi = D66 * 2^-64 ; Pi/2 = D66 * 2^-65
PI2 = Fraction(D66, 1 << 65)
EMIN = -16382


def check_pi66():
    # pi by Machin's formula in integers (independent), top 66 bits must be D66
    prec = 200
    one = 1 << prec

    def arctan_inv(n):
        s = t = one // n
        k = 1
        while t:
            t //= n * n
            k += 2
            s += (t // k) if (k // 2) % 2 == 0 else -(t // k)
        return s

    pi = 4 * (4 * arctan_inv(5) - arctan_inv(239))
    top = pi >> (prec - 64)
    assert top == D66, hex(top)


check_pi66()
LN2 = Decimal(2).ln()

IND = (0xC000000000000000, 0xFFFF)
ONE = (0x8000000000000000, 0x3FFF)


def classify(m, se):
    e = se & 0x7FFF
    if e == 0x7FFF:
        if not m >> 63:
            return "unsup"
        if (m << 1) & MASK64 == 0:
            return "inf"
        return "qnan" if (m >> 62) & 1 else "snan"
    if e == 0:
        return "zero" if m == 0 else "den"
    return "norm" if m >> 63 else "unsup"


def value(m, se):
    e = se & 0x7FFF
    v = Fraction(m) * Fraction(2) ** (max(e, 1) - 16383 - 63)
    return -v if se >> 15 else v


def tag_of(m, se):
    e = se & 0x7FFF
    if e == 0 and m == 0:
        return 1
    if e != 0 and e != 0x7FFF and (m >> 63):
        return 0
    return 2


def floor_log2(a):
    e = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** e > a:
        e -= 1
    if Fraction(2) ** (e + 1) <= a:
        e += 1
    return e


def round_f80(v, rc):
    """v nonzero rational. Returns (m, se, inexact, up, tiny)."""
    sign = v < 0
    a = abs(v)
    e = floor_log2(a)

    def rnd(qe):
        s = a / Fraction(2) ** qe
        n = s.numerator // s.denominator
        rem = s - n
        inexact = rem != 0
        if rc == 0:
            up = rem > Fraction(1, 2) or (rem == Fraction(1, 2) and (n & 1) == 1)
        elif rc == 1:
            up = inexact and sign
        elif rc == 2:
            up = inexact and not sign
        else:
            up = False
        return n + (1 if up else 0), inexact, up

    nu, _, _ = rnd(e - 63)
    tiny = Fraction(nu) * Fraction(2) ** (e - 63) < Fraction(2) ** EMIN
    qe = max(e, EMIN) - 63
    n, inexact, up = rnd(qe)
    sg = 0x8000 if sign else 0
    if e >= EMIN:
        if n == 1 << 64:
            n >>= 1
            e += 1
        assert e <= 16383
        return n, sg | (e + 16383), inexact, up, tiny
    if n == 0:
        return 0, sg, inexact, up, tiny
    if n >= 1 << 63:
        return n, sg | 1, inexact, up, tiny
    return n, sg, inexact, up, tiny


def round_checked(v, rc, below=False):
    """Rounds, and asserts that a relative window of 1e-200 around v rounds to the same result.
    below: the true value lies in (v (1 - 1e-200), v), v exactly representable (tiny arguments: sin r = r (1 - r^2/6),
    cos r = 1 - r^2/2 with r^2 < 1e-200). It is rounded as v (1 - 1e-300), which no boundary separates from the truth."""
    if below:
        return round_f80(v * (1 - Fraction(1, 10 ** 300)), rc)
    r = round_f80(v, rc)
    eps = Fraction(1, 10 ** 200)
    for w in (v * (1 - eps), v * (1 + eps)):
        assert round_f80(w, rc) == r, "rounding is decided too close to a boundary"
    return r


def dec(fr):
    return Decimal(fr.numerator) / Decimal(fr.denominator)


def expm1_series(y):
    s = t = y
    n = 2
    lim = abs(y) * Decimal(10) ** -235
    while True:
        t = t * y / n
        s += t
        if abs(t) < lim:
            break
        n += 1
    return s


def sincos_series(r):
    r2 = r * r
    t = r
    s = r
    n = 1
    lim = abs(r) * Decimal(10) ** -235
    while True:
        t = -t * r2 / ((2 * n) * (2 * n + 1))
        s += t
        if abs(t) < lim:
            break
        n += 1
    t = Decimal(1)
    c = Decimal(1)
    n = 1
    while True:
        t = -t * r2 / ((2 * n - 1) * (2 * n))
        c += t
        if abs(t) < Decimal(10) ** -235:
            break
        n += 1
    return s, c


def f2xm1_exact(x):
    """2^x - 1 as a Fraction (Decimal result, 235+ significant digits), x a nonzero Fraction, |x| < 1."""
    y = dec(x) * LN2
    v = expm1_series(y)
    if abs(y) > Decimal(10) ** -20:
        w = y.exp() - 1
        assert abs(v - w) < abs(v) * Decimal(10) ** -190, "expm1 series and decimal.exp disagree"
    return Fraction(v)


def sincos_exact(x):
    """(sin, cos) of the finite nonzero rational x (|x| < 2^63) per the 66-bit Pi reduction."""
    q = x / PI2
    k = q.numerator // q.denominator
    rem = q - k
    if rem > Fraction(1, 2) or (rem == Fraction(1, 2) and k & 1):
        k += 1
    r = x - k * PI2
    s, c = sincos_series(dec(r))
    assert abs(s * s + c * c - 1) < Decimal(10) ** -230, "sin^2 + cos^2 != 1"
    quad = k % 4
    if quad == 0:
        sn, cs = s, c
    elif quad == 1:
        sn, cs = c, -s
    elif quad == 2:
        sn, cs = -s, -c
    else:
        sn, cs = -c, s
    return Fraction(sn), Fraction(cs)


class Fpu:
    def __init__(self, fill, x, pre):
        self.reg = [None] * 8
        self.top = 0
        for k in range(fill):
            self.push(filler(k))
        self.push(x)
        self.pre = pre
        self.flags = 0
        self.c1 = 0
        self.c2 = None

    def push(self, v):
        self.top = (self.top - 1) & 7
        self.reg[self.top] = v

    def st0(self, v):
        self.reg[self.top] = v

    def line(self):
        # C0 and C3 are not changed by either instruction (undefined in the SDM, unchanged here);
        # F2XM1 leaves C2 unchanged as well (undefined), FSINCOS sets it.
        sw = (self.top << 11) | self.flags | (0x200 if self.c1 else 0)
        if self.c2 is None:
            sw |= self.pre & 0x4500
        else:
            sw |= (self.pre & 0x4100) | (0x400 if self.c2 else 0)
        tw = 0
        regs = []
        for p in range(8):
            v = self.reg[p]
            tw |= (3 if v is None else tag_of(*v)) << (2 * p)
            regs.append("-" if v is None else "%016x:%04x" % (v[0], v[1]))
        return "%04x %04x %s" % (sw, tw, " ".join(regs))


def filler(k):
    return (0x8000000000000000 | (((k + 1) * 0x0123456789ABCDEF) & 0x7FFFFFFFFFFFFFFF), 0x4000 + k)


IE, DE, UE, PE = 1, 2, 0x10, 0x20


def quiet(m, se):
    return (m | 0x4000000000000000, se)


def f2xm1_case(x, rc, fill, pre):
    """Returns the expected state line, or None when the operand is outside -1.0 .. 1.0 (not in the table)."""
    m, se = x
    f = Fpu(fill, x, pre)
    cl = classify(m, se)
    if cl == "snan":
        f.flags |= IE
        f.st0(quiet(m, se))
    elif cl == "qnan":
        f.st0(quiet(m, se))
    elif cl == "unsup":
        f.flags |= IE
        f.st0(IND)
    elif cl == "inf":
        return None
    elif cl == "zero":
        pass
    else:
        v = value(m, se)
        if abs(v) > 1:
            return None
        if cl == "den":
            f.flags |= DE
        if abs(v) == 1:
            f.st0((0x8000000000000000, 0x3FFF) if v > 0 else (0x8000000000000000, 0xBFFE))
        else:
            rm, rse, inexact, up, tiny = round_checked(f2xm1_exact(v), rc)
            f.st0((rm, rse))
            if inexact:
                f.flags |= PE
                if tiny:
                    f.flags |= UE
            f.c1 = 1 if up else 0
    return f.line()


def fsincos_case(x, rc, fill, pre):
    m, se = x
    f = Fpu(fill, x, pre)
    cl = classify(m, se)
    if cl in ("snan", "qnan"):
        if cl == "snan":
            f.flags |= IE
        q = quiet(m, se)
        f.st0(q)
        f.push(q)
        f.c2 = 0
    elif cl in ("unsup", "inf"):
        f.flags |= IE
        f.st0(IND)
        f.push(IND)
        f.c2 = 0
    elif cl == "zero":
        f.push(ONE)  # ST(1) keeps the zero (sin = x), ST(0) = 1
        f.c2 = 0
    else:
        v = value(m, se)
        if abs(v) >= 1 << 63:
            f.c2 = 1
            return f.line()
        if cl == "den":
            f.flags |= DE
        tiny_arg = abs(v) < Fraction(1, 10 ** 100)  # k = 0, r = x
        if tiny_arg:
            sn, cs = v, Fraction(1)
        else:
            sn, cs = sincos_exact(v)
        sm, sse, sinex, sup, stiny = round_checked(sn, rc, tiny_arg)
        cm, cse, cinex, cup, ctiny = round_checked(cs, rc, tiny_arg)
        f.st0((sm, sse))
        f.push((cm, cse))
        if sinex or cinex:
            f.flags |= PE
        if (sinex and stiny) or (cinex and ctiny):
            f.flags |= UE
        f.c1 = 1 if sup else 0
        f.c2 = 0
    return f.line()


# ---------------------------------------------------------------- inputs

rng = random.Random(0x87F2)


def rm():
    k = rng.randrange(8)
    if k == 0:
        return 1 << 63
    if k == 1:
        return MASK64
    if k == 2:
        return (1 << 63) | 1
    if k == 3:
        return (1 << 63) | rng.getrandbits(8)
    if k == 4:
        return ((1 << 63) | rng.getrandbits(63)) & ~((1 << rng.randrange(40)) - 1)
    return (1 << 63) | rng.getrandbits(63)


def sgn():
    return rng.choice((0, 0x8000))


def nearest(v):
    m, se, _, _, _ = round_f80(v, 0)
    return m, se


def neighbours(x):
    """x and the f80 values one step below and above it (same sign, normal)."""
    m, se = x
    out = [x]
    if m & MASK64 > (1 << 63):
        out.append((m - 1, se))
    if m < MASK64:
        out.append((m + 1, se))
    return out


def f2xm1_inputs():
    L = []
    z = (0, 0)
    L += [z, (0, 0x8000), (1 << 63, 0x3FFF), (1 << 63, 0xBFFF), (1 << 63, 0x3FFE), (1 << 63, 0xBFFE)]
    L += [(MASK64, 0x3FFE), (MASK64, 0xBFFE), ((1 << 63) + 1, 0x3FFF), (MASK64, 0x3FFD)]
    L += [(1, 0), (1, 0x8000), ((1 << 63) - 1, 0), ((1 << 63) - 1, 0x8000), (1 << 63, 0), (1 << 63, 0x8000)]
    L += [(1 << 63, 1), (1 << 63, 0x8001), (MASK64, 1), ((1 << 63) + 1, 1), (1 << 63, 2), (3, 0), (0x123456789, 0)]
    for e in (0x3FFF - 64, 0x3FFF - 65, 0x3FFF - 100, 0x3FFF - 1000, 0x3FFF - 10000, 0x3FFF - 16000, 0x3FFF - 16381, 0x3FFF - 16382):
        for s in (0, 0x8000):
            L += [(1 << 63, e | s), (MASK64, e | s), (rm(), e | s)]
    # NaN, SNaN, unsupported, pseudo
    L += [(0xC000000000000000, 0x7FFF), (0xC000000000000001, 0xFFFF), (0xFFFFFFFFFFFFFFFF, 0x7FFF), (0xE00000000000ABCD, 0xFFFF)]
    L += [(0x8000000000000001, 0x7FFF), (0xA000000000000000 | 5, 0xFFFF), (0xBFFFFFFFFFFFFFFF, 0x7FFF)]
    L += [(0x4000000000000000, 0x3FFF), (0x0000000000000001, 0x3FFE), (0x7FFFFFFFFFFFFFFF, 0x7FFF), (0, 0x7FFF), (0x1234, 0xFFFF)]
    # beyond the range (the table skips them; the abort is tested separately)
    L += [(1 << 63, 0x4000), (1 << 63, 0x7FFF), (1 << 63, 0xFFFF)]
    for _ in range(900):
        e = 0x3FFE - rng.randrange(0, 20)
        L.append((rm(), e | sgn()))
    for _ in range(500):
        e = 0x3FFE - rng.randrange(20, 70)
        L.append((rm(), e | sgn()))
    for _ in range(150):
        L.append((MASK64 - rng.randrange(0, 5000), 0x3FFE | sgn()))
    for _ in range(150):
        e = rng.randrange(1, 0x3FFE - 70)
        L.append((rm(), e | sgn()))
    for _ in range(120):
        L.append((rng.getrandbits(63), sgn()))  # denormals
    for _ in range(60):
        L.append(((1 << 63) | rng.getrandbits(63), sgn()))  # pseudo-denormals
    return L


def fsincos_inputs():
    L = []
    L += [(0, 0), (0, 0x8000), (1, 0), (1, 0x8000), ((1 << 63) - 1, 0), (1 << 63, 0), (1 << 63, 1), (MASK64, 1)]
    L += [(1 << 63, 0x3FFF), (1 << 63, 0xBFFF), (1 << 63, 0x4000), (1 << 63, 0x3FFE), (MASK64, 0x3FFE)]
    for e in (0x3FFF - 64, 0x3FFF - 100, 0x3FFF - 1000, 0x3FFF - 16000, 0x3FFF - 16381):
        for s in (0, 0x8000):
            L += [(1 << 63, e | s), (rm(), e | s)]
    # near multiples of Pi/2 (true pi from the decimal string) and of the SDM's Pi/2
    pi = Fraction(Decimal("3.14159265358979323846264338327950288419716939937510582097494459230781640628620899862803482534211706798214808651328230664709384460955058223172535940812848111745028410270193852110555964462294895493038196"))
    for k in list(range(1, 40)) + [63, 64, 100, 127, 128, 1000, 4096, 65536, 10 ** 6, 2 ** 20 + 1, 2 ** 30 + 7, 2 ** 40 + 3, 2 ** 52 + 1, 2 ** 60 + 5, 2 ** 61 - 1]:
        for base in (pi / 2 * k, PI2 * k):
            if base >= 1 << 63:
                continue
            for x in neighbours(nearest(base)):
                L += [x, (x[0], x[1] | 0x8000)]
    # 3*pi/8 and pi/4 boundaries of the reduction
    for base in (PI2 / 2, PI2 * 3 / 2, PI2 * 5 / 2):
        for x in neighbours(nearest(base)):
            L += [x, (x[0], x[1] | 0x8000)]
    # 2^63 boundary and beyond
    L += [(MASK64, 0x403D), (MASK64, 0xC03D), ((1 << 63) + 1, 0x403D), (1 << 63, 0x403E), (1 << 63, 0xC03E), ((1 << 63) + 1, 0x403E), (MASK64, 0x403E), (1 << 63, 0x403F), (1 << 63, 0x4040)]
    L += [(1 << 63, 0x7FFE), (MASK64, 0xFFFE), (rm(), 0x5000), (rm(), 0xD000), (1 << 63, 0x4100)]
    # inf, NaN, unsupported
    L += [(1 << 63, 0x7FFF), (1 << 63, 0xFFFF), (0xC000000000000000, 0x7FFF), (0xC000000000000123, 0xFFFF), (0xFFFFFFFFFFFFFFFF, 0x7FFF)]
    L += [(0x8000000000000001, 0x7FFF), (0xA000000000000000 | 9, 0xFFFF), (0x4000000000000000, 0x3FFF), (1, 0x3FFE), (0x7FFFFFFFFFFFFFFF, 0x7FFF), (0, 0x7FFF), (0x1234, 0xFFFF)]
    for _ in range(500):
        L.append((rm(), (0x3FFF - 1 - rng.randrange(0, 7)) | sgn()))
    for _ in range(500):
        L.append((rm(), (0x3FFF + rng.randrange(0, 3)) | sgn()))
    for _ in range(400):
        L.append((rm(), (0x3FFF + 3 + rng.randrange(0, 40)) | sgn()))
    for _ in range(200):
        L.append((rm(), (0x3FFF + 43 + rng.randrange(0, 20)) | sgn()))
    for _ in range(300):
        L.append((rm(), (0x3FFF - 8 - rng.randrange(0, 70)) | sgn()))
    for _ in range(100):
        L.append((rm(), rng.randrange(1, 0x3FFF - 70) | sgn()))
    for _ in range(100):
        L.append((rng.getrandbits(63), sgn()))
    for _ in range(40):
        L.append(((1 << 63) | rng.getrandbits(63), sgn()))
    return L


def main():
    out = open(sys.argv[1], "x") if len(sys.argv) > 1 else sys.stdout
    n = 0
    pres = [0, 0x4500, 0x4100, 0x0400, 0x0100, 0x4000]
    for op, inputs, fn in ((0, f2xm1_inputs(), f2xm1_case), (1, fsincos_inputs(), fsincos_case)):
        for i, x in enumerate(inputs):
            for rc in range(4):
                fill = rng.choice((0, 0, 1, 2, 3, 5, 7 if op == 0 else 6))
                pre = pres[rng.randrange(len(pres))]
                cw = 0x007F | (rng.randrange(4) << 8) | (rc << 10)
                exp = fn(x, rc, fill, pre)
                if exp is None:
                    continue
                out.write("%d %04x %04x %d %016x %04x | %s\n" % (op, cw, pre, fill, x[0], x[1], exp))
                n += 1
    out.close() if out is not sys.stdout else None
    print("rows", n, file=sys.stderr)


if __name__ == "__main__":
    main()
