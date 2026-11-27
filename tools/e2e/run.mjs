#!/usr/bin/env node
// E2E (Issue #1407, extended by #1485): drives the production delivery (wrangler dev --local of app/worker/index.ts, with the bundle in its local R2) in Chromium at 3840 x 2160
// physical pixels, 120 Hz by the BeginFrame clock (lib/browser.mjs), with real mouse and key events (lib/guest.mjs).
// Usage: node tools/e2e/run.mjs [--scenarios smoke | browser-face | --area <領域>] [--only ID[,ID]] [--shard i/n] [--repeat n] [--budget 40m] [--list]
//                              [--displays 4k-dpr1,4k-dpr2,4k-random,dpr:1.25] [--clock raf|begin] [--shots DIR] [--engine-logs DIR] [--report-only]     (run.sh does the same from the repository)
//   --area <領域>: the scenarios of tools/e2e/scenarios/<領域>/ (the 13 areas: page input file menus windows audio settings plugins topbar resilience visual perf delivery).
//     Each file there default-exports defineScenario({ id, covers, displays, expectFail, run }) (lib/scenario.mjs). The files are found by the runner (no table to edit).
//   --scenarios <set>: an older set (scenarios/<set>/index.mjs, { name, expectFail, run } objects: smoke, browser-face, regression); each of them runs in a page of its own, as before.
//     regression (Issue #1408): one scenario per report of the owner. A mark (expectFail) of an older set must be one of its ALLOWED_MARKS or 'new' (checkMarks); a mark may be a list (a scenario with two known defects).
//   --list: starts nothing. One JSON line per area: { area, scenarios, covers, features } (features: the rows of tools/e2e/features/). With --area, that area's line only.
//   --shard i/n: the scenarios sorted by ID, the index % n === i - 1 (deterministic). --repeat n: each selected scenario n times (the row carries `repeat`).
//   --budget: a number with s, m or h. Each display row has its own budget (a clock made per row). A scenario that would start after the row's budget has passed, or
//     that needs more time than the row has left (needsMs, e.g. resilience/wait-idle-30min: 30 min), is not run: it is written with skipped: true (the row then fails).
//   --only: IDs (page/canvas-fills-viewport) or the older names (file-open-demo). IDs with an area (file/open-box-shows) and no --area or --scenarios take the scenarios of those areas.
//   --displays: the rows to run. A scenario runs in a row only when its own displays (the declaration) include that row's display name; --displays does not widen a scenario.
//     4k-dpr1 (3840 x 2160 CSS px, DPR 1); 4k-dpr2 (1920 x 1080, DPR 2); 4k-random (two DPRs drawn at random from 1.01 to 3.00: CSS px 3840 / DPR, 2160 / DPR);
//     dpr:<number> (e.g. dpr:1.25: CSS px 3840 / DPR, 2160 / DPR). Without --displays the rows are the names the selected scenarios declare (areas: 4k-dpr1,4k-random; the smoke set: 4k-dpr1,4k-dpr2,4k-random).
//     A row's page must have the row's DPR (a fixed DPR; dprOk): a page at another DPR fails the scenario.
//   --clock: raf (default: the page's own rAF, the frame-rate limit off) or begin (120 Hz by the BeginFrame clock; see lib/browser.mjs).
//   --shots DIR: a test aid, PNG pictures of the page after each scenario are written to DIR (the directory is made; nothing is removed).
//   --engine-logs DIR: a test aid, the engine log of each scenario (the lines the page keeps, from the scenario's start) is written to DIR/<display>-<dpr>-<id>.log.
//   --report-only: the run reports and does not judge the scenarios. Exit 0 when every row is printed with all its scenario lines and its error is null (runPasses), whatever the scenarios' ok and counters
//     (the caller checks the rows' structure, e.g. servedHeadersMatch). A row or line missing, or an error (the setup, a row), still exits 1. The log says 'report-only: exit 0'.
// A shard (or an area) starts the page ONCE per row: the scenarios run one after another in it, each after File > New (lib/common.mjs resetProject). The smoke set runs each in a page of its own.
// stdout: one JSON line per display row (the JSON is written also when the run failed: its error fields say what). stderr: the log.
// Exit 0 only when every row meets the acceptance (rowPasses, the same rule as judge.jq, lib/plan.mjs scenarioPasses): servedHeadersMatch; the scenarios; no scenario is skipped;
// each scenario has dprOk not false (its page is at the row's DPR); each scenario that has no expectFail is ok with 0 pageErrors, 0 consoleErrors, 0 stuckToasts and 0 failedRequests;
// each scenario with an expectFail (a known defect: '#<Issue>' or 'new') has 0 pageErrors, 0 consoleErrors and 0 failedRequests, and no engine stop (engineStops 0 and not engineStuckRunning)
// unless it declares the stop (expectStop: '#<Issue>' or 'new'); its stuckToasts and ok are only written to the JSON. The row's expectFail lists the marks of its scenarios: [{name, issue}] (a scenario with two marks gives two entries).
// A scenario that carries a mark and passes (ok, and 0 pageErrors, consoleErrors, stuckToasts and failedRequests) is warned about on stderr: remove the mark. Temporary files are in the repository's .tmp-e2e-* and are removed.
// The count of scenarios is that of the set (browser-face: 1).
// Each scenario has: ms (the time from its start to its end), displays (its declared displays), dprOk (the page's DPR is the row's: true, false, or null for the random row),
// engineStop: null, or when the engine stopped (engineStops > 0, or its state is error or exited) the engine's stop: the facts of its own stop report and the report's lines (lib/stop.mjs),
// and failLog: when the scenario fails the acceptance, the last 200 lines of the engine log since its start (no picture).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { DISPLAY, launchBrowser, newPage, startFrameClock, installRafCounter, startRafSampler } from './lib/browser.mjs';
import { startProxy } from './lib/proxy.mjs';
import { Guest } from './lib/guest.mjs';
import { attachDetectors, installToastWatch, toastState, watchEngine } from './lib/detect.mjs';
import { bundleObjects, RAW_CACHE, startDelivery } from './lib/delivery.mjs';
import { probeServed } from './lib/served.mjs';
import { stopOf } from './lib/stop.mjs';
import { AREAS, checkMarks, legacyScenario, marksOf } from './lib/scenario.mjs';
import { areasOfOnly, budgetClock, dprMatches, erroredScenario, needsFailLog, pageGone, parseArgs, parseBudget, parseRepeat, parseShard, planScenarios, rowsFor, runPasses, scenarioResult, tailLines } from './lib/plan.mjs';
import { resetProject, startProject } from './lib/common.mjs';
import { readFeatures } from './coverage.mjs';

