// node summarize.mjs   prints the tables of docs/investigations/browser-event-loop.md from results/*.json (numbers are copied from this output)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const R = path.join(path.dirname(fileURLToPath(import.meta.url)), 'results');
const J = (n) => JSON.parse(fs.readFileSync(path.join(R, n + '.json'), 'utf8'));
const has = (n) => fs.existsSync(path.join(R, n + '.json'));
const f0 = (o) => ['p50', 'p95', 'p99', 'max'].map((k) => Math.round(o[k])).join(' / ');
const f1 = (o) => ['p50', 'p95', 'p99', 'max'].map((k) => o[k].toFixed(1)).join(' / ');
const f2 = (o) => ['p50', 'p95', 'p99', 'max'].map((k) => o[k].toFixed(2)).join(' / ');

console.log('## RTT (us) p50/p95/p99/max, 10,000 round trips; runs: ma-coi, ma-rtt-r1..r3');
for (const n of ['ma-coi', 'ma-rtt-r1', 'ma-rtt-r2', 'ma-rtt-r3']) {
  const r = J(n).result;
  console.log(`${n}: nonblocking-worker/pm ${f0(r.rtt_nonblocking_pm.rtt)} | A sab->sab ${f0(r.rtt_sab_sab.rtt)} | A sab->pm ${f0(r.rtt_sab_pm.rtt)} | A paced4ms (n=${r.rtt_sab_sab_paced4ms.rtt.n}) ${f0(r.rtt_sab_sab_paced4ms.rtt)} | worker<->worker ${f0(r.pingpong_worker_worker.rtt)}`);
}
{ const r = J('ma-plain').result; console.log(`ma-plain (no COOP/COEP): crossOriginIsolated=${r.env.crossOriginIsolated} SharedArrayBuffer=${r.env.SharedArrayBuffer} timeResolution=${r.env.timeResolution_us}us nonblocking/pm ${f0(r.rtt_nonblocking_pm.rtt)}`); }
for (const n of ['mb-coi', 'mb-rtt-r1', 'mb-rtt-r2', 'mb-plain']) console.log(`${n}: ` + J(n).result.rtt.map((x) => `${x.mode}/${x.opt}/depth${x.depth} ${f0(x.rtt)}`).join(' | '));

console.log('\n## Atomics.wait(timeout) lateness (ms) p50/p95/p99/max');
{ const r = J('ma-coi').result; for (const [k, v] of Object.entries(r.atomics_wait_timeout.rows)) console.log(`T=${k} ms reps=${v.reps}: ${f2(v)}`);
  console.log('timer (absolute deadlines):'); for (const [k, v] of Object.entries(r.timer)) console.log(`${k}: tickLate ${f2(v.tickLate)} events=${v.events} sent=${v.sent} eventLatency ${v.eventLatency.n ? f2(v.eventLatency) : '-'} dropped=${v.dropped}`); }

console.log('\n## busy worker (ma-coi): work ms/event, cap, coalesce -> latency p50/p95/p99/max ms, consumed/sent, dropped, downLost/downSent, maxDepth');
for (const b of J('ma-coi').result.busy) console.log(`work=${b.work_ms_per_event} cap=${b.cap} coalesce=${b.coalesce}: lat ${f1(b.latency)} consumed=${b.consumed}/${b.moveSent + b.downSent} dropped=${b.dropped} downLost=${b.downLost}/${b.downSent} maxDepth=${b.maxDepth} left=${b.leftAtStop}`);
console.log('\n## page busy (ma-coi mainbusy)');
for (const m of J('ma-coi').result.mainbusy) console.log(`page busy ${m.busy_ms} ms every ${m.every_ms} ms: worker tickLate ${f2(m.worker.tickLate)} | ticks at page ${m.arrivalsAtPage} gap p50/p99/max ${m.gapAtPage.p50.toFixed(1)}/${m.gapAtPage.p99.toFixed(1)}/${m.gapMax.toFixed(1)}`);
{ const t = J('ma-coi').result.throughput; console.log(`throughput: post() ${t.post_us_each.toFixed(3)} us each (incl. full-ring rejects); accepted ${t.accepted} in ${t.run_ms.toFixed(0)} ms = ${(t.accepted / t.run_ms * 1000 / 1e6).toFixed(2)} M/s, consumed ${t.consumed}`); }

