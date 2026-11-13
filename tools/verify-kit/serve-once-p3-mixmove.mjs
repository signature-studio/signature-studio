// Runs the P3 song scenario with the Mixer shown and the mouse moved during playback (scn-p3-mixmove-300.json: 5 minutes, 10 moves per second in the measured window) once with sdriveg2-lat.mjs against the E13 bundle. Same serving as serve-once-p3.mjs. Output: out/p3-mixmove-chrome-<N>. No arguments.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { freePort } from './lib/cdp.mjs';

const BUNDLE = 'fl-engine-ci-0363b050';
const kit = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(kit, '..', '..');
const appDir = path.join(repo, 'app');
const bundleDir = path.join(repo, 'artifacts', 'bundles', BUNDLE);
const zip = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
const scenarios = path.join(kit, 'tools', 'scn-p3-mixmove-300.json');
// each run gets its own folder: p3-mixmove-chrome-<largest existing number + 1>
const nextOut = () => {
  const dir = path.join(kit, 'out');
  const ns = fs.existsSync(dir) ? fs.readdirSync(dir).map((d) => /^p3-mixmove-chrome-(\d+)$/.exec(d)).filter(Boolean).map((m) => Number(m[1])) : [];
  return path.join(dir, `p3-mixmove-chrome-${ns.length ? Math.max(...ns) + 1 : 1}`);
};
const outDir = nextOut();
const tail = (s, n) => s.split('\n').slice(-n).join('\n');

async function main() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'serve-once-p3m-'));
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
