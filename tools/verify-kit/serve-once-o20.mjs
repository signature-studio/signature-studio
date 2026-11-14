// Runs the R2-4c-1 start scenario (tools/verify-kit/tools/scn-o20-start.json: close the wizard, wait 800 ms, shot `o20-start`, describe the windows) once with sdriveg2-lat.mjs, on the bare product URL (no query, no flags). Same serving as serve-once-o9.mjs. One positional argument: the bundle, `work-r24` (~/.cache/signature-studio/work-r24/bundle/fl-Oz, the bundle after the R2-4 intake) or `base` (~/.cache/signature-studio/work/bundle/fl-Oz, the bundle before it). Output: tools/verify-kit/out/o20-<work-r24|base>-<N>/ (gitignored: o20-start.png, report.json, report.md).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { freePort } from './lib/cdp.mjs';

const which = process.argv[2];
const kit = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(kit, '..', '..');
const bundleDirs = { 'work-r24': path.join(os.homedir(), '.cache', 'signature-studio', 'work-r24', 'bundle', 'fl-Oz'), 'work-r25': path.join(os.homedir(), '.cache', 'signature-studio', 'work-r25', 'bundle', 'fl-Oz'), base: path.join(os.homedir(), '.cache', 'signature-studio', 'work', 'bundle', 'fl-Oz') };
if (!bundleDirs[which]) { console.error('usage: node serve-once-o20.mjs <work-r24|work-r25|base> [tempo|rclick|esc|rc1..rc6]'); process.exit(2); }
const bundleDir = bundleDirs[which];
if (!fs.existsSync(path.join(bundleDir, 'manifest.json'))) { console.error('no manifest.json in ' + bundleDir); process.exit(2); }
const manifest = JSON.parse(fs.readFileSync(path.join(bundleDir, 'manifest.json'), 'utf8'));
const BUNDLE = /^\/assets\/([^/]+)\//.exec(manifest.engine.wasm.url)?.[1];
if (!BUNDLE) { console.error('manifest.json: no /assets/<name>/ prefix in engine.wasm.url'); process.exit(2); }
const appDir = path.join(repo, 'app');
const zip = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
const scenarios = path.join(kit, 'tools', process.argv[3] === 'tempo' ? 'scn-o20-tempo.json' : process.argv[3] === 'rclick' ? 'scn-o20-rclick.json' : process.argv[3] === 'esc' ? 'scn-o20-esc.json' : /^rc[1-6]$/.test(process.argv[3] ?? '') ? `scn-o20-${process.argv[3]}.json` : 'scn-o20-start.json');
// each run gets its own folder: o20-<work-r24|base>-<largest existing number + 1>
const nextOut = () => {
  const dir = path.join(kit, 'out');
  const re = new RegExp(`^o20-${which}-(\\d+)$`);
  const ns = fs.existsSync(dir) ? fs.readdirSync(dir).map((d) => re.exec(d)).filter(Boolean).map((m) => Number(m[1])) : [];
  return path.join(dir, `o20-${which}-${ns.length ? Math.max(...ns) + 1 : 1}`);
};
const outDir = nextOut();
const tail = (s, n) => s.split('\n').slice(-n).join('\n');

async function main() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'serve-once-o20-'));
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
    process.stdout.write(`rc=${rc}\nout=${outDir}\n--- stdout (tail)\n${tail(out, 30)}\n--- stderr (tail)\n${tail(err, 30)}\n`);
    process.exitCode = rc ?? 1;
  } finally {
    if (server) await server.close().catch(() => {});
    try { fs.unlinkSync(path.join(assets, BUNDLE)); fs.unlinkSync(path.join(assets, 'proj', 'serina1-serum2.zip')); } catch { /* not made */ }
    fs.rmSync(root, { recursive: true, force: true });
  }
}

await main();
