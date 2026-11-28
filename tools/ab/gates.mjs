// tools/ab/gates.mjs (#1422): the medians and the gates of one A/B comparison (tools/ab/run.mjs). Pure: the runs in, the medians and the gates out.
// medians: base and changed (each key's median over a side's runs), and for each key { base, after } (the same values, by key).
// p3: underrunDelta and last60sUnderruns are gated by the changed median at most the base's largest run + NOISE_BAND: not 0, and not the base median. On a loaded runner the same build gives 4 to 14 underruns across its runs (#1426), so a median of three against the base median fails by chance. The absolute 0 is the E2E's startup-no-dropout-10s.
export const MEDIAN_KEYS = { p2: ['minWindowFps', 'inputToPresentMsP95', 'inputDropped'], p3: ['pcmRepeat8192', 'underrunDelta', 'last60sUnderruns', 'nonSilentDelta'] };
export const NOISE_BAND = 4;

const finiteAll = (values) => values.length > 0 && values.every((x) => typeof x === 'number' && Number.isFinite(x));

// the median of the values; null when there are none, or when one of them is not a finite number (a run without the value cannot be compared)
export function median(values) {
  if (!finiteAll(values)) return null;
  const v = [...values].sort((a, b) => a - b);
  const m = v.length >> 1;
  return v.length % 2 === 1 ? v[m] : (v[m - 1] + v[m]) / 2;
}

// the largest of the values; null under the same conditions as median
export function maxOf(values) {
  return finiteAll(values) ? Math.max(...values) : null;
}

export function mediansOf(kind, base, changed) {
  const keyed = (runs, f) => Object.fromEntries(MEDIAN_KEYS[kind].map((k) => [k, f(runs.map((r) => r[k]))]));
  const b = keyed(base, median);
  const c = keyed(changed, median);
  return { base: b, changed: c, baseMax: keyed(base, maxOf), ...Object.fromEntries(MEDIAN_KEYS[kind].map((k) => [k, { base: b[k], after: c[k] }])) };
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
    // the changed median at most the base's largest run + NOISE_BAND (a null on either side fails)
    const noisy = (key) => {
      const bm = medians.baseMax[key], cm = medians.changed[key];
      gate(`${key}: changed median <= base max + ${NOISE_BAND}`, bm !== null && cm !== null && cm <= bm + NOISE_BAND, { base: medians.base[key], baseMax: bm, changed: cm });
    };
    noisy('underrunDelta');
    noisy('last60sUnderruns');
  }
  return gates;
}
