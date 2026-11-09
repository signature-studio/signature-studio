// lat1 (decision 0194): one measurement run on the REAL engine in an isolated headless Chrome (CDP input = test simulation, not a user): the six input kinds, each repeated, with the engine idle and with `Space` playback running.
// usage: node tools/run.mjs --config SHIPH|F1xH|hz120|dedupe1 --out DIR [--reps 10] [--round 1] [--phases idle,play] [--port 4410]
import fs from 'node:fs';
import path from 'node:path';
import { openServer, openSession, join, stages, sleep, say, load1, ROOT , SRC } from './lib.mjs';
const { descendants } = await import(`${SRC}/test/e2e/cdp.mjs`);

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const config = opt('config', 'SHIPH');
const out = path.resolve(opt('out', `${ROOT}/lift/out-lat1/runs/${config}`));
const reps = Number(opt('reps', '10'));
const round = Number(opt('round', '1'));
const phases = opt('phases', 'idle,play').split(',');
const only = opt('only', '');
const f8reps = Number(opt('f8reps', String(reps)));
// each FILE / context-menu open uses 2 of the runtime's 64 SetWindowsHookExW slots for the rest of the session (decision 0194 section 9): a session can open about 31 menus; --menu-reps keeps the menu cells inside that
const menuReps = Number(opt('menu-reps', String(reps)));
const maxLoad = Number(opt('max-load', '8'));
const maxWaitMin = Number(opt('max-wait-min', '20'));
const CONFIGS = {
  SHIPH: '',
  F1xH: '?bundle=/assets/fl-engine-ship1-F1xH/manifest.json',
  hz120: '?hz=120',
  dedupe1: '?dedupe=1',
};
if (!(config in CONFIGS)) throw new Error('unknown config ' + config);
fs.mkdirSync(out, { recursive: true });

// ---- /proc thread CPU (100 Hz ticks) ----
function threads(root) {
  const m = new Map();
  const pids = descendants(root);
  const renderers = new Set();
  for (const pid of pids) { try { if (fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes('--type=renderer')) renderers.add(pid); } catch { /* gone */ } }
  for (const pid of pids) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
        const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]), main: tid === String(pid) && renderers.has(pid) });
      } catch { /* gone */ }
    }
  }
  return m;
}
const cpuDiff = (a, b) => {
  let worker = 0, main = 0, all = 0;
  for (const [k, v1] of b) { const d = (v1.ticks - (a.get(k)?.ticks ?? 0)) / 100; all += d; if (/Worker/i.test(v1.comm)) worker += d; if (v1.main) main += d; }
  return { workerS: +worker.toFixed(3), mainS: +main.toFixed(3), allS: +all.toFixed(3) };
};

// wait (in this process) until the 1-minute load average is at most maxLoad, at most maxWaitMin minutes; the load at the start is recorded either way
const waitStart = Date.now();
while (load1() > maxLoad && Date.now() - waitStart < maxWaitMin * 60000) await sleep(10000);
const loadAtStart = load1(), waitedS = Math.round((Date.now() - waitStart) / 1000);
const srv = await openServer(out, Number(opt('port', '4410')));
const S = await openSession(out, 'run');
const result = { tool: 'lat1 run.mjs', config, params: CONFIGS[config], round, reps, startedAt: new Date().toISOString(), chrome: S.browser.version, loadStart: loadAtStart, waitedForLoadS: waitedS, maxLoad, cells: [], notes: [] };
const fromImg = (ix, iy) => [Math.round((ix / 0.8333 - S.rect.x) * 1920 / S.rect.width), Math.round((iy / 0.8333 - S.rect.y) * 1080 / S.rect.height)];
// targets in SCREEN coordinates (1920x1080 virtual screen), read off the main window of the default entry (SHIPH). They are NOT derived from CSS positions: a bundle with a longer description (F1xH) lays the page out lower,
// and the first version of this table (CSS -> screen at run time) aimed 49 px low there (the F1xH runs of the first two series are discarded for that reason). The page's rectangle is still read before every repetition, for the mapping screen -> client.
const T = {
  file: [18, 19], step1: [597, 144], step3: [628, 144], chanBtn: [522, 144], fader: [526, 800], knob: [407, 143], tempo: [463, 19], mixerArea: [666, 672],
};
const pt = (t) => t;

