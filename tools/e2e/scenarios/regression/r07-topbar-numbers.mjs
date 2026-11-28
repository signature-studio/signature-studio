// R07 (#1408) topbar-numbers. Report (L-060, #1391, #1402): the top bar's Memory Usage, CPU Load and Frame Rate show the real values (not 0 MB, not a fixed 120).
// The digits on the bar are not read here (the text near a unit is not tied to its value: #1402). The judgement follows tools/topbar-metrics: the engine's own numbers.
// Expectation: Memory: the engine's last answer to K32GetProcessMemoryInfo (its log) has WorkingSetSize > 0. CPU: the engine's busy time (the idles' busyMs) over 5 s is > 0.
// Frame Rate: over 5 s, the rate the engine is given (window.__app.hz.hz) and the DwmFlush rate (dwmPerSec) each follow the page's rAF (rafPerSec) within 15%.
// The window starts 10 s after the start (the page measures its rAF every 2 s, and the engine takes a changed rate at its next idle). The page's rAF counter is
// the one run.mjs installed (readRaf); it is not installed again here (a second loop would count every frame twice).
import { readRaf } from '../../lib/browser.mjs';
import { sleep } from '../../lib/flow.mjs';
import { engineLines, startDefault, result } from './common.mjs';

const SCENARIO = {
  name: 'topbar-numbers',
  report: 'R07',
  expect: 'top bar: Memory (engine K32 WorkingSetSize > 0), CPU (engine busy time > 0 over 5 s), Frame Rate (envHz and dwmPerSec each within 15% of the page rAF over 5 s)',
  expectFail: null,
};
const SETTLE_MS = 10000;
const WINDOW_MS = 5000;
const TRACK = 0.15;

// the engine's numbers now: the absolute time, the rate it is given, and its idles as [at, busyMs, now, dwm] (tools/topbar-metrics reads them the same way)
const SAMPLE = `(() => { const a = window.__app; return { abs: performance.timeOrigin + performance.now(), hz: a.hz ? a.hz.hz : null, idles: a.idles.map((x) => [x.at, x.busyMs, x.now, x.dwm]) }; })()`;
const lastDwm = (idles) => idles.reduce((v, x) => (typeof x[3] === 'number' ? x[3] : v), null);

export default {
  ...SCENARIO,
  async run({ guest, page, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await sleep(SETTLE_MS);
    const k32 = (await engineLines(guest)).filter((l) => /memory-usage: K32GetProcessMemoryInfo/.test(l));
    const workingSet = k32.length ? Number(/WorkingSetSize (\d+)/.exec(k32.at(-1))?.[1] ?? 0) : 0;
    const r0 = await readRaf(page);
    const s0 = await guest.ev(SAMPLE);
    await sleep(WINDOW_MS);
    const r1 = await readRaf(page);
    const s1 = await guest.ev(SAMPLE);
    const secs = (r1.t - r0.t) / 1000;
    const rafPerSec = (r1.n - r0.n) / secs;
    const dwm0 = lastDwm(s0.idles), dwm1 = lastDwm(s1.idles);
    const dwmPerSec = dwm0 === null || dwm1 === null ? null : (dwm1 - dwm0) / secs;
    const busyMs = s1.idles.filter(([at]) => at > s0.abs && at <= s1.abs).reduce((t, [, b]) => t + (typeof b === 'number' ? b : 0), 0);
    const memOk = workingSet > 0;
    const cpuOk = busyMs > 0;
    const within = (v) => v !== null && v !== undefined && Math.abs(v - rafPerSec) <= TRACK * rafPerSec;
    const hzOk = within(s1.hz);
    const dwmOk = within(dwmPerSec);
    const notes = [
      ...start.notes,
      `Memory: K32 WorkingSetSize ${workingSet} bytes (${k32.length} answers; > 0: ${memOk})`,
      `CPU: busy ${busyMs.toFixed(1)} ms over ${((s1.abs - s0.abs) / 1000).toFixed(1)} s (> 0: ${cpuOk})`,
      `Frame Rate: page rAF ${rafPerSec.toFixed(1)}/s; envHz ${s1.hz} (within 15%: ${hzOk}); dwmPerSec ${dwmPerSec === null ? 'null' : dwmPerSec.toFixed(1)} (within 15%: ${dwmOk})`,
    ];
    log(notes.slice(-3).join(' | '));
    return result(memOk && cpuOk && hzOk && dwmOk, (await guest.state())?.state ?? null, notes, { workingSet, busyMs: Math.round(busyMs), rafPerSec: Number(rafPerSec.toFixed(2)), envHz: s1.hz, dwmPerSec: dwmPerSec === null ? null : Number(dwmPerSec.toFixed(2)) });
  },
};
