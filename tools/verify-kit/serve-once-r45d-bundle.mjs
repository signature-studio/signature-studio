// R4-5d: a copy of serve-once-r635-bundle.mjs whose page is the app folder given as the fourth argument (an app with a built dist/ of the same tree the bundle was made from: the page's worker start depends on the glue of that tree; R4-5c served each tree with its own app). The scenarios are those of serve-once-r635-bundle.mjs plus latlog (tools/scn-r45d-lat-log.json). Usage: node tools/verify-kit/serve-once-r45d-bundle.mjs <absolute bundle directory> <label: [a-z0-9]+> <lat|latlog|song|front|mixmove|probe> <absolute app directory with dist/>. Output: out/r45d-<scenario>-<label>-chrome-<N>. The Chrome flags come from the environment as in lib/browser.mjs; this script adds none.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { freePort } from './lib/cdp.mjs';

const SCENARIOS = { lat: 'scn-lat-ff.json', latlog: 'scn-r45d-lat-log.json', song: 'scn-s3a-song.json', front: 'scn-r635-front-song.json', mixmove: 'scn-p3-mixmove-300.json', probe: 'scn-r635-probe-f9.json' };
const usage = (msg) => {
  process.stderr.write(`serve-once-r45d-bundle: ${msg}\nusage: node tools/verify-kit/serve-once-r45d-bundle.mjs <absolute bundle directory> <label: [a-z0-9]+> <${Object.keys(SCENARIOS).join('|')}> <absolute app directory with dist/>\n`);
  process.exit(2);
};
const [bundleArg, label, scen, appArg] = process.argv.slice(2);
if (!bundleArg || !label || !scen || !appArg) usage('four arguments are needed');
if (!/^[a-z0-9]+$/.test(label)) usage('the label must match [a-z0-9]+');
if (!Object.hasOwn(SCENARIOS, scen)) usage('the scenario must be one of the listed names');
if (!path.isAbsolute(bundleArg)) usage('the bundle directory must be an absolute path');
if (!path.isAbsolute(appArg)) usage('the app directory must be an absolute path');
if (!fs.existsSync(path.join(appArg, 'dist', 'index.html'))) usage('no dist/index.html in the app directory');
const bundleDir = path.normalize(bundleArg);
if (!fs.existsSync(path.join(bundleDir, 'manifest.json'))) usage('no manifest.json in the bundle directory');
const BUNDLE = path.basename(bundleDir);
const kit = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.normalize(appArg);
const zip = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
const scenarios = path.join(kit, 'tools', SCENARIOS[scen]);
// each run gets its own folder: r45d-<scenario>-<label>-chrome-<largest existing number + 1>
const nextOut = () => {
  const dir = path.join(kit, 'out');
  const re = new RegExp(`^r45d-${scen}-${label}-chrome-(\\d+)$`);
  const ns = fs.existsSync(dir) ? fs.readdirSync(dir).map((d) => re.exec(d)).filter(Boolean).map((m) => Number(m[1])) : [];
  return path.join(dir, `r45d-${scen}-${label}-chrome-${ns.length ? Math.max(...ns) + 1 : 1}`);
};
const outDir = nextOut();
const tail = (s, n) => s.split('\n').slice(-n).join('\n');

async function main() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'serve-once-r45d-'));
  const assets = path.join(root, 'assets');
  let server = null;
  try {
    fs.mkdirSync(path.join(assets, 'proj'), { recursive: true });
    fs.symlinkSync(bundleDir, path.join(assets, BUNDLE));
    fs.copyFileSync(path.join(bundleDir, 'manifest.json'), path.join(assets, 'manifest.json'));
    fs.symlinkSync(zip, path.join(assets, 'proj', 'serina1-serum2.zip'));
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const child = spawn(process.execPath, [path.join(kit, 'tools', 'sdriveg2-lat.mjs'), '--browser', 'chrome', '--url', `http://127.0.0.1:${port}/`, '--out', outDir, '--app', appDir, '--scenarios', scenarios, '--viewport', '2260x1116'], { stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '', err = '';
    child.stdout.on('data', (d) => { out += d; });
    child.stderr.on('data', (d) => { err += d; });
    const rc = await new Promise((resolve) => child.on('close', resolve));
    process.stdout.write(`rc=${rc} out=${outDir}\n--- stdout (tail)\n${tail(out, 30)}\n--- stderr (tail)\n${tail(err, 30)}\n`);
    process.exitCode = rc ?? 1;
  } finally {
    if (server) await server.close().catch(() => {});
    try { fs.unlinkSync(path.join(assets, BUNDLE)); fs.unlinkSync(path.join(assets, 'proj', 'serina1-serum2.zip')); } catch { /* not made */ }
    fs.rmSync(root, { recursive: true, force: true });
  }
}

await main();
