"""Sanity checks of a dumped output (float32, per block: left frames then right frames; from `vst-native --out FILE`). usage: analyze_out.py FILE FRAMES RATE [SCALE]
Prints: finite, min/max, RMS per channel, the strongest FFT bin of the left channel over the last 4 blocks (the input is 440 Hz on the left, 660 Hz on the right), and the
envelope (RMS of 4096-frame blocks). Nothing here compares with a real OTT: there is no reference output."""
import sys, numpy as np
fn, fr, rate = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
d = np.fromfile(fn, dtype='<f4')
nb = d.size // (2 * fr)
d = d[:nb * 2 * fr].reshape(nb, 2, fr)
print('blocks', nb, 'finite', bool(np.isfinite(d).all()), 'min %.5f max %.5f' % (d.min(), d.max()))
for c, nm in ((0, 'L'), (1, 'R')):
    x = d[:, c, :].reshape(-1)
    tail = d[-4:, c, :].reshape(-1)
    w = np.hanning(tail.size)
    sp = np.abs(np.fft.rfft(tail * w))
    k = int(np.argmax(sp[1:])) + 1
    print('%s: rms %.5f, last-4-blocks strongest bin %.1f Hz (bin width %.2f Hz)' % (nm, np.sqrt((x ** 2).mean()), k * rate / tail.size, rate / tail.size))
env = np.sqrt((d[:, 0, :] ** 2).mean(axis=1))
print('left RMS per block:', ' '.join('%.4f' % e for e in env[:12]), '...', ' '.join('%.4f' % e for e in env[-3:]))
