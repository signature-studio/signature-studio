// Child-process helpers for the tests: start a server in its own process group, wait for a line of its output, stop the whole group.
import { spawn } from 'node:child_process';
import net from 'node:net';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export function freePort() {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); });
    s.on('error', reject);
  });
}

// An environment for wrangler that cannot reach the operator's real wrangler configuration or credentials:
// the config / log directories point into the work directory, no Cloudflare token variables are passed on, telemetry is off.
export function wranglerEnv(workDir) {
  const env = { ...process.env, XDG_CONFIG_HOME: `${workDir}/xdg`, WRANGLER_LOG_PATH: `${workDir}/wrangler-logs`, WRANGLER_SEND_METRICS: 'false', CI: '1', NO_COLOR: '1' };
  for (const k of Object.keys(env)) if (/^(CLOUDFLARE_|CF_)/.test(k)) delete env[k];
  return env;
}

export function start(cmd, args, { cwd, env, ready, timeoutMs = 90000, label = cmd }) {
  const child = spawn(cmd, args, { cwd, env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let out = '';
  const done = new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`${label}: not ready after ${timeoutMs} ms. Output:\n${out.slice(-3000)}`)), timeoutMs);
    const onData = (d) => { out += d; if (ready.test(out)) { clearTimeout(t); resolve(); } };
    child.stdout.on('data', onData);
    child.stderr.on('data', onData);
    child.on('exit', (code) => { clearTimeout(t); reject(new Error(`${label}: exited with ${code} before it was ready. Output:\n${out.slice(-3000)}`)); });
  });
  const stop = async () => {
    try { process.kill(-child.pid, 'SIGTERM'); } catch { /* already gone */ }
    await Promise.race([new Promise((r) => child.once('exit', r)), sleep(5000)]);
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* gone */ }
  };
  return { child, ready: done, stop, output: () => out };
}

export function run(cmd, args, { cwd, env, label = cmd }) {
  return new Promise((resolve, reject) => {
    const c = spawn(cmd, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    c.stdout.on('data', (d) => { out += d; });
    c.stderr.on('data', (d) => { out += d; });
    c.on('exit', (code) => (code === 0 ? resolve(out) : reject(new Error(`${label} exited with ${code}:\n${out.slice(-3000)}`))));
  });
}
