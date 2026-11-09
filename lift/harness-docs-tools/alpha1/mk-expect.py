#!/usr/bin/env python3
"""alpha1 (decision 0244): the expected pixel table of lift/harness/alpha1/glass_seq.c, derived from the formula of Microsoft Learn (BLENDFUNCTION / UpdateLayeredWindow, AC_SRC_ALPHA): Dst = Src + (1 - Src.A) * Dst
with Src premultiplied (alpha 0..1 = A / 255), exact rational arithmetic, rounded to the nearest integer (halves up; no tie is possible since the denominator 255 is odd), each channel clamped at 255.
It does not read the C code. usage: mk-expect.py   -> prints the C initialisers."""
from fractions import Fraction as F
import math
def over(src, a, d, sca=255):
    # src = premultiplied (r, g, b) as stored; a = A; d = destination (r, g, b); sca = SourceConstantAlpha
    out = []
    for c, dd in zip(src, d):
        s = F(c * sca, 255)
        aa = F(a * sca, 255) / 255
        v = s + (1 - aa) * dd
        out.append(min(255, math.floor(v + F(1, 2))))
    return tuple(out)
# (A, R, G, B) premultiplied
PIX = [
    (255, 200, 100, 50), (0, 0, 0, 0), (128, 128, 64, 32), (255, 0, 255, 0), (200, 100, 100, 100), (0, 7, 8, 9), (1, 0, 0, 0), (254, 0, 0, 0),
    (64, 64, 0, 0), (64, 0, 0, 64), (100, 50, 50, 50), (255, 1, 2, 3), (128, 128, 128, 128), (128, 0, 0, 0), (2, 1, 1, 1), (253, 250, 250, 250),
]
BG = (5, 7, 10)
RED = (255, 0, 0)
print('static const uint8_t PIX[16][4] = {   /* A, R, G, B (premultiplied, as BitBlt\'d from the DIB) */')
print('    ' + ', '.join('{%d, %d, %d, %d}' % p for p in PIX))
print('};')
for nm, under in (('OVER_RED', RED), ('OVER_BG', BG)):
    print('static const uint8_t %s[16][3] = {   /* Dst = Src + (1 - A/255) * Dst, Dst = %s */' % (nm, under))
    print('    ' + ', '.join('{%d, %d, %d}' % over(p[1:], p[0], under) for p in PIX))
    print('};')

# ---- layered windows (UpdateLayeredWindow / SetLayeredWindowAttributes), the same pixels ----
def table(name, rows, comment):
    print('static const uint8_t %s[16][3] = {   /* %s */' % (name, comment))
    print('    ' + ', '.join('{%d, %d, %d}' % r for r in rows))
    print('};')
C = [p[1:] for p in PIX]              # the colour of a pixel when its A byte is not an alpha (a plain colour)
def solid(c): return tuple(c)
KEYED = {(0, 0, 0)}                   # the colour key (0, 0, 0): pixels 1, 6, 7, 13
table('ULW_PM_128', [over(p[1:], p[0], BG, 128) for p in PIX], 'ULW_ALPHA, AC_SRC_ALPHA, SourceConstantAlpha 128: Src*128/255 in all four channels, then Src + (1 - Src.A) * Dst; Dst = (5, 7, 10)')
table('ULW_CONST_100', [over(c, 255, BG, 100) for c in C], 'ULW_ALPHA, no AC_SRC_ALPHA (the A byte of the DIB is not used), SourceConstantAlpha 100: Dst = Src * 100/255 + Dst * (1 - 100/255)')
table('ULW_OPAQUE', [solid(c) for c in C], 'ULW_OPAQUE: Dst = Src')
table('ULW_KEY', [BG if c in KEYED else solid(c) for c in C], 'ULW_COLORKEY (0, 0, 0): the key colour is transparent, the rest opaque')
table('SLWA_A200', [over(c, 255, BG, 200) for c in C], 'LWA_ALPHA 200: Dst = Src * 200/255 + Dst * (1 - 200/255)')
table('SLWA_KEY', [BG if c in KEYED else solid(c) for c in C], 'LWA_COLORKEY (0, 0, 0)')
table('SLWA_BOTH', [BG if c in KEYED else over(c, 255, BG, 200) for c in C], 'LWA_COLORKEY (0, 0, 0) | LWA_ALPHA 200')
# two layered windows over each other: below SLWA alpha 200 over the background, above ULW premultiplied (SCA 255) of the pixel 15 - i
below = [over(c, 255, BG, 200) for c in C]
table('TWO_LAYERS', [over(PIX[15 - i][1:], PIX[15 - i][0], below[i], 255) for i in range(16)], 'above: ULW AC_SRC_ALPHA pixel 15 - i, SCA 255; below: LWA_ALPHA 200 pixel i; background (5, 7, 10)')
LIGHT = (240, 200, 120)
table('SLWA_A77_LIGHT', [over(c, 255, LIGHT, 77) for c in C], 'LWA_ALPHA 77 over a window of (240, 200, 120): Dst = Src * 77/255 + Dst * (1 - 77/255)')
table('ULW_PM_77_LIGHT', [over(p[1:], p[0], LIGHT, 77) for p in PIX], 'ULW_ALPHA, AC_SRC_ALPHA, SourceConstantAlpha 77 over (240, 200, 120)')
