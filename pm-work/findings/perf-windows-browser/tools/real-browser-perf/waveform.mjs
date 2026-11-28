/** Returns aggregate statistics only; samples stay in the browser. */
export function waveformSummary(capture) {
  if (!capture) return null;
  const f = capture.frames;
  let nonFiniteSamples = 0;
  for (const value of f) if (!Number.isFinite(value)) nonFiniteSamples++;
  const comparisons = [4095, 4096, 4097, 8191, 8192, 8193, 12288, 16384, 32768].map(lag => {
    let eligible = 0, equal = 0;
    for (let i = lag * 2; i + 1 < f.length; i += 2) {
      const l = f[i], r = f[i + 1], p = f[i - lag * 2], q = f[i + 1 - lag * 2];
      if (!Number.isFinite(l) || !Number.isFinite(r) || !Number.isFinite(p) || !Number.isFinite(q) || Math.max(Math.abs(l), Math.abs(r), Math.abs(p), Math.abs(q)) < 1e-5) continue;
      eligible++;
      if (l === p && r === q) equal++;
    }
    return { lag, eligible, equalFraction: eligible ? equal / eligible : null };
  });
  return { valid: capture.valid, frames: f.length / 2, firstFrame: capture.firstFrame, validFrames: capture.validFrames, completedAtMs: capture.page ?? null, nonFiniteSamples, comparisons };
}
