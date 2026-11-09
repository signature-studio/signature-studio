// lat1: can Chrome's own trace tell the part after putImageData (rAF -> commit -> raster -> swap -> presentation)? One session on the default entry; 12 isolated hover moves while Tracing is on.
import fs from 'node:fs';
import { openServer, openSession, sleep, say, ROOT } from './lib.mjs';
import { PAGE_HOOK } from './hooks.mjs';
const argv = process.argv.slice(2);
const optv = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };

// the same hook plus user-timing marks (they land in the trace with the trace clock): lat1-put-N when a changing draw returns, lat1-raf-N when its rAF callback starts
const HOOK = PAGE_HOOK.replace('L.puts.push(rec);', 'L.puts.push(rec); const idx = L.puts.length - 1; if (changed) performance.mark("lat1-put-" + idx);').replace('rec.raf = now();', 'rec.raf = now(); performance.mark("lat1-raf-" + idx);');
const out = `${ROOT}/lift/out-lat1/runs/trace-${optv('config', 'SHIPH')}-${optv('state', 'idle')}`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out, 4440);
const S = await openSession(out, 'trace', HOOK);
const config = optv('config', 'SHIPH'), state = optv('state', 'idle'), n = Number(optv('n', '24'));
const QUERY = { SHIPH: '', F1xH: '?bundle=/assets/fl-engine-ship1-F1xH/manifest.json', hz120: '?hz=120', dedupe1: '?dedupe=1' }[config];
const cats = 'blink.user_timing,benchmark,cc,viz';
try {
  await S.open(`http://127.0.0.1:${srv.port}/${QUERY}`);
  await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(3000);
  if (state === 'play') { await S.tap('Space', 150); await sleep(3000); }
  const events = [];
  S.browser.cdp.on((m) => { if (m.method === 'Tracing.dataCollected') events.push(...m.params.value); });
  const done = new Promise((res) => S.browser.cdp.on((m) => { if (m.method === 'Tracing.tracingComplete') res(); }));
  await S.browser.cdp.send('Tracing.start', { traceConfig: { includedCategories: cats.split(','), recordMode: 'recordAsMuchAsPossible' }, transferMode: 'ReportEvents' });
  await S.reset();
  const t0 = Date.now();
  await S.refreshRect();
  const pos = (ix, iy) => [Math.round((ix / 0.8333 - S.rect.x) * 1920 / S.rect.width), Math.round((iy / 0.8333 - S.rect.y) * 1080 / S.rect.height)];
  const FILE = pos(28, 190), MIX = pos(560, 700), STEP = pos(505, 293);
  const kinds = [];
  for (let i = 0; i < n; i++) { const p = i % 2 ? MIX : FILE; await S.move(p[0], p[1]); kinds.push('hover'); await sleep(600); }
  for (let i = 0; i < n; i++) { const p = i % 2 ? pos(530, 293) : STEP; await S.move(p[0], p[1]); await sleep(150); await S.down(p[0], p[1]); kinds.push('click-down'); await sleep(250); await S.up(p[0], p[1]); await sleep(500); }
  await S.browser.cdp.send('Tracing.end');
  await done;
  const c = await S.collect();
  fs.writeFileSync(`${out}/trace-events.json`, JSON.stringify(events));
  fs.writeFileSync(`${out}/collect.json`, JSON.stringify({ puts: c.puts.filter((p) => p.changed > 0).map((p) => ({ lid: p.lid, tEnd: p.tEnd, raf: p.raf, rafDone: p.rafDone, rect: [p.x, p.y, p.w, p.h] })), dom: c.dom, t0 }));
  // ---- join: for each changing draw, the compositor frame (PipelineReporter) that carried it, and when that frame was presented ----
  const marks = new Map();
  for (const e of events) if (e.cat === 'blink.user_timing' && /^lat1-(put|raf)-\d+$/.test(e.name)) marks.set(e.name, e.ts / 1000);
  const prEv = events.filter((e) => e.name === 'PipelineReporter').sort((x, y) => x.ts - y.ts);
  const frames = []; const stack = [];
  for (const e of prEv) { if (e.ph === 'b') stack.push({ b: e.ts / 1000, state: e.args?.frame_reporter?.state, seq: e.args?.frame_reporter?.frame_sequence }); else if (e.ph === 'e') { const r = stack.pop(); if (r) { r.e = e.ts / 1000; frames.push(r); } } }
  frames.sort((a, b) => a.b - b.b);
  const stateCount = {}; for (const f of frames) stateCount[f.state] = (stateCount[f.state] ?? 0) + 1;
  const rows = [];
  for (const [k, tput] of marks) {
    if (!k.startsWith('lat1-put-')) continue;
    const idx = k.slice('lat1-put-'.length);
    const traf = marks.get('lat1-raf-' + idx);
    if (traf === undefined) continue;
    // the frame whose begin is at or before the rAF callback and whose end is after it
    const f = frames.find((x) => x.b <= traf && x.e >= traf);
    rows.push({ idx: +idx, putToRaf: traf - tput, frameState: f?.state ?? null, frameBeginToRaf: f ? traf - f.b : null, rafToPresented: f ? f.e - traf : null, putToPresented: f ? f.e - tput : null });
  }
  fs.writeFileSync(`${out}/join.json`, JSON.stringify({ stateCount, rows }, null, 1));
  const qv = (v, p) => { v = v.filter((x) => x !== null).sort((a, b) => a - b); return v.length ? v[Math.min(v.length - 1, Math.ceil(p / 100 * v.length) - 1)].toFixed(1) : '-'; };
  say(`marks ${marks.size}, frames ${frames.length}, states ${JSON.stringify(stateCount)}; joined ${rows.length} changing draws`);
  say(`put->rAF p50 ${qv(rows.map((r) => r.putToRaf), 50)} p95 ${qv(rows.map((r) => r.putToRaf), 95)} | rAF->frame presented p50 ${qv(rows.map((r) => r.rafToPresented), 50)} p95 ${qv(rows.map((r) => r.rafToPresented), 95)} max ${qv(rows.map((r) => r.rafToPresented), 100)} | put->presented p50 ${qv(rows.map((r) => r.putToPresented), 50)} p95 ${qv(rows.map((r) => r.putToPresented), 95)} max ${qv(rows.map((r) => r.putToPresented), 100)}`);
  const names = {};
  for (const e of events) names[e.name] = (names[e.name] ?? 0) + 1;
  say('events ' + events.length + '; top names: ' + Object.entries(names).sort((a, b) => b[1] - a[1]).slice(0, 40).map(([k, v]) => `${k}:${v}`).join(' '));
} finally { try { await S.page.close(); await S.browser.close(); } catch {} await srv.stop(); }
