#!/usr/bin/env node
// E2E (Issue #1407): drives the production delivery (wrangler dev --local of app/worker/index.ts, with the bundle in its local R2) in Chromium at 3840 x 2160
// physical pixels, 120 Hz by the BeginFrame clock (lib/browser.mjs), with real mouse and key events (lib/guest.mjs).
// Usage: node tools/e2e/run.mjs [--scenarios smoke] [--only NAME[,NAME]] [--displays 4k-dpr1,4k-dpr2,4k-random] [--clock raf|begin] [--shots DIR]     (run.sh does the same from the repository)
//   --clock: raf (default: the page's own rAF, the frame-rate limit off) or begin (120 Hz by the BeginFrame clock; see lib/browser.mjs).
//   --displays: 4k-dpr1 (3840 x 2160 CSS px, DPR 1); 4k-dpr2 (1920 x 1080 CSS px, DPR 2); 4k-random (two DPRs drawn at random from 1.01 to 3.00: CSS px 3840 / DPR, 2160 / DPR).
//   --only: a test aid, only the named scenarios of the set are run (e.g. --only file-open-demo); each row then has that many scenarios.
//   --shots DIR: a test aid, PNG pictures of the page after each scenario are written to DIR (the directory is made; nothing is removed).
//   --engine-logs DIR: a test aid, the engine log of each scenario (every line the page keeps) is written to DIR/<display>-<dpr>-<scenario>.log (the directory is made).
// stdout: one JSON line per display row (the JSON is written also when the run failed: its error fields say what). stderr: the log.
// Exit 0 only when every row meets the acceptance (rowPasses, the same rule as judge.jq): servedHeadersMatch; the scenarios; each scenario that has no expectFail is ok with 0 pageErrors,
// 0 consoleErrors, 0 stuckToasts and 0 failedRequests; each scenario with an expectFail (a known defect, named by its issue) has 0 pageErrors, 0 consoleErrors and 0 failedRequests
// (its stuckToasts is only written to the JSON). Temporary files are in the repository's .tmp-e2e-* and are removed.
// Each scenario has engineStop: null, or when the engine stopped (engineStops > 0, or its state is error or exited) the engine's stop: the facts of its own
// stop report and the report's lines (lib/stop.mjs).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { DISPLAY, launchBrowser, newPage, startFrameClock, installRafCounter, startRafSampler } from './lib/browser.mjs';
import { Guest } from './lib/guest.mjs';
import { attachDetectors, installToastWatch, toastState, watchEngine } from './lib/detect.mjs';
import { bundleObjects, RAW_CACHE, startDelivery } from './lib/delivery.mjs';
import { probeServed } from './lib/served.mjs';
import { stopOf } from './lib/stop.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const BUNDLE = 'fl-Oz';
const bundleDir = path.join(repo, 'lift', 'out-app', 'bundle', 'raw', BUNDLE);
const brDir = path.join(repo, 'lift', 'out-app', 'bundle', 'br', BUNDLE);

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const round2 = (v) => Math.round(v * 100) / 100;

function parseArgs(argv) {
  const a = {};
  for (let i = 0; i < argv.length; i++) {
    const t = argv[i];
    if (!t.startsWith('--')) throw new Error(`unexpected argument ${t}`);
    const v = argv[++i];
    if (v === undefined) throw new Error(`--${t.slice(2)} needs a value`);
    a[t.slice(2)] = v;
  }
  return a;
}

/** the rows of one display name: the CSS size of the 3840 x 2160 physical picture at the DPR */
export function displayRows(name, rnd = Math.random) {
  const row = (n, dpr) => ({ name: n, dpr, cssWidth: Math.round(DISPLAY.width / dpr), cssHeight: Math.round(DISPLAY.height / dpr) });
  if (name === '4k-dpr1') return [row(name, 1)];
  if (name === '4k-dpr2') return [row(name, 2)];
  if (name === '4k-random') {
    return [0, 1].map(() => row(name, Math.round((1.01 + rnd() * 1.99) * 100) / 100));
  }
  throw new Error(`no display ${name} (4k-dpr1, 4k-dpr2, 4k-random)`);
}

