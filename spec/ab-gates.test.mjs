import { test } from 'node:test';
import assert from 'node:assert/strict';
import { median, mediansOf, gatesOf } from '../tools/ab/gates.mjs';

const p3 = (underrunDelta, extra = {}) => ({ error: null, playing: true, underrunDelta, last60sUnderruns: 0, inputRateOk: true, nonSilentDelta: 100, pcmRepeat8192: 0, ...extra });
const p2 = (minWindowFps, inputDropped) => ({ error: null, inputRateOk: true, minWindowFps, inputToPresentMsP95: 70, inputDropped });
const gateNamed = (gates, part) => gates.find((g) => g.name.includes(part));

test('median: the middle value, the mean of the two middle values, null for a missing value or no values', () => {
  assert.equal(median([3, 1, 2]), 2);
  assert.equal(median([4, 1, 3, 2]), 2.5);
  assert.equal(median([1, null, 2]), null);
  assert.equal(median([]), null);
});

test('medians keep the base and changed medians and the base maximum, and give each key its base and after medians', () => {
  const m = mediansOf('p3', [p3(1), p3(0), p3(1)], [p3(0), p3(0), p3(0)]);
  assert.equal(m.base.underrunDelta, 1);
  assert.equal(m.changed.underrunDelta, 0);
  assert.equal(m.baseMax.underrunDelta, 1);
  assert.deepEqual(m.underrunDelta, { base: 1, after: 0 });
  const q = mediansOf('p2', [p2(14.5, 58)], [p2(15.9, 78)]);
  assert.deepEqual(q.inputDropped, { base: 58, after: 78 });
  assert.equal(q.changed.minWindowFps, 15.9);
});

test('p3: underrunDelta passes when the changed median is at most the base median, even when the base is not 0', () => {
  const runs = { base: [p3(1), p3(0), p3(1)], changed: [p3(0), p3(0), p3(0)] };
  const gates = gatesOf('p3', runs, mediansOf('p3', runs.base, runs.changed));
  assert.equal(gateNamed(gates, 'underrunDelta').pass, true);
  assert.equal(gates.every((g) => g.pass), true);
});

test('p3: a null underrunDelta in the base cannot be compared, so the gate fails', () => {
  const runs = { base: [p3(null), p3(0), p3(0)], changed: [p3(0), p3(0), p3(0)] };
  const gates = gatesOf('p3', runs, mediansOf('p3', runs.base, runs.changed));
  assert.equal(gateNamed(gates, 'underrunDelta').pass, false);
});

// the noise band (#1426): the same build gives 4 to 14 underruns across its runs on a loaded runner, so the changed median may exceed the base median by chance; the gate is the changed median at most the base's largest run + 4
const byKey = {
  underrunDelta: (v) => p3(v),
  last60sUnderruns: (v) => p3(0, { last60sUnderruns: v }),
};
const noiseBand = [
  ['(a) base 7, 4, 8 and changed 5, 7, 7 pass', [7, 4, 8], [5, 7, 7], true],
  ['(b) base 8, 4, 4 and changed 4, 4, 4 pass', [8, 4, 4], [4, 4, 4], true],
  ['(c) base 0, 0, 0 and changed 3, 5, 2 pass', [0, 0, 0], [3, 5, 2], true],
  ['(d) base 0, 0, 0 and changed 9, 10, 8 fail', [0, 0, 0], [9, 10, 8], false],
  ['(e) base 4, 4, 4 and changed 30, 28, 31 fail', [4, 4, 4], [30, 28, 31], false],
];
for (const [key, make] of Object.entries(byKey)) {
  for (const [name, base, changed, pass] of noiseBand) {
    test(`p3: ${key} noise band ${name}`, () => {
      const runs = { base: base.map(make), changed: changed.map(make) };
      assert.equal(gateNamed(gatesOf('p3', runs, mediansOf('p3', runs.base, runs.changed)), key).pass, pass);
    });
  }
  test(`p3: ${key} with an empty base cannot be compared, so the gate fails`, () => {
    const runs = { base: [], changed: [make(0), make(0), make(0)] };
    assert.equal(gateNamed(gatesOf('p3', runs, mediansOf('p3', runs.base, runs.changed)), key).pass, false);
  });
}

test('p3: a null underrunDelta in a changed run cannot be compared, so the gate fails', () => {
  const runs = { base: [p3(0), p3(0), p3(0)], changed: [p3(null), p3(0), p3(0)] };
  assert.equal(gateNamed(gatesOf('p3', runs, mediansOf('p3', runs.base, runs.changed)), 'underrunDelta').pass, false);
});

const last60s = (values) => values.map((v) => p3(0, { last60sUnderruns: v }));
const last60sGate = (base, changed) => gateNamed(gatesOf('p3', { base, changed }, mediansOf('p3', base, changed)), 'last60sUnderruns');

test('p3: last60sUnderruns passes when the changed median is at most the base median, even when the base is not 0', () => {
  const base = last60s([14, 8, 4]);
  const changed = last60s([4, 11, 4]);
  assert.deepEqual(mediansOf('p3', base, changed).last60sUnderruns, { base: 8, after: 4 });
  assert.equal(last60sGate(base, changed).pass, true);
});

test('p3: last60sUnderruns is a median: one changed run of 14 passes when the changed median 7 is within the base max 14 + 4', () => {
  assert.equal(last60sGate(last60s([14, 8, 4]), last60s([7, 7, 14])).pass, true);
});

test('p3: a null last60sUnderruns in a run cannot be compared, so the gate fails', () => {
  assert.equal(last60sGate(last60s([0, 0, 0]), last60s([null, 0, 0])).pass, false);
});

test('p2: the minWindowFps and inputToPresentMsP95 gates are the medians', () => {
  const runs = { base: [p2(14.5, 58), p2(14.4, 60), p2(14.6, 57)], changed: [p2(15.9, 78), p2(15.5, 70), p2(16, 79)] };
  const gates = gatesOf('p2', runs, mediansOf('p2', runs.base, runs.changed));
  assert.equal(gateNamed(gates, 'minWindowFps').pass, true);
  assert.equal(gateNamed(gates, 'inputToPresentMsP95').pass, true);
});
