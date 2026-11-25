// feat-c1-hidpi/scale-grid: FL's scale (the factor of its start-up line "Main GUI scaling factor: F (system set to N PPI)") and the DPI it sees, at each device pixel ratio (Issue #1348).
// Boots the bundle in headless Chrome once for each ratio (a new profile each time, the launch of measure-size.mjs), waits for the first engine-idle and then for FL's line, and prints
// ONE JSON line per ratio on stdout: {"dpr","dpi","scaleFactor","systemPpi","state","standIn","error","elapsedMs"}.
//  - dpi: LOGPIXELSY of the start (the guest's answer). FL should see round(96 x dpr).
//  - scaleFactor: F of the line (1.25 is 125 %). systemPpi: N of the line. Nothing is compared here: the acceptance reads the rows against fl-scale-steps.json.
//  - error: set when the boot failed or FL printed no such line within 30 s after the first idle. A row with an error is data.
// The default ratios: 1.00 to 3.00 by 0.05 (41 ratios, 2.00 among them). --dprs <list>: these ratios instead (the refinement: 0.01 steps between two ratios whose scales differ).
// Exit 2 on a usage error, 1 when the run itself failed (the bundle, the server), 0 otherwise.
// Usage: node tools/feat-c1-hidpi/scale-grid.mjs <absolute path to the bundle directory raw/<name>> [--dprs <list>] > grid.jsonl
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { bootToIdle, debugLogOf, ISOLATION, openPage, startFacts } from './measure-size.mjs';

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const VIEWPORT = { w: 1836, h: 1056 };
const FACTOR_MARKERS = ['Main GUI scaling factor'];
const FACTOR_WAIT_MS = 30000;
const POLL_MS = 1000;

/** the default ratios: 1.00 to 3.00 by 0.05 (41 values; integers over 100, so each is the decimal the reader expects) */
export function gridRatios() {
  return Array.from({ length: 41 }, (_, i) => (100 + 5 * i) / 100);
}

export function parseArgs(argv) {
  const opts = { bundleDir: '', dprs: gridRatios() };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dprs') {
      const v = String(argv[++i] ?? '').split(',').map((x) => x.trim()).filter((x) => x !== '').map(Number);
      if (v.length === 0 || v.some((x) => !Number.isFinite(x) || x <= 0)) throw new Error(`--dprs: not a list of positive numbers: ${argv[i]}`);
      opts.dprs = v;
    } else if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
    else if (opts.bundleDir === '') opts.bundleDir = path.resolve(a);
    else throw new Error(`one bundle directory: ${a}`);
  }
  return opts;
}

/** one ratio: a fresh boot, the first engine-idle, then FL's line (read from the guest's memory, polled for 30 s) */
async function gridRow(dpr, ctx) {
  const row = { dpr, dpi: null, scaleFactor: null, systemPpi: null, state: null, standIn: null, error: null, elapsedMs: null };
  const t0 = Date.now();
  const profile = path.join(ctx.root, `profile-${++ctx.runs}`);
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    const p = await openPage(chrome, dpr, VIEWPORT);
    cdp = p.cdp;
    const st = { state: null };
    await bootToIdle(p.S, p.ev, ctx.port, st);
    row.state = st.state;
    const boot = {};
    const idleScan = await startFacts(p.ev, boot);
    row.dpi = boot.dpi;
    row.standIn = await p.ev(`document.body ? document.body.innerText.includes('STAND-IN') : null`);
    const scan = (what) => p.ev(`window.__app.guestText(${JSON.stringify(what)}, 0)`).catch((e) => ({ error: String(e?.message ?? e).slice(0, 200) }));
    let scans = [['idle', idleScan]];
    let dl = debugLogOf(scans, 'no start-up line of FL');
    for (let i = 0; !dl.scalingFactor && i < FACTOR_WAIT_MS / POLL_MS; i++) {
      await sleep(POLL_MS);
      scans = [['factor', await scan(FACTOR_MARKERS)]];
      dl = debugLogOf(scans, 'no start-up line of FL');
    }
    if (dl.scalingFactor) {
      row.scaleFactor = dl.scalingFactor.factor;
      row.systemPpi = dl.scalingFactor.systemPpi;
    } else row.error = 'FL printed no "Main GUI scaling factor" line within 30 s after the first engine-idle';
  } catch (e) {
    row.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    row.elapsedMs = Date.now() - t0;
  }
  return row;
}

async function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`scale-grid: ${e.message}\n`);
    process.exitCode = 2;
    return;
  }
  if (!opts.bundleDir) {
    process.stderr.write('usage: node tools/feat-c1-hidpi/scale-grid.mjs <absolute path to the bundle directory> [--dprs <list>]\n');
    process.exitCode = 2;
    return;
  }
  const bundleDir = opts.bundleDir;
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-c1-grid-'));
  const assets = path.join(root, 'assets');
  let server = null, failed = false;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const ctx = { port, root, runs: 0 };
    for (const dpr of opts.dprs) {
      const row = await gridRow(dpr, ctx);
      process.stdout.write(JSON.stringify(row) + '\n');
      process.stderr.write(`grid ${dpr}: dpi ${row.dpi} factor ${row.scaleFactor} state ${row.state}${row.error ? ` ERR(${row.error.slice(0, 160)})` : ''}\n`);
    }
  } catch (e) {
    failed = true;
    process.stderr.write(`scale-grid: ${String(e?.message ?? e).slice(0, 500)}\n`);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.exitCode = failed ? 1 : 0;
}

if (isMain) await main();
