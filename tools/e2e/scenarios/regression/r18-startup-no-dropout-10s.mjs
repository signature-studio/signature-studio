// R18 (#1408) startup-no-dropout-10s. Report (L-054, #1375): after the engine's first idle, with no press and no input, the sound is not cut for 10 s. Expectation:
// the underrun quanta of the page's output in the 10 s after the first idle are 0, and the output is running (an output in error counts nothing and is not a pass).
// The page is only read; nothing is sent to it.
import { audioNow, sleep, result } from './common.mjs';
import { waitStable } from '../../lib/flow.mjs';

const SCENARIO = {
  name: 'startup-no-dropout-10s',
  report: 'R18',
  expect: 'from the first engine-idle, 10 s with no input: the audio output is running and 0 underrun quanta',
  // no mark: the 10 s after the first idle have 0 underrun quanta (#1432 stays open for the record; it is not a mark of this scenario)
  expectFail: null,
};
const WINDOW_MS = 10000;

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return result(false, first.state, [`first state ${first.state}`]);
    const a0 = await audioNow(guest);
    if (!a0) return result(false, first.state, ['the page has no audio output'], {});
    const sinceOf = (a) => (a.since !== null && a0.since !== null ? a.since - a0.since : a.ur - a0.ur);
    const t0 = Date.now();
    // the times of the underrun steps (a record for the report only: the judgement is the count below)
    const steps = [];
    let seen = 0;
    while (Date.now() - t0 < WINDOW_MS) {
      await sleep(250);
      const a = await audioNow(guest);
      if (a && sinceOf(a) !== seen) { steps.push(`${((Date.now() - t0) / 1000).toFixed(2)} s: ${sinceOf(a)}`); seen = sinceOf(a); }
    }
    const a1 = await audioNow(guest);
    const since = sinceOf(a1);
    const running = a1.status === 'running';
    const notes = [`first idle, then ${WINDOW_MS / 1000} s with no input: underrun quanta ${since}; the output: status ${a1.status}, context ${a1.ctx}, error ${a1.err ?? 'none'}; the count reached at: ${steps.length ? steps.join(', ') : 'none'}`];
    log(notes[0]);
    return result(since === 0 && running, (await guest.state())?.state ?? null, notes, { underrunQuanta: since, status: a1.status });
  },
};
