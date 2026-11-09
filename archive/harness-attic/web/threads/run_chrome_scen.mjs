// Runs the thread scenarios (thread_scenarios.c built for the web: build_scen_wasm.sh ... web) in a headless Chrome and compares each with the native run of the same scenario
// (decision 0124, PROVISIONAL). Everything in one foreground node process: the local server (../serve.mjs), a Chrome with a throw-away profile (../cdp.mjs), CDP.
// Isolation (decision 0124, item 4 (ii)): the page runs in a NEW BROWSER CONTEXT (Target.createBrowserContext, disposed at the end), the profile is a throw-away directory under the work dir (the
// user's profile is never touched), the network is cut by --host-resolver-rules="MAP * ~NOTFOUND, EXCLUDE 127.0.0.1" (no proxy), the background
// services are switched off, and the net-log of the whole run is written and checked by check_netlog.mjs (socket connections to anything but 127.0.0.1 must be 0; URL requests that Chrome
// itself tries to make to other hosts are listed with their net error: they must all end at name resolution).
// usage: node run_chrome_scen.mjs dist=<dir with scen.js scen.wasm> out=<dir> native=<dir with <name>.native.{out,err,rc} from compare_scen.sh> [coi=1] [jsflags='--flag ...'] [only=a,b,c]
//        [bench='ping:100000,yield:100000,sleep0:100000,timer:10000,idle:20000'] [stackprobe=984,150,145 (the JSPI stack probe of the same page)] [profile=<dir>]
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { startServer } from '../serve.mjs';
import { launchChrome, Cdp, shutdown, sleep } from '../cdp.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const A = Object.fromEntries(process.argv.slice(2).map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
for (const k of ['dist', 'out', 'native']) if (!A[k]) { console.error('missing ' + k + '='); process.exit(2); }
const dist = path.resolve(A.dist), outDir = path.resolve(A.out), nativeDir = path.resolve(A.native);
fs.mkdirSync(outDir, { recursive: true });
const profile = path.resolve(A.profile || path.join(outDir, 'chrome-profile'));
fs.rmSync(profile, { recursive: true, force: true });
const netlog = path.join(outDir, 'netlog.json');
const prefix = A.coi === '1' ? '/coi' : '';

