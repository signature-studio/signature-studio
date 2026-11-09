// A minimal WebDriver (classic, over HTTP) client for geckodriver, so that the file manager scenario also runs in Firefox (decision 0216, opfs1). Headless Firefox, a fresh profile (geckodriver makes one),
// downloads into a directory without a prompt. The snap geckodriver/firefox can read non-hidden files under $HOME but not /tmp, so the test data lives under the work directory.
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { freePort, sleep } from '../helpers/proc.mjs';

export async function openFirefox({ work, downloadDir, dataDir, extraPrefs = {}, extraArgs = [], scriptTimeoutMs = 0 }) {
  const port = await freePort();
  fs.mkdirSync(downloadDir, { recursive: true });
  const logFile = path.join(work, 'geckodriver.log');
  const gd = spawn(process.env.GECKODRIVER || '/snap/bin/geckodriver', ['--port', String(port), '--host', '127.0.0.1'], { stdio: ['ignore', fs.openSync(logFile, 'w'), fs.openSync(logFile + '.err', 'w')] });
  const base = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 100; i++) { try { const r = await fetch(base + '/status'); if (r.ok) break; } catch { /* not yet */ } await sleep(100); }
  const call = async (method, p, body) => {
    const r = await fetch(base + p, { method, headers: { 'content-type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
    const j = await r.json();
    if (j.value && j.value.error) throw new Error(`${j.value.error}: ${j.value.message}`.slice(0, 600));
    return j.value;
  };
  const caps = {
    capabilities: { alwaysMatch: {
      browserName: 'firefox', acceptInsecureCerts: true,
      'moz:firefoxOptions': {
        args: ['-headless', ...extraArgs],
        prefs: { 'browser.download.folderList': 2, 'browser.download.dir': downloadDir, 'browser.download.useDownloadDir': true, 'browser.helperApps.neverAsk.saveToDisk': 'application/zip,application/octet-stream,audio/wav,audio/x-wav,text/plain', 'browser.download.manager.showWhenStarting': false, 'datareporting.policy.dataSubmissionEnabled': false, 'app.update.auto': false, 'app.update.enabled': false, ...extraPrefs },
      },
    } },
  };
  const s = await call('POST', '/session', caps);
  const sid = s.sessionId;
  const version = s.capabilities.browserVersion;
  const S = (m, p, b) => call(m, `/session/${sid}${p}`, b);
  if (scriptTimeoutMs > 0) await S('POST', '/timeouts', { script: scriptTimeoutMs });
  const ev = async (expression) => {
    const r = await S('POST', '/execute/async', { script: `const done = arguments[arguments.length - 1]; Promise.resolve().then(() => (${expression})).then((v) => done({ ok: v === undefined ? null : v }), (e) => done({ err: String(e && e.stack || e) }));`, args: [] });
    if (r.err) throw new Error('evaluate: ' + r.err.slice(0, 800));
    return r.ok;
  };
  const waitFor = async (expr, ms = 30000) => { const t0 = Date.now(); for (;;) { const v = await ev(expr).catch(() => undefined); if (v) return v; if (Date.now() - t0 > ms) throw new Error('waitFor timed out: ' + expr); await sleep(100); } };
  const element = async (selector) => { const e = await S('POST', '/element', { using: 'css selector', value: selector }); return Object.values(e)[0]; };
  const setFiles = async (selector, paths) => { const id = await element(selector); await S('POST', `/element/${id}/value`, { text: paths.join('\n') }); };
  // A folder chooser cannot be driven through WebDriver (Element Send Keys takes files, and Firefox gives them no webkitRelativePath), so the folder input is exercised by a SIMULATION of what the chooser
  // would hand over: File objects with the right bytes and an own webkitRelativePath, as the input's files; then 'change'. The page's handler (upload with folders) runs for real; the chooser does not.
  const setDirectory = async (selector, dir, files) => {
    const list = files.map((f) => ({ rel: path.relative(path.dirname(dir), f).split(path.sep).join('/'), b64: fs.readFileSync(f).toString('base64') }));
    await ev(`(async () => { const list = ${JSON.stringify(list)}; const fl = list.map((x) => { const bin = Uint8Array.from(atob(x.b64), (c) => c.charCodeAt(0)); const f = new File([bin], x.rel.split('/').pop()); Object.defineProperty(f, 'webkitRelativePath', { value: x.rel }); return f; }); const input = document.querySelector(${JSON.stringify(selector)}); Object.defineProperty(input, 'files', { configurable: true, get: () => fl }); input.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
    return 'SIMULATED: File objects with webkitRelativePath put into the input (WebDriver cannot choose a folder)';
  };
  const close = async () => { try { await call('DELETE', `/session/${sid}`); } catch { /* gone */ } gd.kill('SIGTERM'); };
  return { goto: (url) => S("POST", "/url", { url }), ev, waitFor, setFiles, setDirectory, close, version, dataDir, base, sid };
}