export { displayRows, rowPasses } from './lib/plan.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const scenariosDir = path.join(here, 'scenarios');
const featuresDir = path.join(here, 'features');
const BUNDLE = 'fl-Oz';
const bundleDir = path.join(repo, 'lift', 'out-app', 'bundle', 'raw', BUNDLE);
const brDir = path.join(repo, 'lift', 'out-app', 'bundle', 'br', BUNDLE);
const SMOKE_DISPLAYS = '4k-dpr1,4k-dpr2,4k-random';

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const round2 = (v) => Math.round(v * 100) / 100;
const fileTag = (d) => `${d.name}-${String(d.dpr).replace('.', '_').replace(':', '-')}`;

/** the scenarios of one area: the files of scenarios/<area>/ (not index.mjs, not _-prefixed), each a defineScenario; the area of a file must be its folder's */
export async function loadArea(area) {
  const dir = path.join(scenariosDir, area);
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const f of fs.readdirSync(dir).sort()) {
    if (!f.endsWith('.mjs') || f === 'index.mjs' || f.startsWith('_')) continue;
    const mod = await import(pathToFileURL(path.join(dir, f)).href);
    const sc = mod.default;
    if (!sc || sc.kind !== 'scenario') throw new Error(`scenarios/${area}/${f}: default-export defineScenario({...})`);
    if (sc.area !== area) throw new Error(`scenarios/${area}/${f}: the scenario ${sc.id} is of area ${sc.area}`);
    out.push(sc);
  }
  return out;
}

