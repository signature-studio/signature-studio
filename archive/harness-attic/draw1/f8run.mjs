// draw1 (decision 0203): the F8 full-screen plug-in list on the REAL engine in an isolated headless Chrome (CDP input = test simulation, not a user).
// One session: load the bundle, close the Welcome wizard, then REPS times { F8 -> OPENMS of quiet -> Escape -> CLOSEMS of quiet }, with
//   mode cpu  : per phase the Worker thread's CPU seconds (/proc, 100 Hz ticks) and the putImageData draws the page saw (count, pixels, changed pixels, per window)
//   mode prof : the same for 1 rep, plus a V8 CPU profile of the Worker (CDP Profiler, 1 ms sampling); the profile is written as .cpuprofile
// usage: node f8run.mjs --bundle DIRNAME|default --out DIR [--mode cpu|prof] [--reps 3] [--open-ms 6000] [--close-ms 3000] [--query '&x=y']
import fs from 'node:fs';
import path from 'node:path';
import { openServer, openSession, sleep, say, load1, SRC } from '/home/ubuntu/work/signature-studio-develop/lift/harness/web/lat1/lib.mjs';
const { descendants } = await import(`${SRC}/test/e2e/cdp.mjs`);

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const bundle = opt('bundle', 'default');
const out = path.resolve(opt('out', '/tmp/opencode/draw1-run'));
const mode = opt('mode', 'cpu');
const reps = Number(opt('reps', '3'));
const openMs = Number(opt('open-ms', '6000'));
const closeMs = Number(opt('close-ms', '3000'));
const query = opt('query', '');
const midMs = Number(opt('mid-ms', '4000'));
const maxLoad = Number(opt('max-load', '8'));
const maxWaitMin = Number(opt('max-wait-min', '6'));
fs.mkdirSync(out, { recursive: true });

function threads(root) {
  const m = new Map();
  const pids = descendants(root);
  for (const pid of pids) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
        const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]) });
      } catch { /* gone */ }
    }
  }
  return m;
}
const workerCpu = (a, b) => { let w = 0; for (const [k, v1] of b) if (/Worker/i.test(v1.comm)) w += (v1.ticks - (a.get(k)?.ticks ?? 0)) / 100; return +w.toFixed(3); };

