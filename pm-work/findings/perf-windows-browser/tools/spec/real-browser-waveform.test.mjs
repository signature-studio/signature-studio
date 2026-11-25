import { test } from 'node:test';
import assert from 'node:assert/strict';
import { waveformSummary } from '../tools/real-browser-perf/waveform.mjs';

const capture = frames => ({ frames, valid: true, validFrames: frames.length / 2, firstFrame: 123 });
test('detects an exact repeated buffer against neighbouring lag controls', () => {
  const f = new Float32Array(32768 * 2);
  for (let i = 0; i < f.length / 2; i++) { f[2 * i] = ((i % 8192) + 1) / 8192; f[2 * i + 1] = -f[2 * i]; }
  const s = waveformSummary(capture(f));
  assert.equal(s.comparisons.find(x => x.lag === 8192).equalFraction, 1);
  assert.equal(s.comparisons.find(x => x.lag === 8191).equalFraction, 0);
  assert.equal(s.comparisons.find(x => x.lag === 8193).equalFraction, 0);
});
test('silence is unmeasured and non-finite audio is reported', () => {
  const f = new Float32Array(20000 * 2);
  const s = waveformSummary(capture(f));
  assert.equal(s.comparisons.find(x => x.lag === 8192).equalFraction, null);
  f[20000] = NaN; f[20001] = Infinity;
  assert.equal(waveformSummary(capture(f)).nonFiniteSamples, 2);
  assert.equal(waveformSummary(null), null);
});