/** an older set (scenarios/<set>/index.mjs): its objects as legacy scenarios of that set; its marks must be its ALLOWED_MARKS (or new) */
export async function loadLegacySet(set) {
  const file = path.join(scenariosDir, set, 'index.mjs');
  if (!/^[a-z][a-z0-9-]*$/.test(set) || !fs.existsSync(file)) throw new Error(`--scenarios ${set}: no set at scenarios/${set}/index.mjs (smoke, browser-face, regression; or --area <領域>)`);
  const mod = await import(pathToFileURL(file).href);
  checkMarks(mod.default, mod.ALLOWED_MARKS ?? null);
  return mod.default.map((s) => legacyScenario(s, set));
}

/** the engine's stop of the page (its log and state), through lib/stop.mjs */
async function engineStopOf(page, cdp, origin, watchedExit = null) {
  const g = new Guest(page, cdp, origin);
  const lines = await g.ev('window.__app ? window.__app.engineLog() : []');
  return stopOf(lines, (await g.state())?.exit ?? watchedExit);
}

/** a scenario with a mark that passes (ok, and every counter 0) is warned about on stderr: its mark is to be removed (a failure is not raised by it) */
function warnIfPassed(out) {
  const clean = out.ok === true && out.error === null && out.pageErrors === 0 && out.consoleErrors === 0 && out.stuckToasts === 0 && out.failedRequests === 0 && out.engineStops === 0;
  if (!clean) return;
  for (const m of marksOf(out.expectFail)) process.stderr.write(`warning: expectFail scenario passed: ${out.name} ${m}. remove the mark\n`);
}

/** the page's DPR is the row's declared one (lib/plan.mjs dprMatches; the random row is not checked). A page at another DPR does not run the scenario: it throws, and the error says why */
async function checkDpr(page, d, out) {
  const got = await page.evaluate(() => window.devicePixelRatio);
  out.dprOk = dprMatches(d.name, got);
  if (out.dprOk === false) throw new Error(`the page's DPR is ${got}, the display ${d.name} declares ${d.dpr}: the scenario is not run`);
}

/** the engine log's tail of a scenario that fails the acceptance (failLog): the last lines since its start; none when it passes or is skipped */
function attachFailLog(out, lines) {
  if (needsFailLog(out)) out.failLog = tailLines(lines ?? []);
}

/** the engine log's position at the start of a scenario in a shared page: the count of lines and the page's load time (a scenario may reload the page) */
const logMark = (guest) => guest.ev('({ n: window.__app ? window.__app.engineLog().length : 0, t: performance.timeOrigin })');

/** the lines the page keeps since the mark: after a reload in the scenario the log is a new one (all of its lines are since the start), so it is read from 0 */
const linesSince = (guest, mark) => guest.ev(`(() => {
  const reloaded = performance.timeOrigin !== ${JSON.stringify(mark.t)};
  const L = window.__app ? window.__app.engineLog() : [];
  return { lines: L.slice(reloaded ? 0 : ${Number(mark.n)}), reloaded };
})()`);