/** the acceptance of one row of `n` scenarios (the same rule as judge.jq, which the job runs on the output) */
export function rowPasses(row, n = 3) {
  return row.servedHeadersMatch === true && row.error === null && row.scenarios.length === n
    && row.scenarios.every((s) => (s.expectFail !== null
      ? s.pageErrors === 0 && s.consoleErrors === 0 && s.failedRequests === 0
      : s.ok === true && s.pageErrors === 0 && s.consoleErrors === 0 && s.stuckToasts === 0 && s.failedRequests === 0));
}

/** the engine's log of the page and its stop fact, read when the engine has stopped (lib/stop.mjs) */
async function engineStopOf(page, cdp, origin) {
  const g = new Guest(page, cdp, origin);
  const lines = await g.ev("window.__app ? window.__app.engineLog() : []");
  return stopOf(lines, (await g.state())?.exit ?? null);
}

async function runScenario(sc, d, env) {
  const { browser, browserCdp, delivery, shotsDir } = env;
  const out = {
    name: sc.name, ok: false, expectFail: sc.expectFail ?? null, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0,
    engineState: null, engineStops: 0, engineStuckRunning: false, engineRunningMaxMs: 0, engineStop: null, dumpRetries: 0, beginFrames: 0, beginFrameErrors: 0, error: null, notes: [], details: [],
  };
  let pg = null, clock = null, sampler = null, watch = null, det = null, guest = null;
  const frames = { n: 0, s: 0 };
  try {
    pg = await newPage(browser, { cssWidth: d.cssWidth, cssHeight: d.cssHeight, dpr: d.dpr, origin: delivery.origin, token: delivery.newToken() });
    det = attachDetectors(pg.page);
    guest = new Guest(pg.page, pg.cdp, delivery.origin);
    guest.shots = shotsDir ? path.join(shotsDir, `${d.name}-${String(d.dpr).replace('.', '_')}`, sc.name) : null;
    if (guest.shots) fs.mkdirSync(guest.shots, { recursive: true });
    await pg.page.goto(`${delivery.origin}/`, { waitUntil: 'load', timeout: 120000 });
    await installToastWatch(pg.page);
    if (env.clock === 'begin') clock = await startFrameClock(pg.cdp);
    await installRafCounter(pg.page);
    sampler = startRafSampler(pg.page);
    watch = watchEngine(guest);
    log(`${sc.name} (${d.name}, DPR ${d.dpr}): started`);
    const r = await sc.run({ guest, page: pg.page, cdp: pg.cdp, browserCdp, browserContextId: pg.browserContextId, origin: delivery.origin, repo, log: (m) => log(`  ${sc.name}: ${m}`) });
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
    if (env.logsDir && pg) {
      const lines = await new Guest(pg.page, pg.cdp, delivery.origin).ev('window.__app ? window.__app.engineLog() : []').catch(() => []);
      fs.mkdirSync(env.logsDir, { recursive: true });
      fs.writeFileSync(path.join(env.logsDir, `${d.name}-${String(d.dpr).replace('.', '_')}-${sc.name}.log`), `${lines.join('\n')}\n`);
    }
    if (stopped && pg) out.engineStop = await engineStopOf(pg.page, pg.cdp, delivery.origin).catch((e) => ({ error: String(e?.message ?? e).slice(0, 300), report: [] }));
    const t = pg ? await toastState(pg.page).catch(() => ({ stuck: 0, ids: [] })) : { stuck: 0, ids: [] };
    out.stuckToasts = t.stuck;
    out.stuckIds = t.ids;
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
    if (pg) await pg.context.close().catch(() => {});
  }
  log(`${sc.name} (${d.name}, DPR ${d.dpr}): ok ${out.ok}, engine ${out.engineState}, errors ${out.pageErrors}/${out.consoleErrors}/${out.failedRequests}, stuck ${out.stuckToasts}, fps ${out.achievedFps}`);
  return { result: out, frames };
}

