'use strict';
// Worker of the scenario case (decision 0139, PROVISIONAL): runs harness/audio/ds_scenarios.c built for the browser (build-scen-wasm.sh) instead of the engine: the REAL runtime (fle_dsound.c, fle_audio_ring.c, the cooperative threads
// switched by JSPI) with a C "mixer" in the place of FLEngine's, so that the path DirectSound model -> ring -> SharedArrayBuffer -> AudioWorklet is measured with a producer whose speed is known.
// Messages: {cmd:'audio-init', sab, cap, dataOffset} first; {cmd:'run-scen', dist, args:[...]}; {cmd:'audio-state', seq}. Answers: {type:'stdout'|'stderr', line}, {type:'done', exitCode}, {type:'audio-state', ...}.
const post = (m) => self.postMessage(m);
self.addEventListener('message', (ev) => {
  const m = ev.data;
  if (!m) return;
  if (m.cmd === 'audio-init') self.fleAudioSab = { sab: m.sab, cap: m.cap, dataOffset: m.dataOffset, leadMs: m.leadMs || 0 };
  if (m.cmd === 'audio-state') {
    const w = self.fleAudioWorkerState ? self.fleAudioWorkerState() : null;
    let pace = null;
    try { if (self.fleAudioPace) pace = self.fleAudioPace(); } catch (e) { pace = { error: String(e) }; }
    post({ type: 'audio-state', seq: m.seq, state: w, pace, tWorker: performance.now(), epoch: performance.timeOrigin + performance.now() });
  }
  if (m.cmd === 'init') post({ type: 'ready' });
  if (m.cmd === 'run-scen') run(m).catch((e) => post({ type: 'error', message: String((e && e.stack) || e) }));
});
async function run(m) {
  let code = 0;
  importScripts(m.dist + 'audio_scen.js');
  const M = await createScen({
    arguments: m.args,
    print: (s) => post({ type: 'stdout', line: s }),
    printErr: (s) => post({ type: 'stderr', line: s }),
    locateFile: (f) => m.dist + f,
    quit: (c) => { code = c; },
    fleThreadFailed: (idx, e, msg) => { post({ type: 'stderr', line: '[js] ' + msg }); code = 99; },
  });
  post({ type: 'done', exitCode: code });
}