/** one scenario of the older set: a page of its own (the start is the scenario's own) */
async function runLegacy(sc, d, env, budget) {
  const { browser, browserCdp, delivery, shotsDir } = env;
  const out = scenarioResult(sc, 1, d);
  const t0 = Date.now();
  let pg = null, clock = null, sampler = null, watch = null, det = null, guest = null;
  const frames = { n: 0, s: 0 };
  try {
    pg = await newPage(browser, { cssWidth: d.cssWidth, cssHeight: d.cssHeight, dpr: d.dpr, origin: delivery.origin, token: delivery.newToken() });
    det = attachDetectors(pg.page);
    guest = new Guest(pg.page, pg.cdp, delivery.origin);
    guest.shots = shotsDir ? path.join(shotsDir, fileTag(d), sc.name) : null;
    if (guest.shots) fs.mkdirSync(guest.shots, { recursive: true });
    // a scenario that must act before the page's own scripts run (a permission state set for the load; a counter the page's load calls) has beforeLoad
    if (sc.beforeLoad) await sc.beforeLoad({ page: pg.page, browserCdp, browserContextId: pg.browserContextId, origin: delivery.origin });
    await pg.page.goto(`${delivery.origin}/`, { waitUntil: 'load', timeout: 120000 });
    await checkDpr(pg.page, d, out);
    await installToastWatch(pg.page);
    if (env.clock === 'begin') clock = await startFrameClock(pg.cdp);
    await installRafCounter(pg.page);
    sampler = startRafSampler(pg.page);
    watch = watchEngine(guest);
    log(`${sc.name} (${d.name}, DPR ${d.dpr}): started`);
    const r = await sc.run({ guest, page: pg.page, cdp: pg.cdp, browserCdp, browserContextId: pg.browserContextId, origin: delivery.origin, repo, budget, log: (m) => log(`  ${sc.name}: ${m}`) });
    out.ok = r.ok === true;
    out.engineState = r.engineState ?? null;
    out.notes = r.notes ?? [];
    if (r.results) out.results = r.results;
    if (guest.shots) await guest.shot(path.join(guest.shots, 'end.png')).catch(() => {});
  } catch (e) {
    out.error = String(e?.stack ?? e).slice(0, 800);
  } finally {
    if (sampler) { const s = await sampler.stop(); frames.n = s.frames; frames.s = s.seconds; }
    if (clock) { await clock.stop(); const st = clock.stats(); out.beginFrames = st.frames; out.beginFrameErrors = st.errors; }
    out.clock = env.clock;
    const eng = watch ? await watch.stop() : null;
    out.dumpRetries = guest ? guest.dumpRetries : 0;
    const stopped = eng !== null && (eng.stops > 0 || out.engineState === 'error' || out.engineState === 'exited');
    const t = pg ? await toastState(pg.page).catch(() => ({ stuck: 0, ids: [] })) : { stuck: 0, ids: [] };
    out.stuckToasts = t.stuck;
    out.stuckIds = t.ids;
    if (t.info && t.info.length > 0) out.stuckInfo = t.info;
    out.pageErrors = det ? det.pageErrors : 0;
    out.consoleErrors = det ? det.consoleErrors : 0;
    out.failedRequests = det ? det.failedRequests : 0;
    out.details = det ? det.details.slice(0, 12) : [];
    if (eng) {
      out.engineStops = eng.stops;
      out.engineStuckRunning = eng.stuckRunning;
      out.engineRunningMaxMs = eng.runningMs;
      if (eng.stopReason) out.engineStopReason = eng.stopReason.slice(0, 300);
      if (out.engineState === null && eng.last) out.engineState = eng.last.state;
    }
    out.ok = out.ok && out.error === null && (eng ? !eng.stuckRunning && eng.stops === 0 : false);
    out.achievedFps = frames.s > 0 ? round2(frames.n / frames.s) : 0;
    out.rafFrames = frames.n;
    out.rafSeconds = round2(frames.s);
    const lines = pg && (env.logsDir || needsFailLog(out)) ? await new Guest(pg.page, pg.cdp, delivery.origin).ev('window.__app ? window.__app.engineLog() : []').catch(() => []) : null;
    if (env.logsDir && lines) {
      fs.mkdirSync(env.logsDir, { recursive: true });
      fs.writeFileSync(path.join(env.logsDir, `${fileTag(d)}-${sc.name}.log`), `${lines.join('\n')}\n`);
    }
    attachFailLog(out, lines);
    if (stopped && pg) out.engineStop = await engineStopOf(pg.page, pg.cdp, delivery.origin, eng?.exit ?? null).catch((e) => ({ error: String(e?.message ?? e).slice(0, 300), report: [] }));
    if (pg) await pg.context.close().catch(() => {});
    out.ms = Date.now() - t0;
  }
  log(`${sc.name} (${d.name}, DPR ${d.dpr}): ok ${out.ok}, engine ${out.engineState}, errors ${out.pageErrors}/${out.consoleErrors}/${out.failedRequests}, stuck ${out.stuckToasts}, fps ${out.achievedFps}, ${out.ms} ms`);
  warnIfPassed(out);
  return { result: out, frames };
}

