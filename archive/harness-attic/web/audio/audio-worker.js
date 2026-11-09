'use strict';
// The engine Worker of the audio prototype (decision 0139, PROVISIONAL): hands the ring (SharedArrayBuffer) to the engine's JS library (fle_audio_lib.js reads globalThis.fleAudioSab) and then is the Worker of decision 0134
// (../final0129/fle-final-worker.js, imported unchanged: it fetches the seed pack and the DLLs, runs main(), draws the surfaces). Messages added: {cmd:'audio-init', sab, cap, dataOffset} (before 'run'), {cmd:'audio-state'}
// -> {type:'audio-state', ...} (answered whenever the Worker's event loop runs: it does not while the engine computes).
self.addEventListener('message', (ev) => {
  const m = ev.data;
  if (!m) return;
  if (m.cmd === 'audio-init') self.fleAudioSab = { sab: m.sab, cap: m.cap, dataOffset: m.dataOffset, leadMs: m.leadMs || 0 };
  if (m.cmd === 'audio-state') {
    const w = self.fleAudioWorkerState ? self.fleAudioWorkerState() : null;
    let pace = null;
    try { if (self.fleAudioPace) pace = self.fleAudioPace(); } catch (e) { pace = { error: String(e) }; }
    let wf1 = null;
    try { if (self.wf1State) wf1 = self.wf1State(); } catch (e) { wf1 = { error: String(e) }; }
    self.postMessage({ type: 'audio-state', seq: m.seq, state: w, pace, wf1, tWorker: performance.now(), epoch: performance.timeOrigin + performance.now() });
  }
});
importScripts('/final/fle-final-worker.js');
