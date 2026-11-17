// R5-5: a copy of serve-once-r54-bundle.mjs with a third argument that picks the scenario: problems (tools/scn-r55-problems.json: the reference project opened as a zip; the picture of the Problems window is read) or play (tools/scn-r55-play.json: the same, then 10 s of playback). Usage: node tools/verify-kit/serve-once-r55-bundle.mjs <absolute path to the bundle directory> <label: [a-z0-9]+> <problems|play>. Output: out/r55-<label>-<scenario>-chrome-<N>.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { freePort } from './lib/cdp.mjs';

const usage = (msg) => {
  process.stderr.write(`serve-once-r55-bundle: ${msg}\nusage: node tools/verify-kit/serve-once-r55-bundle.mjs <absolute path to the bundle directory> <label: [a-z0-9]+> <problems|play>\n`);
  process.exit(2);
};
const [bundleArg, label, scen] = process.argv.slice(2);
if (!bundleArg || !label || !scen) usage('three arguments are needed');
if (!/^[a-z0-9]+$/.test(label)) usage('the label must match [a-z0-9]+');
if (!['problems', 'play'].includes(scen)) usage('the scenario must be problems or play');
if (!path.isAbsolute(bundleArg)) usage('the bundle directory must be an absolute path');
const bundleDir = path.normalize(bundleArg);
if (!fs.existsSync(path.join(bundleDir, 'manifest.json'))) usage('no manifest.json in the bundle directory');
const BUNDLE = path.basename(bundleDir);
const kit = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(kit, '..', '..');
const appDir = path.join(repo, 'app');
const zip = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
const scenarios = path.join(kit, 'tools', `scn-r55-${scen}.json`);
// each run gets its own folder: r55-<label>-<scenario>-chrome-<largest existing number + 1>
const nextOut = () => {
  const dir = path.join(kit, 'out');
  const re = new RegExp(`^r55-${label}-${scen}-chrome-(\\d+)$`);
  const ns = fs.existsSync(dir) ? fs.readdirSync(dir).map((d) => re.exec(d)).filter(Boolean).map((m) => Number(m[1])) : [];
  return path.join(dir, `r55-${label}-${scen}-chrome-${ns.length ? Math.max(...ns) + 1 : 1}`);
};
const outDir = nextOut();
const tail = (s, n) => s.split('\n').slice(-n).join('\n');

async function main() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'serve-once-p3b-'));
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
    const child = spawn(process.execPath, [path.join(kit, 'tools', 'sdriveg2-export.mjs'), '--browser', 'chrome', '--url', `http://127.0.0.1:${port}/`, '--out', outDir, '--app', appDir, '--scenarios', scenarios, '--viewport', '2260x1116'], { stdio: ['ignore', 'pipe', 'pipe'] });
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
