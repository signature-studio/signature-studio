import { describe, expect, it } from 'vitest';
import { matchStopNotes, summarizeEngineLog } from '../../src/engine/fruity-log.ts';

// decision 0197: the wasm harness' abort path (fle_pruned() and every other abort()) writes the stop lines WITHOUT the "fruity: " prefix; this is the log of a real Chrome run of a bundle whose prune list
// contained an executed function (lift/out-size2, 2026-11-04), shortened. The stop note of the manifest must still be found.
const ABORT = [
  'a4: entry_check started_but_not_entered=132 runnable_entered_beyond_init_count=0',
  '== stop: SIGABRT (runtime abort, see runtime message)',
  'runtime message: fle: pruned function 0x2001040 was executed (prune list, decision 0168; check the CPUID answer of fle_cpuid.c and calls of ippSetCpuFeatures)',
  'abort kind: (not a runtime message)',
  '[err] fle: pruned function 0x2001040 was executed (prune list, decision 0168; check the CPUID answer of fle_cpuid.c and calls of ippSetCpuFeatures)',
];

describe('summarizeEngineLog: the abort path without the "fruity: " prefix', () => {
  const s = summarizeEngineLog(ABORT);
  it('finds the stop line and the runtime message', () => {
    expect(s.stopLine).toBe('== stop: SIGABRT (runtime abort, see runtime message)');
    expect(s.runtimeMessage).toContain('pruned function 0x2001040 was executed');
  });
  it('the stop note of the manifest matches', () => {
    expect(matchStopNotes(s.runtimeMessage, [{ contains: 'pruned function', note: 'N' }])).toEqual(['N']);
  });
  it('the prefixed form still wins when both are present', () => {
    const t = summarizeEngineLog(['fruity: runtime message: A', 'runtime message: B']);
    expect(t.runtimeMessage).toBe('A');
  });
});
