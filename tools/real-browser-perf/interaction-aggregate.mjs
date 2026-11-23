export function aggregateInteraction({ records, startSeq, startAt, endAt, sentDelta, drags, retentionLimit }) {
  const percentile = (sorted, p) => sorted.length ? sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil(p / 100 * sorted.length) - 1))] : null;
  const selected = records.filter((r) => r.seq > startSeq && r.tPage >= startAt && r.tPage <= endAt);
  const times = [];
  let answered = 0, late = 0, noAnswer = 0, dropped = 0, pending = 0;
  for (const r of selected) {
    if (r.dropped !== null) { dropped++; continue; }
    if (r.outcome === 'window' || r.outcome === 'stream') answered++;
    else if (r.outcome === 'late') late++;
    else if (r.outcome === 'none') noAnswer++;
    else pending++;
    if (r.tEvent === null) continue;
    const answerRafs = Object.values(r.answers ?? {}).map((a) => a.raf).filter((raf) => Number.isFinite(raf) && raf >= 0);
    if (answerRafs.length) times.push(Math.min(...answerRafs) - r.tEvent);
  }
  times.sort((a, b) => a - b);
  const missingRecords = Math.max(0, sentDelta - selected.length);
  const warnings = [];
  if (missingRecords) warnings.push(`Latency records missing ${missingRecords} of ${sentDelta} inputs; LatencyTracker.records retains at most ${retentionLimit}. Status counts below cover retained records only.`);
  const completedDrags = drags.filter((d) => d.completed);
  if (drags.length !== completedDrags.length) warnings.push(`${drags.length - completedDrags.length} drag interval(s) ended without pointerup.`);
  const unavailableCanvasCounters = drags.filter((d) => !Number.isFinite(d.canvasFrames)).length;
  if (unavailableCanvasCounters) warnings.push(`Canvas frame counter unavailable or non-monotonic for ${unavailableCanvasCounters} drag interval(s); canvasFrames and rate are null.`);
  return {
    inputCount: sentDelta,
    recordsObserved: selected.length,
    eventToRaf: { n: times.length, p50Ms: percentile(times, 50), p95Ms: percentile(times, 95), maxMs: times.length ? times.at(-1) : null },
    answered, late, noAnswer, dropped, pending,
    drags: drags.map((d, index) => ({ index, canvasFrames: Number.isFinite(d.canvasFrames) ? d.canvasFrames : null, elapsedMs: d.elapsedMs, canvasFramesPerSecond: !Number.isFinite(d.canvasFrames) || !Number.isFinite(d.elapsedMs) || d.elapsedMs <= 0 ? null : d.canvasFrames * 1000 / d.elapsedMs, completed: d.completed })),
    warnings,
  };
}