console.log('\n## M-B bench (mb-bench-coi): ns per iteration; kind 0=plain import, 1=microtask suspend, 2=macrotask suspend');
{ const b = J('mb-bench-coi').result.bench;
  for (const [k, v] of Object.entries(b)) for (const w of [0, 1]) for (const kind of [0, 1, 2]) { const rows = v.rows.filter((x) => x.wide === w && x.kind === kind); if (rows.length) console.log(`${k} wide=${w} kind=${kind}: ` + rows.map((x) => `d${x.depth}:${Math.round(x.ns_per_iter)}`).join(' ')); }
  console.log('net suspend+resume (microtask - plain) ns, wide=0:');
  for (const k of Object.keys(b)) { if (k.startsWith('sync')) continue; const get = (kind, d) => b[k].rows.find((x) => x.wide === 0 && x.kind === kind && x.depth === d).ns_per_iter; console.log(k + ': ' + [0, 10, 40, 160].map((d) => `d${d}:${Math.round(get(1, d) - get(0, d))}`).join(' ')); } }
console.log('\n## M-B recursion depth (mb-depth)'); console.log(JSON.stringify(J('mb-depth').result.depth));

console.log('\n## wasm sizes (build.sh) are in out-web2/mb/sizes.tsv; synthetic Asyncify expansion:');
for (const f of fs.readdirSync(R).filter((x) => /^synth(-rt)?-F\d+-O\d\.tsv$/.test(x)).sort()) {
  const rows = fs.readFileSync(path.join(R, f), 'utf8').trim().split('\n').map((l) => l.split('\t'));
  const base = +rows.find((r) => r[0] === 'base')[1];
  console.log(f + ': ' + rows.map((r) => `${r[0]} ${r[1]} B (x${(r[1] / base).toFixed(3)}) link ${r[2]}s rss ${(r[3] / 1024).toFixed(0)}MB`).join(' | '));
}
console.log('\n## synthetic run time (ns per function entry, min of 6 / median)');
for (const n of ['synthrun-r1', 'synthrun-r2', 'synthrun-r3']) console.log(n + ': ' + J(n).result.rows.map((r) => `${r.opt}/${r.variant} ${r.ns_per_function_entry_min} (med ${(r.ms_median * 1e6 / 3e6).toFixed(0)})`).join(' | '));

console.log('\n## drawing (draw-coi)');
{ const r = J('draw-coi').result; const w = r.worker_offscreen; const m = r.main_canvas;
  console.log('env', JSON.stringify(r.env));
  console.log(`ImageData over linear memory: construct ${f2(w.imageDataConstruct_ms)} ms, shares memory=${w.imageDataSharesMemory}; from SAB view: ${w.imageDataFromSharedView}`);
  console.log(`copy 8.3 MB: shared->nonshared ${f2(w.copyShared8MB_ms)} ms; linear->nonshared ${f2(w.copyLinear8MB_ms)} ms`);
  for (const k of Object.keys(w)) if (w[k] && w[k].fps_total) console.log(`Worker OffscreenCanvas ${k}: ${w[k].fps_total} fps (loop rate), ms/frame ${f2(w[k].ms_per_frame)}`);
  console.log(`Worker putImageData+transferToImageBitmap ${f2(w.put_full_then_transferToImageBitmap.ms_per_frame)} ms; createImageBitmap(ImageData) ${f2(w.createImageBitmap_from_ImageData.ms_per_call_incl_await)} ms`);
  for (const k of Object.keys(m)) if (m[k] && m[k].fps_total) console.log(`Main canvas ${k}: ${m[k].fps_total} fps (loop rate), ms/call ${f2(m[k].ms_per_call)}`);
  console.log(`Main rAF+putImageData full: ${m.raf_with_put_full.fps} fps, interval ${f2(m.raf_with_put_full.interval_ms)}; createImageBitmap+bitmaprenderer ${f2(m.createImageBitmap_bitmaprenderer.ms_per_call_incl_await)} ms`);
  const p = r.postpixels; console.log(`Worker->page 8.3 MB copies: worker copy ${f2(p.worker.copy_ms)} ms, post->receive ${f2(p.post_to_receive_ms)} ms, page putImageData ${f2(p.main_putImageData_ms)} ms, ${p.fps_main_receive_and_draw} fps`);
  for (const [k, v] of Object.entries(r.present)) console.log(`present ${k}: worker frames ${v.workerInfo.frames}; displayed frame R-channel at 8 screenshots: ${v.shots.map((s) => s.rgba && s.rgba[0]).join(',')}; screencast ${v.screencast_fps} fps`);
}
{ const r = J('draw-gpuflag').result; console.log('draw-gpuflag renderer:', r.env.gl, 'put_full', r.worker_offscreen.put_full.fps_total, 'fps loop rate'); }

