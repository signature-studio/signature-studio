#!/usr/bin/env node
// spec/perf-funcnames.test.mjs : tools/perf-beacons/summary.mjs --stops with the engine's function table (Issue #1404): a trap's wasm-function[N] is named from the table of the record's own version (--func-names: a file in a folder func-names-<版>; --func-names-dir: such folders), and a trap that cannot be named says why ([fn=? 表が無い版] or [fn=? 表に無い番号]). The table is the header line and then index, kind, va, name (plain or gzipped). It reads synthetic records and tables in a temporary folder .tmp-spec-funcnames-* (removed at the end). Exit code 0 = all passed.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/perf-beacons/summary.mjs');
const TMP = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-funcnames-'));
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const run = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8' });
// the table rows of the report (the text of its counts and notes is not a row)
const rowsOf = (out) => out.split('\n').filter((l) => l.startsWith('| '));

const VA = 'aaaaaaaaaaaa';
const VB = 'bbbbbbbbbbbb';
const VC = 'cccccccccccc';
const TABLE = 'index\tkind\tva\tname\n0\timport\t\tc.f1\n1\tf\t0x40\tf_40\n2\tc\t\tfle_progress_slice\n3\tc\t\tunnamed_3\n';
const TRAP_WHERE = 'engine worker, running (the engine trapped before it could stop by itself)';
const trapWhat = (n, off) => `@https://h.example/fl/engine-glue.js line 2 > WebAssembly.compile:wasm-function[${n}]:0x${off} (other)`;
const trap = (sid, version, n, off) => ({
  t: '2026-11-24T10:00:00.000Z',
  p: {
    sid, seq: 1, reason: 'exit', pageMs: 85000, bundle: { name: 'fl-Oz', version }, guestScreen: { dpr: 1.75 },
    engine: { state: 'error', exit: { reason: 'trap', code: -1, presents: 0 } },
    error: { what: trapWhat(n, off), where: TRAP_WHERE },
  },
});

