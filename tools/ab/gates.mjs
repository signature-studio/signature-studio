// tools/ab/gates.mjs (#1422): the medians and the gates of one A/B comparison (tools/ab/run.mjs). Pure: the runs in, the medians and the gates out.
// medians: base and changed (each key's median over a side's runs), and for each key { base, after } (the same values, by key).
// p3: underrunDelta and last60sUnderruns are median gates (the changed median at most the base median), not 0: the base run has its own underruns on a loaded runner.
export const MEDIAN_KEYS = { p2: ['minWindowFps', 'inputToPresentMsP95', 'inputDropped'], p3: ['pcmRepeat8192', 'underrunDelta', 'last60sUnderruns', 'nonSilentDelta'] };

// the median of the values; null when one of them is not a finite number (a run without the value cannot be compared)
export function median(values) {
  if (values.some((x) => typeof x !== 'number' || !Number.isFinite(x))) return null;
  const v = [...values].sort((a, b) => a - b);
  const m = v.length >> 1;
  return v.length % 2 === 1 ? v[m] : (v[m - 1] + v[m]) / 2;
}

export function mediansOf(kind, base, changed) {
  const of = (runs) => Object.fromEntries(MEDIAN_KEYS[kind].map((k) => [k, median(runs.map((r) => r[k]))]));
  const b = of(base);
  const c = of(changed);
  return { base: b, changed: c, ...Object.fromEntries(MEDIAN_KEYS[kind].map((k) => [k, { base: b[k], after: c[k] }])) };
}

export function gatesOf(kind, runs, medians) {
  const gates = [];
  const gate = (name, pass, values) => gates.push({ name, pass: pass === true, values });
  if (kind === 'p2') {
    const all = [...runs.base, ...runs.changed];
    gate('every run: error is null', all.every((r) => r.error === null), all.map((r) => r.error));
    gate('every run: inputRateOk is true', all.every((r) => r.inputRateOk === true), all.map((r) => r.inputRateOk));
    const mw = medians.changed.minWindowFps, bw = medians.base.minWindowFps;
    gate('minWindowFps: changed median >= 0.90 x base median', mw !== null && bw !== null && mw >= bw * 0.9, { base: bw, changed: mw });
    const mp = medians.changed.inputToPresentMsP95, bp = medians.base.inputToPresentMsP95;
    gate('inputToPresentMsP95: changed median <= 1.20 x base median', mp !== null && bp !== null && mp <= bp * 1.2, { base: bp, changed: mp });
  } else {
    const ch = runs.changed;
    // each gate: its test on every changed run, and the value of each run that it shows
    const each = (name, key, ok) => gate(`every changed run: ${name}`, ch.length > 0 && ch.every(ok), ch.map((r) => r[key]));
    each('error is null', 'error', (r) => r.error === null);
    each('playing is true', 'playing', (r) => r.playing === true);
    each('inputRateOk is true', 'inputRateOk', (r) => r.inputRateOk === true);
    each('nonSilentDelta is above 0', 'nonSilentDelta', (r) => typeof r.nonSilentDelta === 'number' && r.nonSilentDelta > 0);
    each('pcmRepeat8192 is null or below 0.01', 'pcmRepeat8192', (r) => r.pcmRepeat8192 === null || (typeof r.pcmRepeat8192 === 'number' && r.pcmRepeat8192 < 0.01));
    const ub = medians.base.underrunDelta, uc = medians.changed.underrunDelta;
    gate('underrunDelta: changed median <= base median', ub !== null && uc !== null && uc <= ub, { base: ub, changed: uc });
    const lb = medians.base.last60sUnderruns, lc = medians.changed.last60sUnderruns;
    gate('last60sUnderruns: changed median <= base median', lb !== null && lc !== null && lc <= lb, { base: lb, changed: lc });
  }
  return gates;
}
