#!/usr/bin/env node
// tools/fe-lift-test.mjs : `cargo test -p fe-lift` の失敗を、lift/crates/fe-lift/tests/known-failures.txt(1 行 1 件: <binary> <試験名>)と比べる。
// usage: node tools/fe-lift-test.mjs [--test <binary>]... [--known FILE] [--strict-known]
//   --test: その試験の binary だけを走らせる(繰り返せる)。無ければ全体(cargo test -p fe-lift --no-fail-fast と同じ)。
//   known の試験の失敗は数えて表示し、終了 0。known に無い失敗(試験・途中で止まった binary・known に無い binary の build の失敗)と、走らなかった試験の binary は、終了 1。
//   known の行の試験が通ったら「直った」と出す(known の行を消す合図)。--strict-known のとき、直った試験があれば終了 1。
//   build の失敗(link など)が出ると cargo は他の試験を 1 つも走らせない。そのときは、失敗した binary を除いて、残りを cargo で走らせ直す。
// 終了コード: 0 = 新しい失敗が無い。1 = 新しい失敗・走らなかった試験・内訳の読めない失敗がある。2 = 使い方の誤り。
// cargo の出力(標準出力と標準エラー)は、一時フォルダ .tmp-fe-lift-test-*(リポジトリの中。終わりに消す)の 1 つのファイルに、出た順に書く。
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const MANIFEST = 'lift/Cargo.toml';
const PACKAGE = 'fe-lift';
const BIN = 'fe-lift';
const TESTS_DIR = path.join(ROOT, 'lift/crates/fe-lift/tests');
const DEFAULT_KNOWN = path.join(TESTS_DIR, 'known-failures.txt');
const BASE_ARGS = ['test', '--locked', '--manifest-path', MANIFEST, '-p', PACKAGE, '--no-fail-fast'];
const USAGE = 'usage: node tools/fe-lift-test.mjs [--test <binary>]... [--known FILE] [--strict-known]';

class UsageError extends Error {}