try {
  const { parseFuncNames } = await import(pathToFileURL(TOOL).href);
  const names = typeof parseFuncNames === 'function' ? parseFuncNames(TABLE) : null;
  ok('parse: the table is read by its index (the header line is not a function)', names && [names.size, names.get(0)?.kind, names.get(2)?.name, names.get(3)?.name], [4, 'import', 'fle_progress_slice', 'unnamed_3']);
  ok('parse: a file whose first line is not the header is refused', (() => { try { parseFuncNames('0\tf\t0x40\tf_40\n'); return 'read'; } catch { return 'refused'; } })(), 'refused');

  const tables = path.join(TMP, 'tables');
  fs.mkdirSync(path.join(tables, `func-names-${VA}`), { recursive: true });
  fs.mkdirSync(path.join(tables, `func-names-${VB}`), { recursive: true });
  fs.mkdirSync(path.join(tables, 'other'), { recursive: true });
  fs.writeFileSync(path.join(tables, `func-names-${VA}`, 'wasm-func-names.tsv.gz'), zlib.gzipSync(Buffer.from(TABLE)));
  fs.writeFileSync(path.join(tables, `func-names-${VB}`, 'wasm-func-names.tsv'), Buffer.from('index\tkind\tva\tname\n0\timport\t\tc.f1\n1\tf\t0x40\tf_40\n'));
  fs.writeFileSync(path.join(tables, 'other', 'wasm-func-names.tsv'), 'not a table of a version\n');

  const recs = [
    trap('s1aaaaaaaaaaaa', VA, 2, '93b89be'),
    trap('s2bbbbbbbbbbbb', VB, 1, '5'),
    trap('s3cccccccccccc', VC, 2, '10'),
    trap('s4aaaaaaaaaaaa', VA, 77, '1'),
  ];
  const dir = path.join(TMP, 'recs');
  fs.mkdirSync(dir);
  for (const r of recs) fs.writeFileSync(path.join(dir, `${r.p.sid}-${r.p.seq}.json`), JSON.stringify({ t: r.t, remote: 'workers', perf: r.p }));
  const NOW = ['--now', '2026-11-24T12:00:00Z'];

  const all = run([dir, '--stops', '--func-names-dir', tables, ...NOW]);
  ok('dir: the tables of the folder func-names-<版> name the traps of their versions (gzipped or plain)', all.status, 0);
  ok('dir: the named function is in the fingerprint, and the reasons are told apart',
    [all.stdout.includes('[fn=fle_progress_slice]'), all.stdout.includes('[fn=f_40]'), all.stdout.includes('[fn=? 表が無い版]'), all.stdout.includes('[fn=? 表に無い番号]')],
    [true, true, true, true]);
  ok('dir: the version with no table says 表が無い版 (its row only)', rowsOf(all.stdout).filter((l) => l.includes('[fn=? 表が無い版]')).length, 1);
  ok('dir: the table has no such index: 表に無い番号 (its row only: 77 of the version aaaa)', rowsOf(all.stdout).filter((l) => l.includes('[fn=? 表に無い番号]')).length, 1);
  ok('dir: the session ids are not printed', ['s1aaaaaaaaaaaa', 's2bbbbbbbbbbbb', 's3cccccccccccc', 's4aaaaaaaaaaaa'].filter((s) => all.stdout.includes(s)), []);

  const none = run([dir, '--stops', ...NOW]);
  ok('no table: every version says 表が無い版 (3 rows: aaaa, bbbb, cccc), and no name (the table is an option)', [none.status, rowsOf(none.stdout).filter((l) => l.includes('[fn=? 表が無い版]')).length, none.stdout.includes('[fn=f_')], [0, 3, false]);

  const one = run([dir, '--stops', '--func-names', path.join(tables, `func-names-${VA}`, 'wasm-func-names.tsv.gz'), ...NOW]);
  ok('one file: a table in the folder func-names-<版> names that version only', [one.status, one.stdout.includes('[fn=fle_progress_slice]'), one.stdout.includes('[fn=f_40]'), rowsOf(one.stdout).filter((l) => l.includes('[fn=? 表が無い版]')).length], [0, true, false, 2]);

  const loose = path.join(TMP, 'loose');
  fs.mkdirSync(loose);
  fs.copyFileSync(path.join(tables, `func-names-${VA}`, 'wasm-func-names.tsv.gz'), path.join(loose, 'wasm-func-names.tsv.gz'));
  const bad = run([dir, '--stops', '--func-names', path.join(loose, 'wasm-func-names.tsv.gz'), ...NOW]);
  ok('one file not in a folder func-names-<版>: refused (exit 2), since the version is not known', bad.status, 2);
  ok('one file not in a folder func-names-<版>: the message says the folder name', bad.stderr.includes('func-names-'), true);

  const missing = run([dir, '--stops', '--func-names', path.join(tables, 'func-names-nope', 'wasm-func-names.tsv'), ...NOW]);
  ok('a file that does not exist: refused (exit 2)', missing.status, 2);

  const emptyDir = path.join(TMP, 'tables-empty', `func-names-${VC}`);
  fs.mkdirSync(emptyDir, { recursive: true });
  const noFile = run([dir, '--stops', '--func-names-dir', path.join(TMP, 'tables-empty'), ...NOW]);
  ok('a folder func-names-<版> without wasm-func-names.tsv(.gz): refused (exit 2)', noFile.status, 2);

  const badHead = path.join(TMP, 'tables-bad', `func-names-${VC}`);
  fs.mkdirSync(badHead, { recursive: true });
  fs.writeFileSync(path.join(badHead, 'wasm-func-names.tsv'), '0\tf\t0x40\tf_40\n');
  const badRun = run([dir, '--stops', '--func-names-dir', path.join(TMP, 'tables-bad'), ...NOW]);
  ok('a table whose first line is not the header: refused (exit 2)', badRun.status, 2);

  const alone = run([dir, '--func-names-dir', tables, ...NOW]);
  ok('--func-names without --stops: refused (exit 2)', alone.status, 2);

  const js = run([dir, '--stops', '--json', '--func-names-dir', tables, ...NOW]);
  const parsed = (() => { try { return JSON.parse(js.stdout); } catch { return null; } })();
  const fps = parsed ? parsed.rows.map((x) => x.fingerprint) : [];
  ok('--json: the same names in the full fingerprints (the table cell is cut; the JSON is not)',
    [js.status, fps.filter((f) => f.includes('[fn=fle_progress_slice]')).length, fps.filter((f) => f.includes('[fn=f_40]')).length], [0, 1, 1]);
  ok('--json: no session id', ['s1aaaaaaaaaaaa', 's2bbbbbbbbbbbb'].filter((s) => js.stdout.includes(s)), []);
} finally {
  fs.rmSync(TMP, { recursive: true, force: true });
}

if (failed > 0) {
  console.log(`${failed} failed`);
  process.exit(1);
}
console.log('all passed');
