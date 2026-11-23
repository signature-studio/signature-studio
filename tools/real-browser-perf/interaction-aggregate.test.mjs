import test from 'node:test';
import assert from 'node:assert/strict';
import { aggregateInteraction } from './interaction-aggregate.mjs';

test('aggregates only the requested input range and reports latency retention loss', () => {
  const result = aggregateInteraction({
    startSeq: 10, startAt: 100, endAt: 200, sentDelta: 5, retentionLimit: 2,
    records: [
      { seq: 10, tPage: 110, tEvent: 110, outcome: 'window', dropped: null, answers: { 1: { raf: 120 } } },
      { seq: 11, tPage: 120, tEvent: 120, outcome: 'stream', dropped: null, answers: { 1: { raf: 140 } } },
      { seq: 12, tPage: 130, tEvent: 130, outcome: 'late', dropped: null, answers: {} },
      { seq: 13, tPage: 140, tEvent: 140, outcome: null, dropped: null, answers: {} },
      { seq: 14, tPage: 150, tEvent: 150, outcome: null, dropped: 'queue full', answers: {} },
      { seq: 15, tPage: 201, tEvent: 201, outcome: 'window', dropped: null, answers: { 1: { raf: 202 } } },
    ],
    drags: [{ canvasFrames: 7, elapsedMs: 31, completed: true }],
  });
  assert.equal(result.inputCount, 5);
  assert.equal(result.recordsObserved, 4);
  assert.deepEqual(result.eventToRaf, { n: 1, p50Ms: 20, p95Ms: 20, maxMs: 20 });
  assert.deepEqual([result.answered, result.late, result.noAnswer, result.dropped, result.pending], [1, 1, 0, 1, 1]);
  assert.deepEqual(result.warnings, ['Latency records missing 1 of 5 inputs; LatencyTracker.records retains at most 2. Status counts below cover retained records only.']);
  assert.deepEqual(result.drags[0], { index: 0, canvasFrames: 7, elapsedMs: 31, canvasFramesPerSecond: 7 * 1000 / 31, completed: true });
});

test('leaves an unavailable canvas counter null instead of reporting zero', () => {
  const result = aggregateInteraction({ startSeq: 0, startAt: 1, endAt: 5, sentDelta: 0, retentionLimit: 400, records: [],
    drags: [{ canvasFrames: null, elapsedMs: 20, completed: true }] });
  assert.equal(result.drags[0].canvasFrames, null);
  assert.equal(result.drags[0].canvasFramesPerSecond, null);
  assert.match(result.warnings[0], /Canvas frame counter unavailable/);
});

test('counts dropped and unanswered records separately', () => {
  const result = aggregateInteraction({ startSeq: 0, startAt: 1, endAt: 5, sentDelta: 3, retentionLimit: 400,
    records: [
      { seq: 1, tPage: 2, tEvent: null, outcome: 'none', dropped: null, answers: {} },
      { seq: 2, tPage: 3, tEvent: null, outcome: null, dropped: 'full', answers: {} },
      { seq: 3, tPage: 4, tEvent: null, outcome: null, dropped: null, answers: {} },
    ], drags: [] });
  assert.deepEqual([result.answered, result.late, result.noAnswer, result.dropped, result.pending], [0, 0, 1, 1, 1]);
  assert.deepEqual(result.eventToRaf, { n: 0, p50Ms: null, p95Ms: null, maxMs: null });
  assert.deepEqual(result.warnings, []);
});