/**
 * the page of a shard's row: started once (the delivery's origin, the start of the project); the scenarios run in it. A page whose renderer stopped is made again
 * by runRow (Issue #1528); `crashed` is set by the page's crash event, `recreated` marks a page made again (its File > New is checked by its first scenario).
 */
async function openShared(d, env) {
  const { browser, delivery } = env;
  const pg = await newPage(browser, { cssWidth: d.cssWidth, cssHeight: d.cssHeight, dpr: d.dpr, origin: delivery.origin, token: delivery.newToken() });
  const det = attachDetectors(pg.page);
  const guest = new Guest(pg.page, pg.cdp, delivery.origin);
  const sh = { pg, det, guest, clock: null, sampler: null, watch: null, frames: { n: 0, s: 0 }, start: null, crashed: false, recreated: false };
  pg.page.on('crash', () => { sh.crashed = true; log(`${d.name} (DPR ${d.dpr}): the page's renderer stopped (crash)`); });
  try {
    await pg.page.goto(`${delivery.origin}/`, { waitUntil: 'load', timeout: 120000 });
    await installToastWatch(pg.page);
    if (env.clock === 'begin') sh.clock = await startFrameClock(pg.cdp);
    await installRafCounter(pg.page);
    sh.sampler = startRafSampler(pg.page);
    sh.watch = watchEngine(guest);
    log(`${d.name} (DPR ${d.dpr}): the page started`);
    sh.start = await startProject(guest);
    log(`${d.name} (DPR ${d.dpr}): project ${sh.start.opened ? 'open' : 'not open'} (state ${sh.start.state})`);
    return sh;
  } catch (e) {
    await closeShared(sh);
    throw e;
  }
}

async function closeShared(sh) {
  if (!sh) return;
  if (sh.sampler) { const s = await sh.sampler.stop(); sh.frames = { n: s.frames, s: s.seconds }; }
  if (sh.clock) await sh.clock.stop();
  if (sh.watch) await sh.watch.stop();
  await sh.pg.context.close().catch(() => {});
}

