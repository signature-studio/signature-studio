#!/usr/bin/env python3
# pngdiff.py A.png B.png [OUT.png]: the pixels that differ between two screenshots of the same size (decision 0277 alpha7 side by side): count, bounding box, the 120x120 cells with the most differences; OUT.png = the difference (white where they differ, the picture of A dimmed elsewhere)
import sys, zlib, struct, numpy as np
def dec(p):
    b = open(p, 'rb').read(); assert b[:8] == b'\x89PNG\r\n\x1a\n'; i = 8; idat = b''; w = h = bd = ct = il = 0
    while i < len(b):
        n, t = struct.unpack('>I4s', b[i:i + 8]); d = b[i + 8:i + 8 + n]; i += 12 + n
        if t == b'IHDR': w, h, bd, ct, _, _, il = struct.unpack('>IIBBBBB', d)
        elif t == b'IDAT': idat += d
    assert bd == 8 and il == 0 and ct in (2, 6), (bd, ct, il)
    bpp = 3 if ct == 2 else 4; raw = zlib.decompress(idat); stride = w * bpp; out = np.zeros((h, stride), np.uint8); prev = np.zeros(stride, np.int32)
    for y in range(h):
        f = raw[y * (stride + 1)]; line = np.frombuffer(raw, np.uint8, stride, y * (stride + 1) + 1).astype(np.int32)
        if f == 0: cur = line
        elif f == 1:
            cur = line.copy()
            for x in range(bpp, stride): cur[x] = (cur[x] + cur[x - bpp]) & 255
        elif f == 2: cur = (line + prev) & 255
        elif f == 3:
            cur = line.copy()
            for x in range(stride): cur[x] = (cur[x] + (((cur[x - bpp] if x >= bpp else 0) + prev[x]) >> 1)) & 255
        else:
            cur = line.copy()
            for x in range(stride):
                a = cur[x - bpp] if x >= bpp else 0; bb = prev[x]; c = prev[x - bpp] if x >= bpp else 0; pa = abs(bb - c); pb = abs(a - c); pc = abs(a + bb - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (bb if pb <= pc else c); cur[x] = (cur[x] + pr) & 255
        out[y] = cur; prev = cur
    return out.reshape(h, w, bpp)[:, :, :3].astype(int)
def enc(a):
    h, w, _ = a.shape; raw = b''.join(b'\x00' + a[y].astype(np.uint8).tobytes() for y in range(h))
    ch = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + ch(b'IDAT', zlib.compress(raw, 6)) + ch(b'IEND', b'')
A = dec(sys.argv[1]); B = dec(sys.argv[2])
if A.shape != B.shape: print('different sizes', A.shape, B.shape); sys.exit(2)
m = (np.abs(A - B).max(axis=2) > 0); n = int(m.sum()); h, w = m.shape
print(f'{sys.argv[1]} vs {sys.argv[2]}: {w}x{h}, differing pixels {n} ({100.0 * n / (w * h):.3f} %)')
if n:
    ys, xs = np.where(m); print(f'bounding box x {xs.min()}..{xs.max()}, y {ys.min()}..{ys.max()}')
    cells = {}
    for (y, x) in zip(ys[::7], xs[::7]): cells[(y // 120, x // 120)] = cells.get((y // 120, x // 120), 0) + 7
    for (cy, cx), c in sorted(cells.items(), key=lambda kv: -kv[1])[:10]: print(f'  cell x {cx * 120}-{cx * 120 + 119}, y {cy * 120}-{cy * 120 + 119}: ~{c} px')
if len(sys.argv) > 3:
    o = (A * 0.35).astype(int); o[m] = [255, 255, 255]; open(sys.argv[3], 'wb').write(enc(o)); print('wrote', sys.argv[3])