let mainLid = 14;
let file, step1, chanBtn, fader, knob, tempo, mixer, step3;
// the desktop's rectangle in the page can move (a status line wraps, the header grows): it is read again before every repetition and the targets are recomputed (the F1xH-r1 run of the first series aimed 49 px low because of a stale rectangle)
async function retarget() { await S.refreshRect(); file = pt(T.file); step1 = pt(T.step1); chanBtn = pt(T.chanBtn); fader = pt(T.fader); knob = pt(T.knob); tempo = pt(T.tempo); mixer = pt(T.mixerArea); step3 = pt(T.step3); }
let bg = new Set();
const settle = (ms, quiet) => S.settle(ms, quiet, bg);
const bgKey = (p) => `${p.lid}|${p.x}|${p.y}|${p.h}`;
const isResp = (p) => p.changed > 0 && !bg.has(bgKey(p));

async function learnBackground(seconds, label) {
  await settle(10000, 600);
  await S.reset();
  const t0 = Date.now(), c0 = threads(S.browser.pid), l0 = load1();
  await sleep(seconds * 1000);
  const c1 = threads(S.browser.pid);
  const c = await S.collect();
  const wall = (Date.now() - t0) / 1000;
  bg = new Set(c.puts.filter((p) => p.changed > 0).map(bgKey));
  const chg = c.puts.filter((p) => p.changed > 0);
  const cell = { cell: `background-${label}`, phase: label, wallS: +wall.toFixed(2), load0: +l0.toFixed(1), load1: +load1().toFixed(1), cpu: cpuDiff(c0, c1), draws: c.puts.length, changingDraws: chg.length, drawsPerS: +(c.puts.length / wall).toFixed(2), changingPerS: +(chg.length / wall).toFixed(2), bgKeys: [...bg], draws2: c.puts.filter((p) => p.msg).map((p) => ({ lid: p.lid, r: [p.x, p.y, p.w, p.h], chg: p.changed, post: +p.msg.postedAt.toFixed(3), recv: +p.msg.tRecv.toFixed(3), put: +p.tEnd.toFixed(3) })) };
  result.cells.push(cell);
  say(`background ${label}: ${cell.draws} draws (${cell.drawsPerS}/s), ${chg.length} changing (${cell.changingPerS}/s), classes ${[...bg].join(' ')}, Worker CPU ${cell.cpu.workerS}s / ${wall.toFixed(1)}s, load ${cell.load0}`);
}

