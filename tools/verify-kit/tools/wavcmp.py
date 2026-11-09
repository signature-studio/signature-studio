#!/usr/bin/env python3
"""wavcmp.py A.wav B.wav [--bands]  (ci1, 2026-11-07): frame-by-frame comparison of two 16-bit stereo WAVs (RIFF chunks read, JUNK skipped): frames, differing frames, max |diff|, RMS of the difference, first / last differing frame, per 10 s counts. numpy only."""
import sys, struct, numpy as np
def load(p):
    b = open(p, 'rb').read(); pos = 12
    while pos < len(b) - 8:
        cid = b[pos:pos + 4]; sz = struct.unpack('<I', b[pos + 4:pos + 8])[0]
        if cid == b'fmt ': fmt = struct.unpack('<HHIIHH', b[pos + 8:pos + 24])
        if cid == b'data': return fmt, np.frombuffer(b[pos + 8:pos + 8 + sz], dtype='<i2').reshape(-1, fmt[1])
        pos += 8 + sz + (sz & 1)
fa, A = load(sys.argv[1]); fb, B = load(sys.argv[2])
print('A', sys.argv[1], fa, 'frames', len(A)); print('B', sys.argv[2], fb, 'frames', len(B), '(B - A = %+d frames)' % (len(B) - len(A)))
n = min(len(A), len(B)); d = B[:n].astype(np.int64) - A[:n].astype(np.int64); nz = np.nonzero(np.any(d != 0, axis=1))[0]
print('compared frames', n, 'differing', len(nz), '(%.4f%%)' % (100.0 * len(nz) / n))
if len(nz):
    print('first differing frame %d (%.3f s), last %d (%.3f s)' % (nz[0], nz[0] / 44100, nz[-1], nz[-1] / 44100), '| max |diff|', int(np.abs(d).max()), '| RMS diff %.4f' % np.sqrt((d.astype(np.float64) ** 2).mean()), '| RMS A %.2f' % np.sqrt((A[:n].astype(np.float64) ** 2).mean()))
    if len(nz) <= 40: print('differing frames:', nz.tolist())
    else:
        for lo in range(0, int(n / 44100) + 10, 10):
            s = slice(lo * 44100, min((lo + 10) * 44100, n)); c = int(np.count_nonzero(np.any(d[s] != 0, axis=1)))
            if c: print('  %3d-%3d s: %d differing, RMS %.3f' % (lo, lo + 10, c, np.sqrt((d[s].astype(np.float64) ** 2).mean())))
