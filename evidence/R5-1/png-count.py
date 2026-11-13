# Counts the pixels of the canvas screenshots (evidence/R5-1/g4-run<N>.png) that differ from the background colour (the most frequent colour).
# Python 3 standard library only (zlib, struct). Usage: python3 evidence/R5-1/png-count.py > evidence/R5-1/png-count.txt
import collections
import glob
import os
import struct
import zlib

here = os.path.dirname(os.path.abspath(__file__))


def read_png(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', path
    i, idat, ihdr = 8, b'', None
    while i < len(data):
        (ln,) = struct.unpack('>I', data[i:i + 4])
        kind, body = data[i + 4:i + 8], data[i + 8:i + 8 + ln]
        i += 12 + ln
        if kind == b'IHDR':
            ihdr = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        elif kind == b'IEND':
            break
    w, h, bitdepth, colortype, _, _, interlace = ihdr
    assert bitdepth == 8 and interlace == 0 and colortype in (2, 6), (bitdepth, colortype, interlace)
    channels = {2: 3, 6: 4}[colortype]
    bpp = channels
    stride = w * channels
    raw = zlib.decompress(idat)
    rows, prev, pos = [], bytearray(stride), 0
    for _ in range(h):
        f = raw[pos]
        line = bytearray(raw[pos + 1:pos + 1 + stride])
        pos += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            up = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + up) & 255
            elif f == 3:
                line[x] = (line[x] + ((a + up) >> 1)) & 255
            elif f == 4:
                p = a + up - c
                pa, pb, pc = abs(p - a), abs(p - up), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (up if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, channels, rows


for path in sorted(glob.glob(os.path.join(here, 'g4-run*.png'))):
    w, h, channels, rows = read_png(path)
    counts = collections.Counter()
    for r in rows:
        for x in range(0, len(r), channels):
            counts[r[x:x + channels]] += 1
    total = w * h
    bg, bg_n = counts.most_common(1)[0]
    print(f"{os.path.basename(path)}: {w}x{h} pixels={total} background={tuple(bg)} backgroundPixels={bg_n} nonBackgroundPixels={total - bg_n} distinctColours={len(counts)}")