/** one cell: `reps` repetitions of `act(rep)`; each rep is separated by a settle; the records are joined once at the end */
async function cell(name, phase, n, act, { gapMs = 350, settleMs = 4000, quietMs = 350, streamGapOk = false } = {}) {
  if (only && !only.split(',').includes(name)) return;
  await settle(10000, 600);
  await S.reset();
  const t0 = Date.now(), c0 = threads(S.browser.pid), l0 = load1();
  const marks = [];
  for (let i = 0; i < n; i++) {
    await retarget();
    marks.push(performance.timeOrigin + performance.now());
    await act(i);
    const ok = await settle(settleMs, quietMs);
    if (!ok) result.notes.push(`${name}/${phase} rep ${i}: not quiet after ${settleMs} ms`);
    await sleep(gapMs);
  }
  const c1 = threads(S.browser.pid);
  const wall = (Date.now() - t0) / 1000;
  const c = await S.collect();
  const alive = await S.page.ev('window.__app.state');
  const j = join(c, { isResponse: isResp });
  // the first draw on a window other than the main one, at or after each input's injection (a menu / a plug-in list opening)
  const pop = c.puts.filter((p) => p.msg && p.changed > 0 && p.lid !== mainLid).sort((a, b) => a.msg.postedAt - b.msg.postedAt);
  const inputs = j.map((o) => {
    const st = stages(o);
    const p = pop.find((q) => q.msg.postedAt >= o.tInject && q.msg.postedAt - o.tInject < 1500);
    return { desc: o.desc, ...Object.fromEntries(Object.entries(st).map(([k, v]) => [k, v === null ? null : +v.toFixed(3)])), lid: o.first?.lid ?? null, rect: o.first?.rect ?? null, changedPx: o.first?.changedPx ?? null,
      popup_total_from_event: p ? +(p.tEnd - (o.tEvent ?? o.tHandler ?? o.tPage)).toFixed(3) : null, popup_lid: p ? p.lid : null, popup_inject_to_post: p ? +(p.msg.postedAt - o.tInject).toFixed(3) : null, tInject: o.tInject, tPage: o.tPage };
  });
  const draws = c.puts.filter((p) => p.msg).map((p) => ({ lid: p.lid, r: [p.x, p.y, p.w, p.h], chg: p.changed, resp: isResp(p), post: +p.msg.postedAt.toFixed(3), recv: +p.msg.tRecv.toFixed(3), put: +p.tEnd.toFixed(3), raf: p.raf === null ? null : +p.raf.toFixed(3) }));
  const rec = { hook: c.hook, cell: name, phase, n, wallS: +wall.toFixed(2), load0: +l0.toFixed(1), load1: +load1().toFixed(1), cpu: cpuDiff(c0, c1), alive, nInputs: inputs.length, nDraws: c.puts.length, nChanging: c.puts.filter((p) => p.changed > 0).length, nResp: draws.filter((d) => d.resp).length, marks, inputs, draws };
  result.cells.push(rec);
  const first = inputs.map((x) => x.total_event_to_put).filter((x) => x !== null).sort((a, b) => a - b);
  say(`cell ${name}/${phase}: ${inputs.length} inputs, ${first.length} with a response, p50 event->put ${first.length ? first[Math.floor(first.length / 2)].toFixed(1) : '-'} ms; draws ${rec.nDraws} (${rec.nChanging} changing); Worker CPU ${rec.cpu.workerS}s/${rec.wallS}s; load ${rec.load0}; state ${alive}`);
  if (alive !== 'engine-idle') throw new Error(`the engine is not idle after cell ${name}: ${alive}`);
}

const clickAt = async (p, hold = 200, btn = 'left') => { await S.move(p[0], p[1]); await sleep(120); await S.down(p[0], p[1], btn); await sleep(hold); await S.up(p[0], p[1], btn); };

