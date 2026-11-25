import { test } from 'node:test';
import assert from 'node:assert/strict';
import { waveformSummary, pcmReport, PCM_MIN_ELIGIBLE } from '../tools/p3-audio-probe/waveform.mjs';

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
test('the report: the repeat share of a repeated buffer is 1, silence gives null', () => {
  const f = new Float32Array(32768 * 2);
  for (let i = 0; i < f.length / 2; i++) { f[2 * i] = ((i % 8192) + 1) / 8192; f[2 * i + 1] = -f[2 * i]; }
  const r = pcmReport(waveformSummary(capture(f)));
  assert.equal(r.pcmRepeat8192, 1);
  assert.equal(r.pcmNeighbors.find(x => x.lag === 4096).equalFraction, 0);
  assert.equal(r.pcmNeighbors.some(x => x.lag === 8192), false);
  const silent = pcmReport(waveformSummary(capture(new Float32Array(20000 * 2))));
  assert.equal(silent.pcmRepeat8192, null);
  assert.equal(silent.pcmEligible, 0);
  assert.equal(silent.pcmNonFiniteSamples, 0);
});
test('fewer than PCM_MIN_ELIGIBLE non-silent comparisons: the repeat share is null, the count is kept', () => {
  const f = new Float32Array((8192 + 500) * 2);
  for (let i = 0; i < f.length / 2; i++) { f[2 * i] = ((i % 8192) + 1) / 8192; f[2 * i + 1] = -f[2 * i]; }
  const r = pcmReport(waveformSummary(capture(f)));
  assert.equal(r.pcmEligible, 500);
  assert.ok(r.pcmEligible < PCM_MIN_ELIGIBLE);
  assert.equal(r.pcmRepeat8192, null);
});
test('no capture: every number is null', () => {
  const r = pcmReport(null);
  assert.deepEqual(Object.values(r), [null, null, null, null, null, null]);
});
