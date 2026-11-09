#!/usr/bin/env python3
"""Checks the 256-bit intermediate values of runtime/fle_x87.c (F2XM1, FSINCOS; decision 0043) against
Python's decimal (240 digits), independent of the code under test.  Reads the output of the C probe on stdin:

  LN2 sign e m7 .. m0
  EXPM1 sign y_hex y_shift  | sign e m7 .. m0      (y = y_hex * 2^y_shift, the argument of the series)
  SINCOS sign r_hex r_shift | sign e m7 .. m0 | sign e m7 .. m0   (r = r_hex * 2^r_shift; sine, cosine)

bf value = (-1)^sign * M * 2^(e - 255), M = m7:m6:...:m0 (eight 32-bit limbs, bit 255 set), 0 if all limbs are 0.
Constant: 0 <= ln2 - LN2 < 2^-256 (truncated). Series: relative error below 2^-248 (a few units of the
last place of the 256-bit significand after the rounding errors of about 40 operations).
Exit status 0 and "ok N" when all agree."""
import sys
from decimal import Decimal, getcontext
from fractions import Fraction

getcontext().prec = 260


def bf(tokens):
    sign = int(tokens[0])
    e = int(tokens[1])
    limbs = [int(t, 16) for t in tokens[2:10]]
    m = 0
    for l in limbs:
        m = (m << 32) | l
    if m == 0:
        return Fraction(0)
    v = Fraction(m) * Fraction(2) ** (e - 255)
    return -v if sign else v


def dec(fr):
    return Decimal(fr.numerator) / Decimal(fr.denominator)


def series_expm1(y):
    s = t = y
    n = 2
    while True:
        t = t * y / n
        s += t
        if abs(t) < abs(y) * Decimal(10) ** -255:
            break
        n += 1
    return s


def series_sincos(r):
    r2 = r * r
    t = s = r
    n = 1
    while True:
        t = -t * r2 / ((2 * n) * (2 * n + 1))
        s += t
        if abs(t) < abs(r) * Decimal(10) ** -255:
            break
        n += 1
    t = c = Decimal(1)
    n = 1
    while True:
        t = -t * r2 / ((2 * n - 1) * (2 * n))
        c += t
        if abs(t) < Decimal(10) ** -255:
            break
        n += 1
    return s, c


def rel_ok(got, want, bits=248):
    if want == 0:
        return got == 0
    return abs(got - want) <= abs(want) * Fraction(1, 1 << bits)


def main():
    ln2 = Decimal(2).ln()
    n = 0
    for line in sys.stdin:
        parts = [p.split() for p in line.split("|")]
        head = parts[0]
        if head[0] == "LN2":
            got = bf(head[1:])
            d = Fraction(ln2) - got
            assert 0 <= d < Fraction(1, 1 << 256), "LN2 constant is not ln 2 truncated to 256 bits: %s" % (float(d * (1 << 256)))
        elif head[0] == "EXPM1":
            y = Fraction(int(head[2], 16)) * Fraction(2) ** int(head[3])
            if head[1] == "1":
                y = -y
            want = Fraction(series_expm1(dec(y)))
            got = bf(parts[1])
            assert rel_ok(got, want), "expm1(%s): got %s want %s" % (head[2:], float(got), float(want))
        elif head[0] == "SINCOS":
            r = Fraction(int(head[2], 16)) * Fraction(2) ** int(head[3])
            if head[1] == "1":
                r = -r
            s, c = series_sincos(dec(r))
            gs, gc = bf(parts[1]), bf(parts[2])
            assert rel_ok(gs, Fraction(s)), "sin(%s): got %s want %s" % (head[2:], float(gs), float(s))
            assert rel_ok(gc, Fraction(c)), "cos(%s): got %s want %s" % (head[2:], float(gc), float(c))
        else:
            raise SystemExit("bad line: " + line)
        n += 1
    print("ok", n)


if __name__ == "__main__":
    main()
