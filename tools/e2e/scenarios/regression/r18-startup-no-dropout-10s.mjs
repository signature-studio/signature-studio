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

// the record of the window for the report (Issue #1432): the page's output history and the engine's reports and pulses from the first idle, times in ms from the first idle
const recordOf = (guest) => guest.ev(`(() => {
  const a = window.__app.audio;
  const f = a.marks['first-idle'];
  const t0 = f ? f.page : 0;
  const rel = (p) => Math.round(p - t0);
  return {
    t0: Math.round(t0),
    hist: a.history.filter((h) => h.page >= t0 - 200).map((h) => [rel(h.page), h.ur]),
    reports: a.reports.filter((r) => r.page >= t0 - 2000).map((r) => ({ at: rel(r.page), pubs: r.publishes, gap: r.gapFrames, skip: r.skipped, stretchMax: r.pace ? r.pace.stretchMaxMs : null, over50: r.pace ? r.pace.stretchOver50 : null, over78: r.pace ? r.pace.stretchOver78 : null, lateMax: r.pace ? r.pace.lateMaxMs : null, waits: r.waits })),
    pulses: a.pulses.filter((p) => p.page >= t0 - 2000).map((p) => ({ at: rel(p.page), mixer: p.mixer, threads: p.threads.map((t) => [Math.round(t[0]), Math.round(t[1]), t[2]]) })),
    unlocks: a.workerUnlocks.filter((u) => u.t - performance.timeOrigin >= t0 - 1000).map((u) => [Math.round(u.t - performance.timeOrigin - t0), u.n]),
  };
})()`);

// the underrun spans of the record: the step (ms from the first idle), the quanta, the engine's last report and pulse before the step
function spansOf(rec) {
  const out = [];
  for (let i = 1; i < rec.hist.length; i++) {
    const [at, ur] = rec.hist[i], prev = rec.hist[i - 1][1];
    if (ur <= prev) continue;
    const rep = [...rec.reports].reverse().find((r) => r.at <= at) ?? null;
    const pul = [...rec.pulses].reverse().find((p) => p.at <= at) ?? null;
    const unl = [...rec.unlocks].reverse().find((u) => u[0] <= at) ?? null;
    out.push({ at, dq: ur - prev, report: rep, pulseAt: pul ? pul.at : null, mixerThread: pul ? pul.mixer : null, lastUnlockAt: unl ? unl[0] : null, sinceLastUnlock: unl ? at - unl[0] : null });
  }
  return out;
}

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
    const rec = await recordOf(guest);
    const spans = spansOf(rec);
    const notes = [`first idle, then ${WINDOW_MS / 1000} s with no input: underrun quanta ${since}; the output: status ${a1.status}, context ${a1.ctx}, error ${a1.err ?? 'none'}; the count reached at: ${steps.length ? steps.join(', ') : 'none'}`];
    notes.push(`the record (ms from the first idle at ${rec.t0} ms of the page): spans ${JSON.stringify(spans.map((s) => ({ at: s.at, dq: s.dq, stretchMax: s.report?.stretchMax ?? null, over50: s.report?.over50 ?? null, over78: s.report?.over78 ?? null, lateMax: s.report?.lateMax ?? null, pubs: s.report?.pubs ?? null, gap: s.report?.gap ?? null, pulseAt: s.pulseAt, mixerThread: s.mixerThread, sinceLastUnlock: s.sinceLastUnlock })))}`);
    log(notes[0]);
    log(notes[1]);
    return result(since === 0 && running, (await guest.state())?.state ?? null, notes, { underrunQuanta: since, status: a1.status, spans, record: rec });
  },
};