const waitStart = Date.now();
while (load1() > maxLoad && Date.now() - waitStart < maxWaitMin * 60000) await sleep(5000);
const loadStart = load1();
const srv = await openServer(out, Number(opt('port', '4620')));
const S = await openSession(out, 'f8');
const workers = [];
S.browser.cdp.on((m) => { if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId); });
await S.page.S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
const result = { tool: 'draw1 f8run.mjs', bundle, mode, reps, openMs, closeMs, query, loadStart, startedAt: new Date().toISOString(), chrome: S.browser.version, reps_: [], notes: [] };
const summarize = (puts) => {
  const per = {};
  let draws = 0, px = 0, chgPx = 0, chgDraws = 0;
  for (const p of puts) {
    draws++; px += p.w * p.h; chgPx += p.changed; if (p.changed) chgDraws++;
    const k = `lid${p.lid}`; const e = (per[k] ??= { draws: 0, px: 0, changedDraws: 0, changedPx: 0, maxW: 0, maxH: 0 });
    e.draws++; e.px += p.w * p.h; if (p.changed) { e.changedDraws++; e.changedPx += p.changed; } e.maxW = Math.max(e.maxW, p.w); e.maxH = Math.max(e.maxH, p.h);
  }
  return { draws, px, changedDraws: chgDraws, changedPx: chgPx, perWindow: per };
};
try {
  const base = bundle === 'default' ? '' : `bundle=/assets/${bundle}/manifest.json`;
  const url = `http://127.0.0.1:${srv.port}/?${base}${query}`;
  say(`open ${url}`);
  await S.open(url);
  await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(4000);
  result.bundleVersion = await S.page.ev("window.__app && window.__app.manifest ? JSON.stringify(window.__app.manifest.bundle ?? null) : 'n/a'").catch(() => 'n/a');
  await S.settle(10000, 600, null);
  // idle background for 6 s (Worker CPU per second with the main window only)
  await S.reset();
  { const c0 = threads(S.browser.pid), t0 = Date.now(); await sleep(6000); const c1 = threads(S.browser.pid); const c = await S.collect(); result.idle = { wallS: (Date.now() - t0) / 1000, workerCpuS: workerCpu(c0, c1), ...summarize(c.puts) }; say(`idle: Worker CPU ${result.idle.workerCpuS}s in ${result.idle.wallS.toFixed(1)}s, draws ${result.idle.draws}`); }
  for (let r = 0; r < reps; r++) {
    const prof = mode === 'prof' && r === 0;
    const W = (method, params) => S.browser.cdp.send(method, params, workers[0]);
    if (prof) { if (!workers.length) throw new Error('no worker target attached'); await W('Profiler.enable', {}); await W('Profiler.setSamplingInterval', { interval: 1000 }); }
    await S.settle(8000, 600, null);
    await S.reset();
    const rep = { r, load0: +load1().toFixed(1) };
    const c0 = threads(S.browser.pid), t0 = Date.now();
    if (prof) await W('Profiler.start', {});
    await S.tap('F8', 150);
    await sleep(midMs);
    const cMid = threads(S.browser.pid), tMid = Date.now();
    await sleep(openMs - midMs);
    const c1 = threads(S.browser.pid), t1 = Date.now();
    const cOpen = await S.collect();
    await S.reset();
    await S.tap('Escape', 150);
    await sleep(closeMs);
    const c2 = threads(S.browser.pid), t2 = Date.now();
    if (prof) {
      const p = (await W('Profiler.stop', {})).profile;
      fs.writeFileSync(path.join(out, 'worker-f8.cpuprofile'), JSON.stringify(p));
      rep.profileFile = 'worker-f8.cpuprofile';
    }
    const cClose = await S.collect();
    const tl = (c, from) => c.puts.map((p) => [+(p.tEnd - from).toFixed(1), p.lid, p.x, p.y, p.w, p.h, p.changed, p.msg ? +(p.msg.postedAt - from).toFixed(1) : null]);
    rep.timelineOpen = tl(cOpen, t0); rep.timelineClose = tl(cClose, t1);
    rep.open = { wallS: (t1 - t0) / 1000, workerCpuS: workerCpu(c0, c1), ...summarize(cOpen.puts) };
    rep.openFirst = { wallS: (tMid - t0) / 1000, workerCpuS: workerCpu(c0, cMid) }; rep.openRest = { wallS: (t1 - tMid) / 1000, workerCpuS: workerCpu(cMid, c1), draws: cOpen.puts.filter((p) => p.tEnd - t0 >= midMs).length };
    rep.close = { wallS: (t2 - t1) / 1000, workerCpuS: workerCpu(c1, c2), ...summarize(cClose.puts) };
    rep.total = { wallS: (t2 - t0) / 1000, workerCpuS: workerCpu(c0, c2) };
    rep.load1 = +load1().toFixed(1);
    result.reps_.push(rep);
    say(`rep ${r}${prof ? ' (profiled)' : ''}: Worker CPU open ${rep.open.workerCpuS}s/${rep.open.wallS.toFixed(1)}s close ${rep.close.workerCpuS}s/${rep.close.wallS.toFixed(1)}s total ${rep.total.workerCpuS}s, draws open ${rep.open.draws} (changed ${rep.open.changedDraws}) px ${rep.open.px}, load ${rep.load0}->${rep.load1}`);
    await sleep(2500);
  }
  result.windows = await S.page.ev("JSON.stringify((window.__app.appView?.windows ?? []).filter(w=>w.visible).map(w=>w.className))").catch(() => null);
} catch (e) { result.error = String(e?.stack ?? e); say('ERROR ' + result.error); }
finally {
  fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result, null, 1));
  try { await S.page.close(); await S.browser.close(); } catch {}
  await srv.stop();
}