/** one scenario in the shard's page: File > New first (an empty project), then the scenario; the counts are the scenario's own (the differences) */
async function runShared(sc, repeat, d, env, sh, index, budget) {
  const out = scenarioResult(sc, repeat, d);
  const t0 = Date.now();
  // the file area (#1488): an engine that an earlier scenario of the page stopped would stop every later one; the page is loaded again first (the file area only)
  if (sc.area === 'file') {
    const st = await sh.guest.state().catch(() => null);
    if (st && (st.state === 'error' || st.state === 'exited')) {
      log(`${sc.id}: the engine is ${st.state} (an earlier scenario's stop): the page is loaded again`);
      await sh.pg.page.goto(`${env.delivery.origin}/`, { waitUntil: 'load', timeout: 120000 });
      await installToastWatch(sh.pg.page);
      await startProject(sh.guest);
    }
  }
  const before = { pageErrors: sh.det.pageErrors, consoleErrors: sh.det.consoleErrors, failedRequests: sh.det.failedRequests, stops: sh.watch.w.stops, dumps: sh.guest.dumpRetries };
  let mark = { n: 0, t: null };
  try {
    mark = await logMark(sh.guest);
    const reset = await resetProject(sh.guest);
    if (reset.left.length) out.notes.push(`after File > New, open: ${JSON.stringify(reset.left)}`);
    // a page made again (Issue #1528): File > New must make a new project (its form is shown, the engine is idle) before the scenario runs; until it does, each scenario checks it
    if (sh.recreated) {
      const st = (await sh.guest.state().catch(() => null))?.state ?? null;
      if (!reset.form || st !== 'engine-idle') throw new Error(`the page was made again, but File > New did not make a new project (form ${reset.form}, engine ${st}): the scenario is not run`);
      sh.recreated = false;
      out.notes.push('the page was made again: File > New made a new project');
    }
    await checkDpr(sh.pg.page, d, out);
    const r = await sc.run({ guest: sh.guest, page: sh.pg.page, cdp: sh.pg.cdp, browserCdp: env.browserCdp, browserContextId: sh.pg.browserContextId, origin: env.delivery.origin, repo, budget, log: (m) => log(`  ${sc.id}: ${m}`), index });
    out.ok = r.ok === true;
    out.engineState = r.engineState ?? null;
    out.notes.push(...(r.notes ?? []));
    if (r.results) out.results = r.results;
  } catch (e) {
    out.error = String(e?.stack ?? e).slice(0, 800);
  }
  const eng = sh.watch.w;
  out.pageErrors = sh.det.pageErrors - before.pageErrors;
  out.consoleErrors = sh.det.consoleErrors - before.consoleErrors;
  out.failedRequests = sh.det.failedRequests - before.failedRequests;
  out.engineStops = eng.stops - before.stops;
  out.engineStuckRunning = eng.stuckRunning;
  out.engineRunningMaxMs = eng.runningMs;
  if (eng.stopReason && out.engineStops > 0) out.engineStopReason = eng.stopReason.slice(0, 300);
  if (out.engineState === null && eng.last) out.engineState = eng.last.state;
  out.dumpRetries = sh.guest.dumpRetries - before.dumps;
  const t = await toastState(sh.pg.page).catch(() => ({ stuck: 0, ids: [] }));
  out.stuckToasts = t.stuck;
  out.stuckIds = t.ids;
  if (t.info && t.info.length > 0) out.stuckInfo = t.info;
  out.details = sh.det.details.slice(-12);
  const stopped = out.engineStops > 0 || out.engineState === 'error' || out.engineState === 'exited';
  if (stopped) out.engineStop = await engineStopOf(sh.pg.page, sh.pg.cdp, env.delivery.origin, eng.exit).catch((e) => ({ error: String(e?.message ?? e).slice(0, 300), report: [] }));
  out.ok = out.ok && out.error === null && out.engineStops === 0 && !eng.stuckRunning;
  const since = env.logsDir || needsFailLog(out) ? await linesSince(sh.guest, mark).catch(() => null) : null;
  const lines = since?.lines ?? null;
  if (since?.reloaded) out.notes.push('the page was reloaded in the scenario: its engine log is the new one (the lines are from its load)');
  if (env.logsDir && lines) {
    fs.mkdirSync(env.logsDir, { recursive: true });
    fs.writeFileSync(path.join(env.logsDir, `${fileTag(d)}-${sc.id.replace('/', '-')}.log`), `${lines.join('\n')}\n`);
  }
  attachFailLog(out, lines);
  if (env.shotsDir) {
    const dir = path.join(env.shotsDir, fileTag(d), sc.id.replace('/', '-'));
    fs.mkdirSync(dir, { recursive: true });
    await sh.guest.shot(path.join(dir, 'end.png')).catch(() => {});
  }
  out.ms = Date.now() - t0;
  log(`${sc.id} (${d.name}, DPR ${d.dpr}): ok ${out.ok}, engine ${out.engineState}, errors ${out.pageErrors}/${out.consoleErrors}/${out.failedRequests}, stuck ${out.stuckToasts}, ${out.ms} ms`);
  warnIfPassed(out);
  return out;
}

