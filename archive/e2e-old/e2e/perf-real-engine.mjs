// Opens the page WITHOUT any parameter (the default entrance /assets/manifest.json) in the isolated headless Chrome of the e2e tests against `vite preview`, lets the REAL engine start, and reads what the
// measurement beacon (decision 0152) delivered: the 'first-idle' beacon, a 'periodic' one 30 s later, and the 'exit' one after the page stops the engine. It asserts nothing about speed: it prints what was measured.
// usage: FL_ASSETS_DIR=<dir whose manifest.json is the real engine's> [FL_PERF_LOG=<jsonl>] node test/e2e/perf-real-engine.mjs [--out DIR] [--wait-idle-s 240] [--inputs 8] [--port N]
// The default entrance (no URL parameter: the page has none) is the one the owner opens. Decision 0192: it also checks the beacon's screen blocks (rAF, canvas, client, hz). The headless Chrome's rAF is 60 Hz whatever the real screen is.
// Without FL_PERF_LOG the server's own default is used and tested: perf-log.jsonl in the PARENT of FL_ASSETS_DIR. The file is never deleted: only the lines added during the run are read. Still, point FL_ASSETS_DIR at a copy (symlinks) of the bundle, not at the production directory: the run adds its records to that log.
// FL_ASSETS_DIR is only READ. The default entrance is used on purpose: what the owner opens is what is tested.
import fs from 'node:fs';
import path from 'node:path';
import { freePort, start } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { perfLogPath } from '../../tools/perf-middleware.ts';
import { beaconProblems } from '../../shared/perf.ts';
import { openBrowser, sleep } from './browser.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> [FL_PERF_LOG=<file>] node test/e2e/perf-real-engine.mjs [--out DIR]'); process.exit(2); }
const LOG = perfLogPath(process.env);
const out = path.resolve(opt('out', path.join(workDirOf('perf-real'), 'out')));
fs.mkdirSync(out, { recursive: true });
const waitIdleS = Number(opt('wait-idle-s', '240'));
const nInputs = Number(opt('inputs', '8'));
// the server appends a record with one write while this reads the file: a last line that is not complete yet is not a record yet (decision 0192: seen once, SyntaxError at the end of a 4 KB line)
const readAll = () => (fs.existsSync(LOG) ? fs.readFileSync(LOG, 'utf8').split('\n').filter(Boolean).flatMap((l, i, a) => { try { return [JSON.parse(l)]; } catch (e) { if (i === a.length - 1) return []; throw e; } }) : []);
const before = readAll().length;
const readLog = () => readAll().slice(before);
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);