// 解析: cargo の出力を、binary ごとの塊にする。塊は、見出し(Running・Doc-tests)から次の見出しまで
export function parseCargoOutput(text) {
  const blocks = [];
  const builds = new Map();
  let cur = null;
  for (const line of text.replace(/\r/g, '').split('\n')) {
    let m;
    if ((m = /^\s*Running (.+?) \(/.exec(line))) {
      cur = newBlock(labelOfRunning(m[1]));
      blocks.push(cur);
      continue;
    }
    if (/^\s*Doc-tests \S/.test(line)) {
      cur = newBlock('doc');
      blocks.push(cur);
      continue;
    }
    if ((m = /could not compile `[^`]*` \(([^)]*)\)/.exec(line))) {
      for (const item of m[1].split(', ')) builds.set(labelOfBuildItem(item), line.trim());
      continue;
    }
    if (!cur) continue;
    if ((m = /^test (.+) \.\.\. (ok|FAILED|ignored)$/.exec(line))) {
      if (m[2] === 'ok') cur.passed.add(m[1]);
      else if (m[2] === 'FAILED') cur.failed.add(m[1]);
      continue;
    }
    if (line.startsWith('test result: ')) cur.resultSeen = true;
    if (line.startsWith('error: test failed, to rerun pass ')) cur.testFailedLine = true;
    cur.lines.push(line);
  }
  return { blocks, builds };
}

function newBlock(label) {
  return { label, passed: new Set(), failed: new Set(), resultSeen: false, testFailedLine: false, lines: [] };
}

// binary の名前。tests/<名前>.rs・tests/<名前>/main.rs は名前そのもの。単体試験は lib と bin、文書の試験は doc
function labelOfRunning(what) {
  const m = /^tests\/(.+)\.rs$/.exec(what);
  if (m) return m[1].endsWith('/main') ? m[1].slice(0, -'/main'.length) : m[1];
  if (what === 'unittests src/lib.rs') return 'lib';
  if (what === 'unittests src/main.rs') return 'bin';
  return what;
}

// build の失敗の対象: test "名前" は名前、lib test は lib、bin "fe-lift" test は bin。知らない形は、そのまま(known に無いので、新しい失敗になる)
function labelOfBuildItem(item) {
  let m;
  if ((m = /^test "(.+)"$/.exec(item))) return m[1];
  if (item === 'lib' || item === 'lib test') return 'lib';
  if (/^bin "(.+)"( test)?$/.test(item)) return 'bin';
  return item;
}

export function loadKnown(text) {
  const keys = new Set();
  for (const raw of text.replace(/\r/g, '').split('\n')) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    const [label, ...rest] = line.split(/\s+/);
    if (rest.length === 0) throw new Error(`known-failures の行に試験名が無い: ${line}`);
    keys.add(`${label} ${rest.join(' ')}`);
  }
  return keys;
}

// 失敗した試験の、出力の先頭(---- 名前 stdout ---- の後の 4 行まで)
function detailOf(block, name) {
  const at = block.lines.indexOf(`---- ${name} stdout ----`);
  if (at < 0) return [];
  const out = [];
  for (const l of block.lines.slice(at + 1)) {
    if (out.length >= 4 || l.startsWith('---- ') || l === 'failures:' || l.startsWith('test result: ')) break;
    if (l.trim() !== '') out.push(l.trim().slice(0, 200));
  }
  return out;
}

// 判定。runs: [{ status, parsed }](cargo を走らせた回の、終了コードと解析の結果)。expected: 必ず走るべき試験の binary
export function judge({ runs, known, expected = [], strictKnown = false }) {
  const res = { newFailures: [], knownFailed: 0, knownBuild: new Set(), fixed: [], passed: 0, ran: new Set(), problems: [], knownTotal: known.size, exitCode: 0 };
  const knownLabels = new Set([...known].map((k) => k.split(' ')[0]));
  const fixedKeys = new Set();
  const built = new Set();
  for (const { status, parsed } of runs) {
    let attributable = 0;
    for (const b of parsed.blocks) {
      res.ran.add(b.label);
      res.passed += b.passed.size;
      for (const name of b.failed) {
        attributable++;
        if (known.has(`${b.label} ${name}`)) res.knownFailed++;
        else res.newFailures.push({ kind: 'test', label: b.label, name, detail: detailOf(b, name) });
      }
      for (const name of b.passed) {
        const key = `${b.label} ${name}`;
        if (known.has(key) && !fixedKeys.has(key)) {
          fixedKeys.add(key);
          res.fixed.push({ label: b.label, name });
        }
      }
      if (!b.resultSeen) {
        attributable++;
        res.newFailures.push({ kind: 'crash', label: b.label });
      } else if (b.testFailedLine && b.failed.size === 0) {
        attributable++;
        res.newFailures.push({ kind: 'binary', label: b.label });
      }
    }
    for (const [label, message] of parsed.builds) {
      attributable++;
      built.add(label);
      if (knownLabels.has(label)) res.knownBuild.add(label);
      else res.newFailures.push({ kind: 'build', label, message });
    }
    if (status !== 0 && attributable === 0) res.problems.push(`cargo test が終了コード ${status} で終わったが、失敗の内訳を読めない`);
  }
  for (const label of expected) {
    if (!res.ran.has(label) && !built.has(label)) res.problems.push(`実行されなかった試験の binary: ${label}`);
  }
  res.knownBuild = [...res.knownBuild];
  res.exitCode = res.newFailures.length > 0 || res.problems.length > 0 || (strictKnown && res.fixed.length > 0) ? 1 : 0;
  return res;
}

export function formatReport(res) {
  const lines = [];
  for (const f of res.fixed) lines.push(`直った: ${f.label} ${f.name}`);
  if (res.fixed.length) lines.push('直った試験の行を、lift/crates/fe-lift/tests/known-failures.txt から消す');
  for (const f of res.newFailures) {
    if (f.kind === 'test') {
      lines.push(`新しい失敗: ${f.label} ${f.name}`);
      for (const d of f.detail) lines.push(`    ${d}`);
    } else if (f.kind === 'crash') lines.push(`新しい失敗: ${f.label} が途中で止まった(結果の行が出ない)`);
    else if (f.kind === 'binary') lines.push(`新しい失敗: ${f.label} が失敗で終わった(失敗した試験の名前が出ない)`);
    else lines.push(`新しい失敗(ビルド): ${f.label}: ${f.message}`);
  }
  for (const label of res.knownBuild) lines.push(`既知のビルドの失敗: ${label}(その binary は走らない。残りは走らせ直した)`);
  for (const p of res.problems) lines.push(`問題: ${p}`);
  lines.push(`fe-lift の試験: 通った ${res.passed} 件・既知の失敗 ${res.knownFailed} 件(既知の一覧 ${res.knownTotal} 件)・既知のビルドの失敗 ${res.knownBuild.length} 件・直った ${res.fixed.length} 件・新しい失敗 ${res.newFailures.length} 件・実行した binary ${res.ran.size} 件`);
  return lines;
}

export function discoverTests(dir = TESTS_DIR) {
  const names = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.isFile() && e.name.endsWith('.rs')) names.push(e.name.slice(0, -3));
    else if (e.isDirectory() && fs.existsSync(path.join(dir, e.name, 'main.rs'))) names.push(e.name);
  }
  return names.sort();
}

function parseArgs(argv) {
  const opt = { tests: [], known: DEFAULT_KNOWN, strictKnown: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--test' || a === '--known') {
      const v = argv[++i];
      if (v === undefined || v.startsWith('--')) throw new UsageError(`${a} に値が無い`);
      if (a === '--known') opt.known = path.resolve(v);
      else if (/^[A-Za-z0-9_-]+$/.test(v)) opt.tests.push(v);
      else throw new UsageError(`試験の binary の名前が不正: ${v}`);
    } else if (a === '--strict-known') opt.strictKnown = true;
    else throw new UsageError(`知らない引数: ${a}`);
  }
  opt.tests = [...new Set(opt.tests)];
  return opt;
}

// cargo を 1 回走らせる。出力は 1 つのファイルに、出た順に書く(標準出力と標準エラーの順を保つ)
function runCargo(args, tmp, seq) {
  const logPath = path.join(tmp, `cargo-${seq}.log`);
  const fd = fs.openSync(logPath, 'w');
  const all = [...BASE_ARGS, ...args];
  process.stderr.write(`$ cargo ${all.join(' ')}\n`);
  return new Promise((resolve, reject) => {
    const child = spawn('cargo', all, { cwd: ROOT, stdio: ['ignore', fd, fd] });
    child.on('error', (e) => {
      fs.closeSync(fd);
      reject(e);
    });
    child.on('close', (code) => {
      fs.closeSync(fd);
      resolve({ status: code === null ? 1 : code, parsed: parseCargoOutput(fs.readFileSync(logPath, 'utf8')) });
    });
  });
}

// 試験の列。tests が空なら全体(cargo test -p fe-lift と同じ呼び出しから始める)。build の失敗が出たら、その binary を除いて残りを走らせ直す
async function runAll(tests, tmp) {
  const full = tests.length === 0;
  const items = full
    ? [{ label: 'lib', args: ['--lib'] }, { label: 'bin', args: ['--bin', BIN] }, ...discoverTests().map((t) => ({ label: t, args: ['--test', t] })), { label: 'doc', doc: true }]
    : tests.map((t) => ({ label: t, args: ['--test', t] }));
  let seq = 0;
  const runSet = async (sel) => {
    const made = [];
    const nonDoc = sel.filter((i) => !i.doc);
    if (nonDoc.length) made.push(await runCargo(nonDoc.flatMap((i) => i.args), tmp, seq++));
    if (sel.some((i) => i.doc)) made.push(await runCargo(['--doc'], tmp, seq++));
    return made;
  };
  const runs = full ? [await runCargo([], tmp, seq++)] : await runSet(items);
  const excluded = new Set();
  let last = runs;
  for (;;) {
    const fresh = new Set();
    for (const r of last) for (const label of r.parsed.builds.keys()) if (!excluded.has(label)) fresh.add(label);
    if (fresh.size === 0) break;
    for (const label of fresh) excluded.add(label);
    const ranNow = new Set(runs.flatMap((r) => r.parsed.blocks.map((b) => b.label)));
    const pending = items.filter((i) => !ranNow.has(i.label) && !excluded.has(i.label));
    if (pending.length === 0) break;
    last = await runSet(pending);
    runs.push(...last);
  }
  return { runs, items };
}

async function main(argv) {
  let opt;
  try {
    opt = parseArgs(argv);
  } catch (e) {
    if (!(e instanceof UsageError)) throw e;
    console.error(`fe-lift-test: ${e.message}`);
    console.error(USAGE);
    return 2;
  }
  const known = loadKnown(fs.readFileSync(opt.known, 'utf8'));
  const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-fe-lift-test-'));
  try {
    const { runs, items } = await runAll(opt.tests, tmp);
    const expected = items.filter((i) => i.args && i.args[0] === '--test').map((i) => i.label);
    const res = judge({ runs, known, expected, strictKnown: opt.strictKnown });
    for (const line of formatReport(res)) console.log(line);
    return res.exitCode;
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  process.exitCode = await main(process.argv.slice(2)).catch((e) => {
    console.error(`fe-lift-test: ${e.message}`);
    return 1;
  });
}