async function runRow(d, items, env, served, budget) {
  const row = {
    display: { width: DISPLAY.width, height: DISPLAY.height, dpr: d.dpr, refreshHzTarget: DISPLAY.refreshHzTarget, achievedFps: 0, cssWidth: d.cssWidth, cssHeight: d.cssHeight },
    name: d.name, delivery: env.delivery ? env.delivery.delivery : null, servedHeadersMatch: served?.servedHeadersMatch === true, accessGate: served?.accessGate ?? null,
    wireContentEncoding: served?.wireContentEncoding ?? null, servedChecks: served?.checks ?? [], scenarios: [], error: null,
    expectFail: items.flatMap(({ sc }) => marksOf(sc.expectFail).map((issue) => ({ name: sc.name, issue }))),
  };
  if (!env.browser || !env.delivery) {
    row.error = env.setupError ?? 'the delivery or the browser did not start';
    return row;
  }
  let frames = 0, seconds = 0;
  let sh = null;
  let index = 0;
  let recreate = false;
  try {
    for (const { sc, repeat } of items) {
      if (!budget.canStart(sc.needsMs)) {
        row.scenarios.push({ ...scenarioResult(sc, repeat, d), skipped: true, ok: false });
        log(`${sc.id}: skipped (the budget of ${d.name}: ${Math.round(budget.elapsedMs())} ms used, ${Math.round(budget.remainingMs())} ms left, the scenario needs ${sc.needsMs} ms)`);
        continue;
      }
      if (sc.kind === 'legacy') {
        const { result, frames: f } = await runLegacy(sc, d, env, budget);
        row.scenarios.push({ ...result, repeat });
        frames += f.n;
        seconds += f.s;
        continue;
      }
      // a page that did not start is an error of this scenario (it did not run); the next scenario tries again (Issue #1528)
      if (sh === null) {
        const t0 = Date.now();
        try {
          sh = await openShared(d, env);
        } catch (e) {
          const err = `the page did not start (${d.name}, DPR ${d.dpr}): ${String(e?.stack ?? e)}`;
          row.scenarios.push(erroredScenario(sc, repeat, d, err, Date.now() - t0));
          log(`${sc.id}: ${String(e?.message ?? e).slice(0, 200)}`);
          continue;
        }
        sh.recreated = recreate;
        recreate = false;
      }
      const res = await runShared(sc, repeat, d, env, sh, index++, budget);
      row.scenarios.push(res);
      // the renderer stopped (Target crashed) or the page does not answer: the page is closed and the next scenario starts a new one (Issue #1528)
      const unanswered = res.error !== null && !(await sh.guest.ev('1').then(() => true, () => false));
      if (unanswered || pageGone({ crashed: sh.crashed, closed: sh.pg.page.isClosed(), error: res.error })) {
        log(`${sc.id}: the page is gone (${res.error ? res.error.split('\n')[0] : 'closed'}): the next scenario starts a new page`);
        await closeShared(sh);
        frames += sh.frames.n;
        seconds += sh.frames.s;
        sh = null;
        recreate = true;
      }
    }
  } catch (e) {
    row.error = `row ${d.name} (DPR ${d.dpr}): ${String(e?.stack ?? e).slice(0, 800)}`;
    log(row.error);
  } finally {
    if (sh) {
      await closeShared(sh);
      frames += sh.frames.n;
      seconds += sh.frames.s;
    }
  }
  row.display.achievedFps = seconds > 0 ? round2(frames / seconds) : 0;
  row.display.rafFrames = frames;
  row.display.rafSeconds = round2(seconds);
  return row;
}

/** the set the run takes: --area (the area's files), else --scenarios (smoke by default) */
async function loadSet(args) {
  if (args.area !== undefined) {
    if (!AREAS.includes(args.area)) throw new Error(`--area ${args.area}: the areas are ${AREAS.join(', ')}`);
    return loadArea(args.area);
  }
  // --only with area IDs (file/open-...) and no --scenarios: the scenarios of those areas (the set of the IDs), not the smoke set
  const areas = args.scenarios === undefined && args.only !== undefined ? areasOfOnly(args.only.split(',').map((s) => s.trim()).filter(Boolean)) : null;
  if (areas !== null) return (await Promise.all(areas.map((a) => loadArea(a)))).flat();
  return loadLegacySet(args.scenarios ?? 'smoke');
}

