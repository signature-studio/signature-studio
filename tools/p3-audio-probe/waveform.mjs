// PCM repeat check (#1422): a copy of pm-work/findings/perf-windows-browser/tools/real-browser-perf/waveform.mjs, with the expressions that arm and take the capture in the page and the report the probes print.
// The check: among the non-silent comparisons, the share of frames whose left and right samples equal the ones PCM_LAG frames earlier. The neighbouring lags are the controls. A loop repeats exactly, so the share is not a verdict on its own: the bar (0.01) is the caller's.
// The summary is computed in the page; the samples never leave it.

/** Returns aggregate statistics only; samples stay in the browser. It is injected into the page by its source text, so it uses nothing from this module. */
export function waveformSummary(capture) {
  if (!capture) return null;
  const f = capture.frames;
  let nonFiniteSamples = 0;
  for (const value of f) if (!Number.isFinite(value)) nonFiniteSamples++;
  const comparisons = [4095, 4096, 4097, 8191, 8192, 8193, 12288, 16384, 32768].map(lag => {
    let eligible = 0, equal = 0;
    for (let i = lag * 2; i + 1 < f.length; i += 2) {
      const l = f[i], r = f[i + 1], p = f[i - lag * 2], q = f[i + 1 - lag * 2];
      if (!Number.isFinite(l) || !Number.isFinite(r) || !Number.isFinite(p) || !Number.isFinite(q) || Math.max(Math.abs(l), Math.abs(r), Math.abs(p), Math.abs(q)) < 1e-5) continue;
      eligible++;
      if (l === p && r === q) equal++;
    }
    return { lag, eligible, equalFraction: eligible ? equal / eligible : null };
  });
  return { valid: capture.valid, frames: f.length / 2, firstFrame: capture.firstFrame, validFrames: capture.validFrames, completedAtMs: capture.page ?? null, nonFiniteSamples, comparisons };
}

export const PCM_LAG = 8192;
/** fewer non-silent comparisons than this: the repeat share is not measured (null) */
export const PCM_MIN_ELIGIBLE = 1000;
/** the output the check reads: the first CAPTURE_S seconds of sound after the capture is armed */
export const CAPTURE_S = 8;

/** the page's expression that arms the capture (PageAudio.armCapture, timeline kept: the frames the engine had not delivered are copied as silence). Its value is the index the capture will have in audio.captures. */
export function armExpression() {
  return `(() => { const a = window.__app.audio; if (!a) return null; const i = a.captures.length; a.armCapture(Math.floor(a.opts.sampleRate * ${CAPTURE_S}), true); return i; })()`;
}

/** the page's expression that takes the capture at `index`: its summary is computed in the page and the capture is removed there. Its value is null while the capture has not arrived. */
export function takeExpression(index) {
  return `(() => { const a = window.__app.audio; if (!a || a.captures.length <= ${index}) return null; const c = a.captures.splice(${index}, 1)[0]; return (${waveformSummary.toString()})(c); })()`;
}

/** the numbers the probes print from a summary (null: the capture was not taken) */
export function pcmReport(summary) {
  if (!summary) return { pcmRepeat8192: null, pcmEligible: null, pcmNeighbors: null, pcmCaptureFrames: null, pcmCaptureValid: null, pcmNonFiniteSamples: null };
  const at = summary.comparisons.find((c) => c.lag === PCM_LAG);
  return {
    pcmRepeat8192: at.eligible < PCM_MIN_ELIGIBLE ? null : at.equalFraction,
    pcmEligible: at.eligible,
    pcmNeighbors: summary.comparisons.filter((c) => c.lag !== PCM_LAG).map((c) => ({ lag: c.lag, eligible: c.eligible, equalFraction: c.equalFraction })),
    pcmCaptureFrames: summary.frames,
    pcmCaptureValid: summary.valid,
    pcmNonFiniteSamples: summary.nonFiniteSamples,
  };
}