async function runPhase(phase) {
  // (a) left click on a button: a channel-rack step button (down = the step toggles; up = the release)
  await cell('a_click_step', phase, reps, async (i) => { const p = i % 2 ? step3 : step1; await S.move(p[0], p[1]); await sleep(150); await settle(2000, 300); await S.down(p[0], p[1]); await sleep(200); await S.up(p[0], p[1]); });
  // (b) hover: one pointer move, alternately over the FILE menu title and the mixer
  await cell('b_hover', phase, reps * 2, async (i) => { const p = i % 2 ? mixer : file; await S.move(p[0], p[1]); }, { gapMs: 300 });
  // (c) continuous drag: a mixer fader and a channel-rack knob. One repetition = press, 8 moves up and 8 moves back at 16 ms, release, so that every repetition starts where the last one ended
  // (the first version dragged the fader 72 px up and then 72 px down in turn; the down drags missed the thumb after the first ones and their moves changed nothing)
  const excursion = (name, pos, step) => cell(name, phase, reps, async () => {
    await S.move(pos[0], pos[1]); await sleep(150); await settle(2000, 300);
    await S.down(pos[0], pos[1]); await sleep(30);
    for (let k = 1; k <= 8; k++) { await S.dragMove(pos[0], pos[1] - k * step); await sleep(16); }
    for (let k = 7; k >= 0; k--) { await S.dragMove(pos[0], pos[1] - k * step); await sleep(16); }
    await S.up(pos[0], pos[1]);
  }, { gapMs: 400, settleMs: 6000 });
  await excursion('c_drag_knob', knob, 5);
  await excursion('c_drag_fader', fader, 6);
  // (d) wheel over the tempo field (the playlist's wheel scroll stops the engine at user32!ScrollWindowEx, see the document)
  await cell('d_wheel_tempo', phase, reps, async (i) => { await S.move(tempo[0], tempo[1]); await sleep(150); await settle(2000, 300); await S.wheel(tempo[0], tempo[1], i % 2 ? -120 : 120); });
  // (e) keys: F8 opens the plug-in list (a full-screen window of the guest), Escape closes it
  await cell('e_key_F8_open_close', phase, f8reps, async () => { await S.tap('F8', 150); await settle(7000, 900); await S.tap('Escape', 150); await settle(7000, 900); }, { gapMs: 600, settleMs: 9000, quietMs: 900 });
  // (f) opening a window: the FILE menu (click) and the channel button's context menu (right click); Escape closes
  await cell('f_open_file_menu', phase, menuReps, async () => { await S.move(file[0], file[1]); await sleep(150); await settle(2000, 300); await S.down(file[0], file[1]); await sleep(200); await S.up(file[0], file[1]); await settle(3000, 400); await S.tap('Escape', 150); });
  await cell('f_open_context_menu', phase, menuReps, async () => { await S.move(chanBtn[0], chanBtn[1]); await sleep(150); await settle(2000, 300); await S.down(chanBtn[0], chanBtn[1], 'right'); await sleep(200); await S.up(chanBtn[0], chanBtn[1], 'right'); await settle(3000, 400); await S.tap('Escape', 150); });
  if (phase === 'idle') {
    // Space: start and stop of playback (each pair: down/up of Space twice)
    await cell('e_key_Space_start_stop', phase, reps, async () => { await S.tap('Space', 150); await sleep(2000); await S.tap('Space', 150); }, { gapMs: 500, settleMs: 5000, quietMs: 500 });
  }
}

try {
  await S.open(`http://127.0.0.1:${srv.port}/${CONFIGS[config]}`);
  await S.waitIdle();
  await S.refreshRect();
  result.timeline = JSON.parse(await S.page.ev('JSON.stringify(window.__app.timeline)'));
  result.engineKind = await S.page.ev('window.__app.engineKind');
  result.bundle = await S.page.ev('window.__app.manifest.bundle.name + " " + window.__app.manifest.bundle.version');
  result.wizardClosed = await S.closeWizard();
  mainLid = await S.page.ev('window.__app.mainWindow()?.lid ?? 14');
  result.mainLid = mainLid;
  await sleep(2500);
  if (phases.includes('idle')) {
    await learnBackground(6, 'idle');
    await runPhase('idle');
  }
  if (phases.includes('play')) {
    await settle(8000, 600);
    await S.tap('Space', 150);
    await sleep(3000);
    await learnBackground(4, 'play');
    await runPhase('play');
    await S.tap('Space', 150); await sleep(800);
  }
  result.ok = true;
} catch (e) {
  result.ok = false; result.error = String(e.stack ?? e).slice(0, 1200);
  say('ERROR ' + result.error);
} finally {
  result.loadEnd = load1();
  result.finishedAt = new Date().toISOString();
  try { result.memBytes = await S.page.ev('window.__app.engineMemoryBytes'); result.driverStats = await S.page.ev('JSON.stringify(window.__app.driverStats)'); result.exitInfo = await S.page.ev('JSON.stringify(window.__app.exit)'); result.exitText = await S.page.ev("(document.getElementById('exit')?.textContent ?? '').slice(0, 1500)"); result.lastLog = await S.page.ev('JSON.stringify(window.__app.engineLog().slice(-5).map((l) => l.slice(0, 300)))'); } catch { /* ignore */ }
  fs.writeFileSync(path.join(out, `result-${config}-r${round}.json`), JSON.stringify(result));
  try { await S.page.close(); await S.browser.close(); } catch (e) { say('close: ' + String(e).slice(0, 100)); }
  await srv.stop();
}
process.exit(result.ok ? 0 : 1);