async function runRow(d, env, served) {
  const row = {
    display: { width: DISPLAY.width, height: DISPLAY.height, dpr: d.dpr, refreshHzTarget: DISPLAY.refreshHzTarget, achievedFps: 0, cssWidth: d.cssWidth, cssHeight: d.cssHeight },
    name: d.name, delivery: env.delivery ? env.delivery.delivery : null, servedHeadersMatch: served?.servedHeadersMatch === true, accessGate: served?.accessGate ?? null,
    wireContentEncoding: served?.wireContentEncoding ?? null, servedChecks: served?.checks ?? [], scenarios: [], error: null,
  };
  if (!env.browser || !env.delivery) {
    row.error = env.setupError ?? 'the delivery or the browser did not start';
    return row;
  }
  let frames = 0, seconds = 0;
  for (const sc of env.scenarios) {
    const { result, frames: f } = await runScenario(sc, d, env);
    row.scenarios.push(result);
    frames += f.n;
    seconds += f.s;
  }
  row.display.achievedFps = seconds > 0 ? round2(frames / seconds) : 0;
  row.display.rafFrames = frames;
  row.display.rafSeconds = round2(seconds);
  return row;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const set = args.scenarios ?? 'smoke';
  const all = (await import(`./scenarios/${set}/index.mjs`)).default;
  const only = args.only === undefined ? null : args.only.split(',').map((s) => s.trim()).filter(Boolean);
  if (only !== null && (only.length === 0 || only.some((n) => !all.some((s) => s.name === n)))) {
    throw new Error(`--only: ${args.only} is not a scenario of the set ${set} (${all.map((s) => s.name).join(', ')})`);
  }
  const scenarios = only === null ? all : all.filter((s) => only.includes(s.name));
  const names = (args.displays ?? '4k-dpr1,4k-dpr2,4k-random').split(',').map((s) => s.trim()).filter(Boolean);
  const plan = names.flatMap((n) => displayRows(n).map((d) => ({ ...d, display: n })));
  const shotsDir = args.shots ? path.resolve(args.shots) : null;
  const logsDir = args["engine-logs"] ? path.resolve(args["engine-logs"]) : null;
  if (!fs.existsSync(bundleDir)) throw new Error(`no bundle at ${bundleDir} (fe-build bundle)`);
  const workDir = fs.mkdtempSync(path.join(repo, '.tmp-e2e-'));
  const wranglerDir = path.join(appDir, '.wrangler');
  const hadWrangler = fs.existsSync(wranglerDir);
  const env = { scenarios, shotsDir, logsDir, clock: args.clock ?? 'raf', browser: null, browserCdp: null, delivery: null, served: null, setupError: null };
  const rows = [];
  try {
    const objects = bundleObjects(bundleDir, brDir, BUNDLE);
    const brFile = objects.find((o) => o.key.startsWith(`${BUNDLE}/`) && o.httpMetadata.contentEncoding === 'br').key.slice(BUNDLE.length + 1);
    try {
      env.delivery = await startDelivery({ appDir, workDir, objects, log });
      env.served = await probeServed({ origin: env.delivery.origin, token: env.delivery.newToken(), bundle: BUNDLE, brFile, objects, accessEnv: env.delivery.accessEnv, manifestCacheStored: RAW_CACHE });
      log('served headers', { match: env.served.servedHeadersMatch, accessGate: env.served.accessGate, wireContentEncoding: env.served.wireContentEncoding });
    } catch (e) {
      env.setupError = `delivery: ${String(e?.stack ?? e).slice(0, 1500)}`;
      log(env.setupError);
    }
    if (env.delivery) {
      env.browser = await launchBrowser(env.clock);
      env.browserCdp = await env.browser.newBrowserCDPSession();
    }
    for (const d of plan) {
      const row = await runRow(d, env, env.served);
      rows.push(row);
      process.stdout.write(JSON.stringify(row) + '\n');
    }
  } catch (e) {
    log(`run: ${String(e?.stack ?? e)}`);
  } finally {
    if (env.browser) await env.browser.close().catch(() => {});
    if (env.delivery) await env.delivery.close().catch(() => {});
    fs.rmSync(workDir, { recursive: true, force: true });
    if (!hadWrangler) fs.rmSync(wranglerDir, { recursive: true, force: true });
  }
  const ok = rows.length === plan.length && rows.every((r) => rowPasses(r, scenarios.length));
  log(`rows ${rows.length}, accepted ${ok}`);
  return ok ? 0 : 1;
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  process.exitCode = await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    return 1;
  });
}
