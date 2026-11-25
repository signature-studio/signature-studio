#!/usr/bin/env node
// spec/perf-stops.test.mjs : tools/perf-beacons/summary.mjs --stops (Issue #1404): the stop fingerprint (numbers and hex addresses become #, every Windows path becomes <path>, the cell's "|" is escaped), the sessions it counts (a stop, an error, or no first idle), the window, and the printed table. It reads the synthetic fixture (tools/perf-beacons/fixtures/stop) and records built in a temporary folder .tmp-spec-stops-* (removed at the end). Exit code 0 = all passed.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/perf-beacons/summary.mjs');
const FIXTURE = path.join(ROOT, 'tools/perf-beacons/fixtures/stop');
const TMP = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-stops-'));
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const run = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8' });
const { fingerprintPart } = await import(pathToFileURL(TOOL).href);

try {
  ok('part: a drive path is <path> (folders with spaces included), numbers and hex addresses are #, "|" is "/"',
    fingerprintPart('fle: X: C:\\Windows\\system32\\a.dll failed 0x1f, code 12 | y'), 'fle: X: <path> failed #, code # / y');
  ok('part: a path with spaces in its folder names is one <path>', fingerprintPart('stop at C:\\Program Files\\Image-Line\\FL Studio 21\\x.flp here'), 'stop at <path> here');
  ok('part: a URL, an e-mail address and an address are removed', fingerprintPart('see https://h.example/x a@b.example 10.1.2.3'), 'see <url> <email> <ip>');
  ok('part: a POSIX path is <path>', fingerprintPart('read /home/u/x y'), 'read <path> y');
  ok('part: the same wall at other numbers and addresses has the same text', fingerprintPart('lid 8 at 0x411e4d'), fingerprintPart('lid 13 at 0x5af85c'));

  const rec = (sid, seq, t, perf) => ({ t, p: { sid, seq, ...perf } });
  const V = { version: 'vtest00000001' };
  const stopPerf = {
    reason: 'exit', pageMs: 120000, bundle: V, guestScreen: { dpr: 1.7647 },
    engine: { state: 'exited', exit: { reason: 'stopped', code: 134, presents: 1, stop: { lastFle: 'fle: RegOpenKeyExW: an empty component, reached after 3 keys', stopLine: '== stop: SIGABRT (runtime abort, see runtime message)', abortKind: '(not a runtime message)', memoryBytes: 1 } } },
  };
  const quietPerf = { reason: 'periodic', pageMs: 15000, bundle: V, engine: { state: 'engine-idle', exit: null } };
  const startedPerf = { reason: 'first-idle', pageMs: 9000, bundle: V, engine: { state: 'engine-idle', exit: null } };
  const recs = [
    rec('aaaaaaaaaaaa', 1, '2026-11-24T10:00:00.000Z', quietPerf),
    rec('aaaaaaaaaaaa', 2, '2026-11-24T10:00:30.000Z', stopPerf),
    rec('bbbbbbbbbbbb', 1, '2026-11-24T10:05:00.000Z', { reason: 'periodic', pageMs: 42000, bundle: V, engine: { state: 'loading', exit: null }, failure: { kind: 'left-before-first-idle' } }),
    rec('cccccccccccc', 1, '2026-11-24T10:06:00.000Z', startedPerf),
    rec('cccccccccccc', 2, '2026-11-24T10:06:30.000Z', { ...startedPerf, reason: 'periodic', pageMs: 30000 }),
    rec('dddddddddddd', 1, '2026-11-24T10:07:00.000Z', { ...startedPerf, error: { what: 'trap at wasm-function[12]:0x9f', where: 'engine worker, running' }, engine: { state: 'error', exit: { reason: 'trap', code: -1, presents: 0 } } }),
  ];
  const dir = path.join(TMP, 'recs');
  fs.mkdirSync(dir);
  for (const r of recs) fs.writeFileSync(path.join(dir, `${r.p.sid}-${r.p.seq}.json`), JSON.stringify({ t: r.t, remote: 'workers', perf: r.p }));

  const fx = run([FIXTURE, '--stops', '--now', '2026-11-24T08:00:00Z']);
  ok('cli: the fixture with --stops exits 0', fx.status, 0);
  ok('cli: the fixture names its stop and its version, and prints no session id', [fx.stdout.includes('StopFixture'), fx.stdout.includes('## 版 d7a6e6f392f2'), fx.stdout.includes('5a0f1e2d3c4b')], [true, true, false]);
  ok('cli: the fixture fingerprint is the five parts joined by " | " (escaped in the cell)', fx.stdout.includes('fle: StopFixture: the last fle line of a stop (synthetic fixture) \\| == stop: SIGABRT (runtime abort, see runtime message) \\| fle_stop (a runtime message that names an API and the reason) \\| - \\| -'), true);

  const out = run([dir, '--stops', '--now', '2026-11-24T12:00:00Z']);
  ok('cli: the built records exit 0', out.status, 0);
  ok('cli: 3 sessions stopped (a stop, a session that never reached the first idle, an error); the started one is not counted',
    [out.stdout.includes('停止したセッション: 3 件(記録 4 件)'), out.stdout.includes('停止 1 件、エラー 1 件、届かず離脱 1 件')], [true, true]);
  ok('cli: the gone session row: the fingerprint is "-" in all five parts, kind 届かず離脱', out.stdout.includes('| - \\| - \\| - \\| - \\| - | 1 | 1 | 届かず離脱 ×1 |'), true);
  ok('cli: the stop row: records, sessions, kind, times, DPR class, file answers (no record), page time class', out.stdout.includes('| fle: RegOpenKeyExW: an empty component, reached after # keys \\| == stop: SIGABRT (runtime abort, see runtime message) \\| (not a runtime message) \\| - \\| - | 2 | 1 | 停止 ×1 | 2026-11-24 10:00Z | 2026-11-24 10:00Z | 1 超 2 未満 ×1 | 記録無し ×1 | 記録無し ×1 | 60 秒以上 ×1 |'), true);
  ok('cli: the error row: its what and where, kind エラー', out.stdout.includes('| - \\| - \\| - \\| trap at wasm-function[#]:# \\| engine worker, running | 1 | 1 | エラー ×1 |'), true);
  const rows = out.stdout.split('\n').filter((l) => l.startsWith('| '));
  ok('table: every line has the same cell count as the header (the cell separator is escaped)', [...new Set(rows.map((l) => l.replace(/\\\|/g, '').split('|').length))], [12]);
  const empty = run([dir, '--stops', '--now', '2030-01-01T00:00:00Z']);
  ok('empty window: exits 0 and prints the counts, no table', [empty.status, empty.stdout.includes('停止したセッションは無い'), empty.stdout.includes('| --- |')], [0, true, false]);
  const both = run([dir, '--stops', '--exits']);
  ok('--stops with --exits: refused (exit 2)', both.status, 2);
} finally {
  fs.rmSync(TMP, { recursive: true, force: true });
}

if (failed > 0) {
  console.log(`${failed} failed`);
  process.exit(1);
}
console.log('all passed');
