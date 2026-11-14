// parity1 (decision 0226): the operation matrix on the REAL engine in an isolated headless Chrome, in real time (CDP input = a harness simulation, NOT a user operation).
// For every op: wall seconds, CPU seconds of the engine Worker thread, of the renderer main thread and of all threads of the browser tree (/proc, 100 Hz ticks), and the page's canvas updates (putImageData count / count that changed pixels / changed pixels).
// The default entry (parameter-free URL) is used unless --query is given. The project is the engine's own default project after the Welcome wizard is closed.
// usage: node chrome-ops.mjs --out DIR [--ops a,b,c] [--query '?x=y'] [--max-load 12] [--reps 1] [--screen 1920x1080]
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { openServer, openSession, sleep, say, load1, SRC } from '/home/ubuntu/work/signature-studio-develop/lift/harness/web/lat1/lib.mjs';
const { descendants } = await import(`${SRC}/test/e2e/cdp.mjs`);
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const out = path.resolve(opt('out', path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../lift/out-parity1/runs/chrome-ops')));
const query = opt('query', '');
const only = opt('ops', '').split(',').filter(Boolean);
const maxLoad = Number(opt('max-load', '12')), maxWaitMin = Number(opt('max-wait-min', '10'));
const reps = Number(opt('reps', '1'));
const prof = argv.includes('--prof');   // V8 CPU profile of the engine Worker per op (CDP Profiler, 1 ms)
fs.mkdirSync(out, { recursive: true });
function threads(root) {
  const m = new Map(); const renderers = new Set();
  const pids = descendants(root);
  for (const pid of pids) { try { if (fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes('--type=renderer')) renderers.add(pid); } catch { /* gone */ } }
  for (const pid of pids) {
    let tids = []; try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')')); const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]), main: tid === String(pid) && renderers.has(pid) });
      } catch { /* gone */ }
    }
  }
  return m;
}
const cpuDiff = (a, b) => { let worker = 0, main = 0, all = 0; for (const [k, v1] of b) { const d = (v1.ticks - (a.get(k)?.ticks ?? 0)) / 100; all += d; if (/Worker/i.test(v1.comm)) worker += d; if (v1.main) main += d; } return { workerS: +worker.toFixed(3), mainS: +main.toFixed(3), allS: +all.toFixed(3) }; };