/** the --list lines: one per area (or the one area given) */
async function listLines(args) {
  const features = readFeatures(featuresDir);
  const areas = args.area !== undefined ? [args.area] : AREAS;
  const lines = [];
  for (const area of areas) {
    const scs = await loadArea(area);
    const covers = new Set(scs.flatMap((s) => s.covers));
    lines.push({ area, scenarios: scs.length, covers: covers.size, features: features.filter((f) => f.area === area).length });
  }
  return lines;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.list) {
    for (const line of await listLines(args)) process.stdout.write(`${JSON.stringify(line)}\n`);
    return 0;
  }
  const all = await loadSet(args);
  const only = args.only === undefined ? null : args.only.split(',').map((s) => s.trim()).filter(Boolean);
  const shard = args.shard === undefined ? null : parseShard(args.shard);
  const repeat = args.repeat === undefined ? 1 : parseRepeat(args.repeat);
  const budgetMs = args.budget === undefined ? null : parseBudget(args.budget);
  const picked = planScenarios(all, { only, shard, repeat });
  if (picked.length === 0) throw new Error('no scenario is selected (check --area, --only, --shard)');
  const names = args.displays !== undefined ? args.displays.split(',').map((s) => s.trim()).filter(Boolean) : [...new Set(picked.flatMap((p) => p.sc.displays))];
  const plan = names.flatMap((name) => rowsFor(name, picked).filter((r) => r.items.length > 0));
  if (plan.length === 0) throw new Error(`no selected scenario declares the displays ${names.join(', ')}`);
  const shotsDir = args.shots ? path.resolve(args.shots) : null;
  const logsDir = args['engine-logs'] ? path.resolve(args['engine-logs']) : null;
  if (!fs.existsSync(bundleDir)) throw new Error(`no bundle at ${bundleDir} (fe-build bundle)`);
  const workDir = fs.mkdtempSync(path.join(repo, '.tmp-e2e-'));
  const wranglerDirs = [path.join(appDir, '.wrangler'), path.join(repo, '.wrangler')];
  const hadWrangler = wranglerDirs.map((d) => fs.existsSync(d));
  const env = { shotsDir, logsDir, clock: args.clock ?? 'raf', browser: null, browserCdp: null, delivery: null, served: null, setupError: null };
  const rows = [];
  const expected = [];
  try {
    const objects = bundleObjects(bundleDir, brDir, BUNDLE);
    const brFile = objects.find((o) => o.key.startsWith(`${BUNDLE}/`) && o.httpMetadata.contentEncoding === 'br').key.slice(BUNDLE.length + 1);
    try {
      env.delivery = await startDelivery({ appDir, workDir, objects, log });
      env.served = await probeServed({ origin: env.delivery.upstream, token: env.delivery.newToken(), bundle: BUNDLE, brFile, objects, accessEnv: env.delivery.accessEnv, manifestCacheStored: RAW_CACHE });
      log('served headers', { match: env.served.servedHeadersMatch, accessGate: env.served.accessGate, wireContentEncoding: env.served.wireContentEncoding });
    } catch (e) {
      env.setupError = `delivery: ${String(e?.stack ?? e).slice(0, 1500)}`;
      log(env.setupError);
    }
    if (env.delivery) {
      env.proxy = await startProxy({ origin: env.delivery.origin, token: env.delivery.newToken() });
      env.browser = await launchBrowser(env.clock, env.proxy.port);
      env.browserCdp = await env.browser.newBrowserCDPSession();
    }
    for (const { d, items } of plan) {
      expected.push(items.length);
      const row = await runRow(d, items, env, env.served, budgetClock(budgetMs));
      rows.push(row);
      process.stdout.write(`${JSON.stringify(row)}\n`);
    }
  } catch (e) {
    log(`run: ${String(e?.stack ?? e)}`);
  } finally {
    if (env.browser) await env.browser.close().catch(() => {});
    if (env.proxy) await env.proxy.close().catch(() => {});
    if (env.delivery) await env.delivery.close().catch(() => {});
    fs.rmSync(workDir, { recursive: true, force: true });
    wranglerDirs.forEach((d, i) => { if (!hadWrangler[i]) fs.rmSync(d, { recursive: true, force: true }); });
  }
  const accepted = runPasses(rows, expected);
  log(`rows ${rows.length}, accepted ${accepted}`);
  if (args['report-only'] !== true) return accepted ? 0 : 1;
  const reported = runPasses(rows, expected, { reportOnly: true });
  if (reported) log('report-only: exit 0');
  return reported ? 0 : 1;
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  process.exitCode = await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    return 1;
  });
}