const { server, port } = await startServer({ dist });
const extra = [
  '--password-store=basic',                      // jspi-real-wasm.md F3: without it Page.navigate did not complete in this environment (cause not confirmed)
  '--host-resolver-rules=MAP * 127.0.0.1:9, EXCLUDE 127.0.0.1',   // every name except 127.0.0.1 is mapped to 127.0.0.1:9 (nothing listens): a request for another host can only connect to loopback.
  // (An earlier attempt mapped to ~NOTFOUND: the resolver then still ran its IPv6 reachability check, a UDP connect() to [2001:4860:4860::8888]:443 that the net-log records; an IP literal needs no check.)
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',                          // no proxy (a proxy would resolve names itself and bypass the rule); no IPv6 reachability probe
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  `--crash-dumps-dir=${path.join(profile, 'crash')}`,
  `--log-net-log=${netlog}`, '--net-log-capture-mode=Everything',
  ...(A.jsflags ? [`--js-flags=${A.jsflags}`] : []),
];
const chrome = await launchChrome({ profile, extraArgs: extra });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const report = { chrome: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'], port, coi: A.coi === '1', jsflags: A.jsflags || null, args: chrome.args.filter((x) => !x.startsWith('--remote-debugging-port') && x !== 'about:blank') };
const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
report.browserContextId = browserContextId;
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params) => Promise.race([cdp.send(method, params, sessionId), sleep(600000).then(() => { throw new Error('CDP call timed out: ' + method); })]);
await S('Runtime.enable');
await S('Page.enable');
await S('Network.enable');
await S('Network.setCacheDisabled', { cacheDisabled: true });
const url = `http://127.0.0.1:${port}${prefix}/threads/threads.html`;
await S('Page.navigate', { url });
for (let i = 0; i < 100; i++) {
  const r = await S('Runtime.evaluate', { expression: 'window.__ready === true', returnByValue: true });
  if (r.result && r.result.value) break;
  await sleep(100);
}
const isolated = (await S('Runtime.evaluate', { expression: 'self.crossOriginIsolated', returnByValue: true })).result.value;
report.crossOriginIsolated = isolated;
report.url = url;
const distUrl = `${prefix}/dist/`;
async function runScen(args) {
  const expr = `window.runScen(${JSON.stringify(args)}, ${JSON.stringify(distUrl)})`;
  const r = await S('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) return { out: [], err: ['[page] ' + JSON.stringify(r.exceptionDetails).slice(0, 300)], code: 97, stats: null, ms: 0 };
  return r.result.value;
}
const norm = (txt) => txt.split('\n').map((l) => l.replace(/rsp 0x[0-9a-f]+/, 'rsp X')).filter((l) => l !== '' && !/^\s*at /.test(l) && !/^Aborted/.test(l) && !/^\[js\] /.test(l)).join('\n');
const names = A.only ? A.only.split(',') : fs.readdirSync(nativeDir).filter((f) => f.endsWith('.native.rc')).map((f) => f.replace('.native.rc', '')).sort((a, b) => a.localeCompare(b));
const results = [];
let same = 0;
for (const name of names) {
  const nrc = +fs.readFileSync(path.join(nativeDir, `${name}.native.rc`), 'utf8').trim();
  const nout = fs.readFileSync(path.join(nativeDir, `${name}.native.out`), 'utf8');
  const nerr = fs.readFileSync(path.join(nativeDir, `${name}.native.err`), 'utf8');
  const r = await runScen([name]);
  const wout = r.out.length ? r.out.join('\n') + '\n' : '';
  const werr = r.err.length ? r.err.join('\n') + '\n' : '';
  fs.writeFileSync(path.join(outDir, `${name}.chrome.out`), wout);
  fs.writeFileSync(path.join(outDir, `${name}.chrome.err`), werr);
  const outOk = wout === nout;
  const errOk = nrc === 0 ? werr === nerr : norm(werr) === norm(nerr);
  const rcOk = nrc === 0 ? r.code === 0 : r.code !== 0;
  const ok = outOk && errOk && rcOk;
  if (ok) same++;
  results.push({ name, ok, outOk, errOk, rcOk, nativeRc: nrc, chromeRc: r.code, errLines: nerr.split('\n').length - 1, ms: +r.ms.toFixed(1), stats: r.stats });
  console.log(`${ok ? 'IDENTICAL' : 'DIFFERENT'} ${name} (stdout ${outOk ? 'yes' : 'no'}, stderr ${errOk ? 'yes' : 'no'} [${nerr.split('\n').length - 1} lines], exit native ${nrc} chrome ${r.code})`);
}
console.log(`IDENTICAL ${same} of ${names.length}`);
report.identical = same; report.total = names.length; report.results = results;

// optional: JSPI stack probe on the same page (a fresh Worker per probe is NOT possible with a different V8 flag: the flag is for the whole browser; this probes the browser's own value)
if (A.stackprobe !== undefined) {
  const probe = async (depth, n, mask) => {
    const r = await runScen(['deep', String(depth), String(n), '0x' + mask.toString(16), '16']);
    const oks = r.out.filter((l) => /^thread \d+ deep \d+ ok$/.test(l)).length;
    return { ok: r.code === 0 && oks === n, failedThreads: r.err.filter((l) => l.startsWith('[js] fle thread')).map((l) => (/thread (\d+)/.exec(l) || [])[1]), err: r.err.filter((l) => /Maximum call stack|out of bounds|failed/.test(l)).slice(0, 2) };
  };
  const maxDepth = async (n, mask) => {
    let lo = 1, hi = 400;
    if (!(await probe(lo, n, mask)).ok) return 0;
    while ((await probe(hi, n, mask)).ok) { lo = hi; hi *= 2; if (hi > 1e6) return -1; }
    while (hi - lo > 1) { const mid = (lo + hi) >> 1; if ((await probe(mid, n, mask)).ok) lo = mid; else hi = mid; }
    return lo;
  };
  report.stack = { jsflags: A.jsflags || null, rows: [] };
  for (const [what, n, mask] of [['single deep thread 0', 3, 1], ['single deep thread 1', 3, 2], ['single deep thread 2', 3, 4], ['all deep 1', 1, 1], ['all deep 2', 2, 3], ['all deep 4', 4, 15], ['all deep 8', 8, 255], ['all deep 16', 16, 65535]]) {
    const d = await maxDepth(n, mask);
    report.stack.rows.push({ what, threads: n, mask, maxDepth: d });
    console.log(`stack ${A.jsflags || 'default'}: ${what} threads=${n}: max depth ${d}`);
  }
  for (const idx of [0, 1, 2]) {
    const d = await maxDepth(3, 1 << idx);
    const p = await probe(d + 1, 3, 1 << idx);
    report.stack.rows.push({ what: `thread ${idx} one frame beyond`, threads: 3, mask: 1 << idx, maxDepth: d + 1, probe: p });
    console.log(`stack ${A.jsflags || 'default'}: thread ${idx} depth ${d + 1}: ok=${p.ok} failedThreads=[${p.failedThreads}] ${JSON.stringify(p.err)}`);
  }
}

// optional: transfers per second (the transfer count is exact; the time is the browser's performance.now in the Worker: 100 us steps, 5 us when cross-origin isolated)
if (A.bench) {
  report.bench = [];
  for (const spec of A.bench.split(',')) {
    const [kind, n] = spec.split(':');
    const r = await runScen(['bench', kind, n]);
    const line = r.out.find((l) => l.startsWith('bench ')) || '';
    const m = /transfers~(\d+) elapsed_ms=([\d.]+)/.exec(line);
    const row = { kind, n: +n, line, code: r.code, transfers: m ? +m[1] : null, elapsedMs: m ? +m[2] : null, perSec: m ? Math.round(+m[1] / (+m[2] / 1000)) : null, stats: r.stats };
    report.bench.push(row);
    console.log(`bench ${kind} n=${n}: ${line} => ${row.perSec} transfers/s (code ${r.code})`);
  }
}
await shutdown(chrome, cdp);
server.close();
await sleep(500);
fs.writeFileSync(path.join(outDir, 'report.json'), JSON.stringify(report, null, 1));
const nl = spawnSync('node', [path.join(HERE, 'check_netlog.mjs'), netlog, '127.0.0.1', ...(process.env.NETLOG_TOLERATE_IPV6_PROBE === '1' ? ['tolerate-ipv6-probe'] : [])], { encoding: 'utf8' });
console.log((nl.stdout || '').trim() || nl.stderr);
fs.writeFileSync(path.join(outDir, 'netlog-summary.txt'), nl.stdout || nl.stderr);
fs.rmSync(profile, { recursive: true, force: true });
process.exit(same === names.length && nl.status === 0 ? 0 : 1);
