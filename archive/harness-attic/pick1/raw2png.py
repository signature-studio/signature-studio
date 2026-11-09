#!/usr/bin/env python3
"""pick1: FLE_G3_DUMP_DIR raw surface (B,G,R,X per pixel, top-down; name <snapshot>-lid<N>.bgra) -> PNG (no PIL). usage: raw2png.py RAW WIDTH HEIGHT OUT.png [--crop X Y W H] [--scale N]"""
import sys, zlib, struct
import numpy as np
a = sys.argv[1:]
raw, w, h, out = a[0], int(a[1]), int(a[2]), a[3]
d = np.fromfile(raw, dtype=np.uint8)
d = d[: w * h * 4].reshape(h, w, 4)[:, :, [2, 1, 0]]
if '--crop' in a:
    i = a.index('--crop'); x, y, cw, ch = [int(v) for v in a[i + 1:i + 5]]; d = d[y:y + ch, x:x + cw]
if '--scale' in a:
    s = int(a[a.index('--scale') + 1]); d = d.repeat(s, 0).repeat(s, 1)
hh, ww = d.shape[:2]
rows = b''.join(b'\x00' + d[r].tobytes() for r in range(hh))
def chunk(t, b): c = struct.pack('>I', len(b)) + t + b; return c + struct.pack('>I', zlib.crc32(t + b) & 0xffffffff)
open(out, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', ww, hh, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows, 3)) + chunk(b'IEND', b''))