console.log('\n## audio (audio-coi)');
for (const x of J('audio-coi').result.runs) { const c = x.cfg; console.log(`sr=${c.sampleRate}->${x.created.sampleRate} ring=${c.cap}f chunk=${c.chunk} stall=${c.stallMs || 0}ms/${c.stallEveryMs || 0} load=${c.loadMs || 0}ms mainBusy=${c.mainBusy ? 1 : 0}: state=${x.created.state} rt=${x.timing.realtime_factor} blocks=${x.worklet.blocks}/${x.worklet.expectedBlocks} underrun start-up=${x.worklet.underrunBlocksTotal - x.worklet.underrunBlocksAfterStart} steady=${x.worklet.underrunBlocksAfterStart} fill ${x.worklet.minFill}..${x.worklet.maxFill}`); }
{ const r = J('audio-coi').result; console.log('audio env', JSON.stringify(r.env)); const a = r.runs[0]; console.log('baseLatency', a.created.baseLatency, 'outputLatency after', a.after.outputLatency); }

console.log('\n## input (input-coi)');
{ const j = J('input-coi'); const r = j.result; const s = r.summary;
  console.log('seen', JSON.stringify(s.seen)); console.log(`CDP move round trip ${r.cdp_move_roundtrip_ms} ms; handler-time minus event.timeStamp ${f1(s.queueDelay_ms)} ms; handler cost ${f1(s.handlerCost_us)} us; handler-post -> Worker dequeue ${f2(s.worker.latency)} ms`);
  console.log('coalesced events per pointermove', JSON.stringify(s.coalescedPerMove)); console.log('gesture', JSON.stringify(r.gesture)); console.log('apis', JSON.stringify(r.probe.apis)); console.log('cursor', JSON.stringify(r.probe.cursor));
  console.log('drop', JSON.stringify(s.drop)); console.log('ime', JSON.stringify(r.ime.map((e) => e.type + ':' + e.data).join(' ')));
  console.log('keys', JSON.stringify(s.keys.map((k) => `${k.t}:${k.code}/${k.key}/${k.keyCode}`).join(' '))); console.log('lastPointer', JSON.stringify(s.lastPointer)); }

console.log('\n## M-C\' (generator-made unwind/rewind) on synthetic code: runs');
for (const n of ['cpsrun-F2000-micro', 'cpsrun-F2000-micro-r2', 'cpsrun-F2000-micro-r3', 'cpsrun-F2000-task', 'cpsrun-F8000-O0']) { const j = J(n); console.log(n + ': ' + j.result.rows.map((r) => `${r.opt}/${r.variant} min ${r.ns_per_function_entry_min} ns/entry (median ${(r.ms_median * 1e6 / 3e6).toFixed(0)}) checksum ${r.checksums.join(',')} suspensions ${r.suspensions.join(',')}`).join(' | ')); }
console.log(fs.readFileSync(path.join(R, 'cps-build.log'), 'utf8').split('\n').filter((l) => /wasm/.test(l)).join('\n'));
console.log('\n## sensitivity of the Asyncify expansion to the shape (real FLEngine C: 10.6 % calls, 15.3 % dispatch, 43 ops/function)');
console.log(fs.readFileSync(path.join(R, 'synth-sensitivity.log'), 'utf8'));