const waitStart = Date.now();
while (load1() > maxLoad && Date.now() - waitStart < maxWaitMin * 60000) await sleep(10000);
const loadStart = load1();
const srv = await openServer(out, Number(opt('port', '4630')));
const S = await openSession(out, 'ops');
const page = S.page;
const workers = [];
S.browser.cdp.on((m) => { if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId); });
await page.S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
const W = (method, params) => S.browser.cdp.send(method, params, workers[0]);
const result = { tool: 'parity1 chrome-ops.mjs', query, startedAt: new Date().toISOString(), chrome: S.browser.version, loadStart, ops: [], notes: ['INPUT IS A HARNESS SIMULATION (CDP), NOT A USER OPERATION'] };
const FPS = 16;   // event period in ms (62.5 events/s)
const mouse = (type, sx, sy, o = {}) => S.mouse(type, sx, sy, o);
const mid = { button: 'middle', buttons: 4 }, lef = { button: 'left', buttons: 1 };
async function drag(btn, x0, y0, x1, y1, n, dt = FPS) {
  const o = btn === 'M' ? mid : lef;
  await mouse('mouseMoved', x0, y0); await sleep(dt);
  await mouse('mousePressed', x0, y0, { ...o, clickCount: 1 }); await sleep(dt);
  for (let i = 1; i <= n; i++) { await mouse('mouseMoved', x0 + Math.round((x1 - x0) * i / n), y0 + Math.round((y1 - y0) * i / n), o); await sleep(dt); }
  await mouse('mouseReleased', x1, y1, { button: o.button, buttons: 0, clickCount: 1 }); await sleep(300);
}
async function moves(x0, y0, x1, y1, n, dt = FPS) { for (let i = 0; i <= n; i++) { await mouse('mouseMoved', x0 + Math.round((x1 - x0) * i / n), y0 + Math.round((y1 - y0) * i / n)); await sleep(dt); } }
async function wheel(x, y, delta, n, dt = FPS) { for (let i = 0; i < n; i++) { await S.wheel(x, y, -delta); await sleep(dt); } }
async function click(x, y, b = 'left') { await mouse('mouseMoved', x, y); await sleep(30); await mouse('mousePressed', x, y, { button: b, buttons: b === 'left' ? 1 : 2, clickCount: 1 }); await sleep(50); await mouse('mouseReleased', x, y, { button: b, buttons: 0, clickCount: 1 }); }
const KC = { F5: 116, F7: 118, F8: 119, F9: 120, F10: 121, Escape: 27, Space: 32, Enter: 13 };
async function tapKey(name) { const vk = KC[name]; const e = { code: name === 'Space' ? 'Space' : name, key: name === 'Space' ? ' ' : name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk }; await page.S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...e }); await sleep(40); await page.S('Input.dispatchKeyEvent', { type: 'keyUp', ...e }); }
const OPS = {
  idle_10s: async () => { await sleep(10000); },
  play_10s: async () => { await tapKey('Space'); await sleep(10000); await tapKey('Space'); await sleep(500); },
  pl_wheel_60hz: async () => { await wheel(1300, 400, 120, 60); await wheel(1300, 400, -120, 60); await sleep(500); },
  pl_midbutton_drag_3s: async () => { await drag('M', 1600, 420, 1000, 420, 190); },
  pl_leftdrag_paint_2s: async () => { await drag('L', 1160, 300, 1800, 300, 125); },
  pl_hover_2s: async () => { await moves(1160, 250, 1800, 700, 125); },
  cr_vol_drag_2s: async () => { await drag('L', 407, 174, 407, 120, 125); },
  mx_hover_2s: async () => { await moves(540, 620, 1100, 900, 125); },
  mx_fader_drag_2s: async () => { await drag('L', 570, 802, 570, 900, 125); },
  win_split_drag_2s: async () => { await drag('L', 600, 308, 600, 250, 125); await drag('L', 600, 250, 600, 308, 125); },
  pr_open_wheel: async () => { await tapKey('F7'); await sleep(3000); await wheel(1100, 500, 120, 60); await wheel(1100, 500, -120, 60); await sleep(300); await tapKey('F7'); await sleep(1500); },
  cr_btn_mx_hover: async () => { await click(522, 145); await sleep(1500); await moves(540, 620, 1100, 900, 125); await sleep(500); },
  menu_file: async () => { await click(18, 20); await sleep(1000); await tapKey('Escape'); await sleep(500); },
  f8_open_4s: async () => { await tapKey('F8'); await sleep(4000); await tapKey('Escape'); await sleep(3000); },
  settings_tabs: async () => { await tapKey('F10'); await sleep(3000); for (const [x, y] of (JSON.parse(process.env.SETTINGS_TABS ?? '[[772,138]]'))) { await click(x, y); await sleep(1500); } await tapKey('Escape'); await sleep(1500); },
};
const bgKey = (p) => `${p.lid}|${p.x}|${p.y}|${p.h}`;
try {
  const url = `http://127.0.0.1:${srv.port}/${query}`;
  say(`open ${url}`);
  await S.open(url); await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(4000);
  await S.settle(10000, 600, null);
  result.state = await page.ev('window.__app.state'); result.rect = S.rect;
  for (let r = 0; r < reps; r++) for (const name of (only.length ? only : Object.keys(OPS))) {
    await S.settle(8000, 600, null); await S.reset();
    const c0 = threads(S.browser.pid), t0 = Date.now(), l0 = load1();
    let err = null;
    if (prof) { await W('Profiler.enable', {}); await W('Profiler.setSamplingInterval', { interval: 1000 }); await W('Profiler.start', {}); }
    try { await OPS[name](); } catch (e) { err = String(e?.message ?? e); }
    if (prof) { const pr = (await W('Profiler.stop', {})).profile; fs.writeFileSync(path.join(out, `prof-${name}-${r}.cpuprofile`), JSON.stringify(pr)); }
    const wall = (Date.now() - t0) / 1000; const c1 = threads(S.browser.pid);
    let puts = [];
    try { puts = (await S.collect()).puts; } catch (e) { err = (err ?? '') + ' collect:' + e; }
    const cpu = cpuDiff(c0, c1);
    const changed = puts.filter((p) => p.changed > 0);
    const perLid = {}; for (const p of changed) { const k = 'lid' + p.lid; perLid[k] = (perLid[k] ?? 0) + 1; }
    const st = await page.ev('window.__app.state').catch(() => '?');
    const rec = { op: name, rep: r, wallS: +wall.toFixed(2), ...cpu, draws: puts.length, changedDraws: changed.length, changedPx: changed.reduce((a, p) => a + p.changed, 0), drawsPerS: +(puts.length / wall).toFixed(1), changedPerS: +(changed.length / wall).toFixed(1), perLid, load: +((l0 + load1()) / 2).toFixed(1), state: st, err };
    result.ops.push(rec); say(JSON.stringify(rec));
    fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result, null, 1));
    if (st !== 'engine-idle') { result.notes.push(`state ${st} after ${name}: stop`); break; }
  }
  result.bodyTail = (await page.ev('document.body.innerText')).split('\n').filter((l) => /stopped at a wall|fle:|exit code|The engine stopped/i.test(l)).slice(0, 4);
} catch (e) { result.error = String(e?.stack ?? e); say('ERROR ' + result.error); }
fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result, null, 1));
try { await S.browser.close(); } catch { /* none */ }
await srv.stop();
process.exit(0);