const port = opt('port', '') ? Number(opt('port', '')) : await freePort();
const server = start(path.join(APP, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env }, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
const browser = await openBrowser(workDirOf('perf-real'), 'perf-real');
const page = await browser.newPage({ width: 2400, height: 1300 });
const result = { url: `http://127.0.0.1:${port}/`, perfLog: LOG, checks: [] };
const check = (name, ok, detail = '') => { result.checks.push({ name, ok: !!ok, detail }); say(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? '  -- ' + String(detail).slice(0, 300) : ''}`); };
try {
  const t0 = Date.now();
  await page.goto(result.url);
  let idleAt = null;
  while ((Date.now() - t0) / 1000 < waitIdleS) {
    await sleep(2000);
    const j = JSON.parse(await page.ev('JSON.stringify({ state: window.__app?.state, kind: window.__app?.engineKind, tl: window.__app?.timeline, err: window.__app?.error })').catch(() => '{}'));
    if (j.state === 'error' || j.state === 'exited') { say(`page ended early: ${JSON.stringify(j).slice(0, 600)}`); break; }
    if (j.tl && typeof j.tl.idle === 'number') { idleAt = j.tl.idle; break; }
  }
  const kind = await page.ev('window.__app.engineKind');
  check('the default entrance (no parameters) runs the REAL engine: engineKind emscripten-fruity-v1', kind === 'emscripten-fruity-v1', JSON.stringify({ kind }));
  check('the engine reached its first idle', idleAt !== null, idleAt === null ? 'not within ' + waitIdleS + ' s' : `${(idleAt / 1000).toFixed(1)} s after the navigation started`);
  result.timeline = await page.ev('window.__app.timeline');
  result.firstIdleSeconds = idleAt === null ? null : idleAt / 1000;
  if (idleAt !== null) {
    const first = (await (async () => { for (let i = 0; i < 100; i++) { const r = readLog().find((x) => x.perf?.reason === 'first-idle'); if (r) return r; await sleep(200); } return null; })());
    check('the first-idle beacon is in the server log (reason first-idle, engine kind emscripten-fruity-v1, remote loopback)', first && first.perf.bundle.engineKind === 'emscripten-fruity-v1' && first.remote === 'loopback', JSON.stringify(first?.perf?.bundle));
    if (first) {
      check('the first-idle beacon already says the engine is idle (state engine-idle)', first.perf.engine.state === 'engine-idle', first.perf.engine.state);
      check('it carries the start-up time the page measured (startup.firstIdleSeconds = timeline.idle / 1000)', Math.abs(first.perf.startup.firstIdleSeconds - idleAt / 1000) < 0.01, JSON.stringify(first.perf.startup));
      result.firstBeacon = first;
    }
    // some input: pointer moves over the main window, 2 s apart (the engine takes input only between its frames)
    for (let i = 0; i < nInputs; i++) {
      await page.ev(`window.__app.sendRaw({ k: 'mouse', kind: 1, x: ${500 + 40 * i}, y: ${300 + 10 * i}, data: 0 })`);
      await sleep(2000);
    }
    say(`sent ${nInputs} pointer moves; waiting for the periodic beacon (30 s after the first)`);
    const periodic = await (async () => { for (let i = 0; i < 300; i++) { const r = readLog().find((x) => x.perf?.reason === 'periodic'); if (r) return r; await sleep(200); } return null; })();
    check('a periodic beacon followed about 30 s after the first', periodic !== null, periodic ? `seq ${periodic.perf.seq}` : 'none within 60 s');
    if (periodic) result.periodicBeacon = periodic;
    // decision 0192: what the beacons say about the screen. Headless Chrome's rAF is 60 Hz (a virtual vsync), so these numbers say 'a 60 Hz screen' — they are a check that the fields are filled, not a measurement of any real screen
    const p = periodic?.perf ?? first?.perf;
    if (p) {
      const d = p.display;
      check('the beacon has the screen blocks and passes the shape check (beaconProblems: none)', p.v === 3 && /^[0-9a-f]{12}$/.test(p.sid) && d && p.client && p.hz && beaconProblems(p).length === 0, JSON.stringify(beaconProblems(p)).slice(0, 300));
      // decision 0202: the inputs above were made by window.__app.sendRaw (from 'test'): there is no DOM event, so no event.timeStamp: the new number is empty (null, not zero) and version 2's number (listener -> put) is there
      // decision 0205: an input is answered only by a frame the Worker posted before the engine's next idle (+ 1 ms); with dedupe on by default (decision 0201) a pointer move that changes nothing has no answer, so listenerToPut can be empty. What must hold: no timeStamp -> eventToRaf empty and stages null; every processed input is accounted for as answered, late or noAnswer.
      check('input (decisions 0202/0205): the 8 test inputs have no event timeStamp, so eventToRaf is empty and stages is null; answered + late + noAnswer = processed', p.input?.sent >= nInputs && p.input.eventToRaf?.n === 0 && p.input.eventToRaf.p95Ms === null && p.input.stages === null && p.input.answered + p.input.late + p.input.noAnswer === p.input.processed, JSON.stringify(p.input).slice(0, 400));
      // decision 0202: the screen meter asks for animation frames in 2 s bursts (the start, the first idle, then every 60 s), not at every frame: in the ~35 s of this run a continuous loop would have counted ~2,000 callbacks (the 1,801 of the earlier run), bursts about 140
      check('the screen meter ran in bursts (decision 0202): mode burst, 2 s every 60 s, 1-2 bursts so far, and far fewer rAF intervals than a continuous loop (n < 600; it was 1801 with the loop)', d?.mode === 'burst' && d.burst?.onMs === 2000 && d.burst?.periodMs === 60000 && d.burst?.count >= 1 && d.burst.count <= 2 && d.raf?.total?.n > 100 && d.raf.total.n < 600, JSON.stringify({ mode: d?.mode, burst: d?.burst, n: d?.raf?.total?.n }));
      check('rAF was measured since the first idle (n > 100: one burst) and its rate is a number (the headless Chrome gives about 60)', d?.raf?.sinceIdle?.n > 100 && d.raf.sinceIdle.hz > 20 && d.raf.sinceIdle.hz < 400, JSON.stringify(d?.raf?.sinceIdle));
      check('the canvas block is null: under display method D the Worker presents and the page draws no pixel (decision 0223), so it counts none', d?.canvas === null, JSON.stringify(d?.canvas));
      check('the client block: browser Chrome, a viewport, physical cores unavailable (null with a reason), page reached on a localhost host', p.client?.browser?.name === 'Chrome' && p.client.viewport?.width > 0 && p.client.physicalCores?.value === null && p.client.page?.hostKind === 'localhost', JSON.stringify(p.client));
      check('the hz block: the engine rate (a number: 60 in the headless Chrome, whose rAF is 60 Hz), the rAF rate and a verdict', typeof p.hz?.engineHz === 'number' && typeof p.hz.rafHz === 'number' && typeof p.hz.fit === 'string', JSON.stringify(p.hz));
      result.screen = { raf: d?.raf, canvas: d?.canvas, visibility: d?.visibility, longTasks: d?.longTasks, events: d?.events, hz: p.hz, client: p.client };
    }
  }
  await page.ev('window.__app.stop()').catch(() => undefined);
  const exitRec = await (async () => { for (let i = 0; i < 150; i++) { const r = readLog().find((x) => x.perf?.reason === 'exit' || x.perf?.reason === 'error'); if (r) return r; await sleep(200); } return null; })();
  check('after the page stopped the engine an exit (or error) beacon arrived', exitRec !== null, exitRec ? `${exitRec.perf.reason}: ${JSON.stringify(exitRec.perf.engine.exit)}` : 'none');
  if (exitRec) result.endBeacon = exitRec;
  result.log = readLog();
  result.page = await page.ev('JSON.stringify({ history: window.__app.perf.history.map((h) => ({ seq: h.seq, reason: h.reason, bytes: h.bytes, status: h.result.status, stored: h.result.stored })), sent: window.__app.perf.sent })').then(JSON.parse);
  const rows = await (async () => { await page.ev("document.getElementById('perf-panel').open = true"); await sleep(1300); return page.ev("[...document.querySelectorAll('#perf-body tr[data-row]')].map((tr) => ({ id: tr.dataset.row, status: tr.dataset.status, cells: [...tr.children].map((c) => c.textContent) }))"); })();
  result.panel = rows;
  say('panel rows: ' + JSON.stringify(rows.map((r) => [r.id, r.status, r.cells[1]])));
  const errs = page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION'));
  check('no console errors or exceptions', errs.length === 0, errs.join(' | ').slice(0, 300));
  try { fs.writeFileSync(path.join(out, 'real-engine-perf-page.png'), await page.shot(0, 0, 2400, 1300)); } catch (e) { say('screenshot failed: ' + e.message); }
} catch (e) {
  check('scenario ran to completion', false, String(e.stack ?? e));
} finally {
  fs.writeFileSync(path.join(out, 'perf-real-engine-result.json'), JSON.stringify(result, null, 2));
  await page.close().catch(() => undefined);
  const nl = await browser.close();
  say(nl.line);
  await server.stop();
}
const failed = result.checks.filter((c) => !c.ok).length;
say(`${result.checks.length - failed}/${result.checks.length} checks passed`);
process.exit(failed ? 1 : 0);
