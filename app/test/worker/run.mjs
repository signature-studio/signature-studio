// Starts the Worker under `wrangler dev --local` and runs test/worker/worker.test.ts against it. Every Worker is configured for a test Access application (decision 0243: there is no switch that skips the check):
//   dev, guard, noperf   behind a proxy that does what Access does (a valid Cf-Access-Jwt-Assertion on every request); the tests that are not about Access ask the proxy
//   access               the Worker itself: JWT enforced, the signing keys come from a local JWKS server; the tests send the tokens they want (valid, wrong aud, expired ...)
//   closed               wrangler.jsonc as shipped (team / aud empty) -> every request is 403
// Fixtures are put into the local R2 simulation with `wrangler r2 object put --local`.
import fs from 'node:fs';
import path from 'node:path';
import { FIXTURE, MANIFEST_REAL, PRE_BR } from '../helpers/fixtures.ts';
import { run, wranglerEnv } from '../helpers/proc.mjs';
import { APP, putObject, startAccessKeys, startWorker, workDirOf } from '../helpers/wrangler.mjs';

const work = workDirOf('worker-test');
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
const bin = (n) => path.join(APP, 'node_modules/.bin', n);
const stops = [];
let status = 1;
try {
  if (!fs.existsSync(path.join(APP, 'dist/index.html'))) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });

  // fixtures
  const fx = path.join(work, 'fixtures');
  fs.mkdirSync(fx, { recursive: true });
  fs.writeFileSync(path.join(fx, 'blob.bin'), FIXTURE.blob);
  fs.writeFileSync(path.join(fx, 'engine.wasm'), FIXTURE.wasm);
  fs.writeFileSync(path.join(fx, 'pre.wasm.br'), PRE_BR);
  fs.writeFileSync(path.join(fx, 'pinned.bin'), 'x');
  fs.writeFileSync(path.join(fx, 'manifest.json'), MANIFEST_REAL); // the default manifest is for the real engine only (decision 0152)
  await putObject(work, 'blob.bin', path.join(fx, 'blob.bin'), { contentType: 'application/octet-stream' });
  await putObject(work, 'engine.wasm', path.join(fx, 'engine.wasm'), { contentType: 'application/octet-stream' }); // the Worker must still say application/wasm
  await putObject(work, 'pre.wasm', path.join(fx, 'pre.wasm.br'), { contentType: 'application/wasm', contentEncoding: 'br' });
  await putObject(work, 'v1-0123456789ab/pinned.bin', path.join(fx, 'pinned.bin'));
  await putObject(work, 'manifest.json', path.join(fx, 'manifest.json'), { contentType: 'application/json' });

  // Access: a throw-away key pair, a JWKS server on 127.0.0.1, tokens
  const keys = await startAccessKeys();
  stops.push(keys.stop);
  fs.writeFileSync(path.join(work, 'tokens.json'), JSON.stringify(keys.tokens));

  const dev = await startWorker(work, {}, { keys });
  stops.push(dev.stop);
  const access = await startWorker(work, {}, { keys });
  stops.push(access.stop);
  const closed = await startWorker(work, {}, { unconfigured: true });
  stops.push(closed.stop);
  // decision 0152: a Worker of its own whose R2 starts EMPTY; the guard tests put a stand-in, a broken manifest ... at the key manifest.json and take them away again
  const gwork = workDirOf('worker-test-guard');
  fs.rmSync(gwork, { recursive: true, force: true });
  fs.mkdirSync(gwork, { recursive: true });
  const guard = await startWorker(gwork, {}, { keys });
  stops.push(guard.stop);

  // decision 0224: a Worker whose config has no PERF_BUCKET binding (the beacon is still checked and answered 204, but says it was not stored)
  const noperf = path.join(work, 'wrangler.noperf.jsonc');
  const base = JSON.parse(fs.readFileSync(path.join(APP, 'wrangler.jsonc'), 'utf8').replace(/^\s*\/\/.*$/gm, '').replace(/\s\/\/ .*$/gm, '').replace(/,(\s*[}\]])/g, '$1'));
  base.main = path.join(APP, 'worker/index.ts');
  base.assets.directory = path.join(APP, 'dist');
  base.r2_buckets = base.r2_buckets.filter((b) => b.binding !== 'PERF_BUCKET');
  fs.writeFileSync(noperf, JSON.stringify(base, null, 2));
  const nowork = workDirOf('worker-test-noperf');
  fs.rmSync(nowork, { recursive: true, force: true });
  fs.mkdirSync(nowork, { recursive: true });
  const noperfWorker = await startWorker(nowork, {}, { config: noperf, keys });
  stops.push(noperfWorker.stop);

  const env = { ...wranglerEnv(work), WORKER_DEV_WORK: work, WORKER_NOPERF_URL: noperfWorker.url, WORKER_DEV_URL: dev.url, WORKER_GUARD_URL: guard.url, WORKER_GUARD_WORK: gwork, WORKER_ACCESS_URL: access.directUrl, WORKER_CLOSED_URL: closed.directUrl, WORKER_TOKENS: path.join(work, 'tokens.json'), PATH: process.env.PATH };
  const out = await run(bin('vitest'), ['run', '-c', 'vitest.worker.config.ts', '--reporter=verbose'], { cwd: APP, env, label: 'vitest (worker)' }).catch((e) => { process.stdout.write(String(e.message) + '\n'); throw e; });
  process.stdout.write(out);
  fs.writeFileSync(path.join(work, 'jwks-hits.txt'), `JWKS server hits: ${keys.hits()}\n`);
  console.log(`JWKS server hits during the run: ${keys.hits()}`);
  status = 0;
} catch (e) {
  console.error(String(e.stack ?? e));
} finally {
  for (const s of stops) await s();
}
process.exit(status);
