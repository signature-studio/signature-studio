#!/usr/bin/env node
// spec/fe-lift-test.test.mjs : tools/fe-lift-test.mjs(cargo test -p fe-lift の失敗を lift/crates/fe-lift/tests/known-failures.txt と比べる道具)を試験する。
// 解析・判定は、cargo の出力の形を写した合成の文字列で試す(cargo は走らせない)。CLI は、偽の cargo(一時フォルダの node の script を PATH の先頭に置く)で、
// 再実行の順序・終了コード・報告の文言を試す。一時フォルダはリポジトリの中の .tmp-spec-felift-*(終わりに消す)。終了コード 0 = 全て通った。
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/fe-lift-test.mjs');
const REAL_KNOWN = path.join(ROOT, 'lift/crates/fe-lift/tests/known-failures.txt');
const TESTS_DIR = path.join(ROOT, 'lift/crates/fe-lift/tests');
const { parseCargoOutput, loadKnown, judge, formatReport } = await import(pathToFileURL(TOOL).href);
const TMP = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-felift-'));

let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};

// 合成の出力: 1 つの binary の出力(cargo test の libtest の形)。rows は [名前, 'ok' | 'FAILED']。crash なら結果の行を出さず、abort の行だけを出す
function binary(label, rows, opt = {}) {
  const head = label === 'doc' ? '   Doc-tests fe-lift'
    : label === 'lib' ? '     Running unittests src/lib.rs (target/debug/deps/fe_lift-1)'
      : label === 'bin' ? '     Running unittests src/main.rs (target/debug/deps/fe_lift-2)'
        : `     Running tests/${label}.rs (target/debug/deps/${label}-3)`;
  const out = [head, `running ${rows.length} tests`];
  for (const [name, res] of rows) out.push(`test ${name} ... ${res}`);
  const fails = rows.filter(([, res]) => res === 'FAILED').map(([name]) => name);
  if (fails.length) {
    out.push('', 'failures:');
    for (const name of fails) out.push('', `---- ${name} stdout ----`, `thread '${name}' panicked at tests/${label}.rs:1:1:`, 'assertion failed: boom');
    out.push('', 'failures:', ...fails.map((name) => `    ${name}`));
  }
  if (!opt.crash) out.push(`test result: ${fails.length ? 'FAILED' : 'ok'}. ${rows.length - fails.length} passed; ${fails.length} failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`);
  if (fails.length || opt.crash) out.push(`error: test failed, to rerun pass \`--test ${label}\``, 'Caused by:', '  process didn\'t exit successfully: `target/debug/deps/x` (signal: 6, SIGABRT: process abort signal)');
  return out.join('\n') + '\n';
}
// 合成の出力: 1 件の build の失敗(cargo の行の形。試験の binary は走らない)
const buildFail = (item) => `error: could not compile \`fe-lift\` (${item}) due to 1 previous error\n`;
const run = (text, status = text.includes('FAILED') || text.includes('could not compile') ? 101 : 0) => ({ status, parsed: parseCargoOutput(text) });

const KNOWN_TEXT = '# 見出し\n\nwall_fr8 known_one\nwall_fr8 known_two\nwall_r gone\n';
const known = loadKnown(KNOWN_TEXT);

try {
  // 解析
  const p1 = parseCargoOutput(binary('wall_r', [['passes_a', 'ok'], ['fails_b', 'FAILED']]));
  ok('parse: 通った名前と失敗した名前を、binary ごとに数える', [p1.blocks.length, p1.blocks[0].label, [...p1.blocks[0].passed], [...p1.blocks[0].failed], p1.blocks[0].resultSeen], [1, 'wall_r', ['passes_a'], ['fails_b'], true]);
  const pd = parseCargoOutput(binary('doc', [['src/lib.rs - g (line 4)', 'FAILED']]));
  ok('parse: 文書の試験の名前は空白を含み、binary は doc', [pd.blocks[0].label, [...pd.blocks[0].failed]], ['doc', ['src/lib.rs - g (line 4)']]);
  const pl = parseCargoOutput(binary('lib', [['t::unit', 'ok']]) + binary('bin', [['u', 'ok']]));
  ok('parse: 単体試験の見出しは lib と bin になる', pl.blocks.map((b) => b.label), ['lib', 'bin']);
  const pb = parseCargoOutput(buildFail('test "wall_fr8"') + buildFail('lib test') + buildFail('bin "fe-lift" test'));
  ok('parse: build の失敗の対象を、binary の名前(lib・bin は既定の名)で取る', [...pb.builds.keys()].sort(), ['bin', 'lib', 'wall_fr8']);
  const pc = parseCargoOutput(binary('wall_e1', [['x', 'ok']], { crash: true }));
  ok('parse: 結果の行が無い binary は、解析の上で止まった(resultSeen が偽)', [pc.blocks[0].resultSeen, pc.blocks[0].label], [false, 'wall_e1']);

  // known の一覧
  ok('known: 実際の known-failures.txt は 59 件(#1416 で int_more2 の CPU に依る 1 件を足した。#1427 で wall_fr20 の 1 件が直った)', loadKnown(fs.readFileSync(REAL_KNOWN, 'utf8')).size, 59);
  ok('known: 見出し・空行を読まず、CRLF も扱う', [...loadKnown('# a\r\n\r\nwall_r x y\r\n')], ['wall_r x y']);

  // 判定
  const ranAll = (names) => names;
  const r1 = judge({ runs: [run(binary('wall_fr8', [['known_one', 'FAILED'], ['known_two', 'FAILED'], ['fine', 'ok']]))], known, expected: ranAll(['wall_fr8']) });
  ok('judge: 失敗が全て known なら終了 0。既知の数を数える', [r1.exitCode, r1.knownFailed, r1.newFailures.length], [0, 2, 0]);
  const r2 = judge({ runs: [run(binary('wall_r', [['new_one', 'FAILED']]))], known, expected: ['wall_r'] });
  ok('judge: known に無い失敗は新しい失敗(終了 1)', [r2.exitCode, r2.newFailures.map((f) => `${f.label} ${f.name}`)], [1, ['wall_r new_one']]);
  const r3 = judge({ runs: [run(binary('wall_r', [['known_one', 'FAILED']]))], known, expected: ['wall_r'] });
  ok('judge: 同じ名前でも binary が違えば別の試験(wall_fr8 の known は wall_r では効かない)', [r3.exitCode, r3.newFailures.length], [1, 1]);
  const r4 = judge({ runs: [run(binary('wall_r', [['gone', 'ok']]))], known, expected: ['wall_r'] });
  ok('judge: known の行の試験が通ったら「直った」。既定は終了 0', [r4.exitCode, r4.fixed.map((f) => `${f.label} ${f.name}`)], [0, ['wall_r gone']]);
  const r5 = judge({ runs: [run(binary('wall_r', [['gone', 'ok']]))], known, expected: ['wall_r'], strictKnown: true });
  ok('judge: --strict-known のとき、直った物があれば終了 1', r5.exitCode, 1);
  const r6 = judge({ runs: [run(buildFail('test "wall_fr8"'))], known, expected: ['wall_fr8'] });
  ok('judge: 既知の binary の build の失敗は許す(binary は走らない)', [r6.exitCode, r6.knownBuild, r6.newFailures.length], [0, ['wall_fr8'], 0]);
  const r7 = judge({ runs: [run(buildFail('test "wall_e1"'))], known, expected: ['wall_e1'] });
  ok('judge: known に無い binary の build の失敗は新しい失敗(終了 1)', [r7.exitCode, r7.newFailures.map((f) => `${f.kind}:${f.label}`)], [1, ['build:wall_e1']]);
  const r8 = judge({ runs: [run(binary('wall_fr8', [['known_one', 'FAILED']], { crash: true }))], known, expected: ['wall_fr8'] });
  ok('judge: 途中で止まった binary は、known の行があっても新しい失敗(終了 1)', [r8.exitCode, r8.newFailures.map((f) => f.kind)], [1, ['crash']]);
  const r9 = judge({ runs: [run(binary('wall_r', [['ok_a', 'ok']]), 0)], known, expected: ['wall_r', 'wall_e1'] });
  ok('judge: 実行されなかった試験の binary があれば終了 1(沈黙の合格を防ぐ)', [r9.exitCode, r9.problems.length, r9.problems[0].includes('wall_e1')], [1, 1, true]);
  const r10 = judge({ runs: [run('error: no test target named `zz`\n', 101)], known, expected: [] });
  ok('judge: 終了コードが 0 でなく、失敗の内訳が無い cargo の失敗は、終了 1', [r10.exitCode, r10.problems.length], [1, 1]);
  const r11 = judge({ runs: [run(binary('wall_fr8', [['known_one', 'FAILED']])), run('error: no test target named `zz`\n', 101)], known, expected: ['wall_fr8'] });
  ok('judge: 一方の実行の失敗が既知でも、もう一方の内訳が読めない失敗は隠れない', r11.exitCode, 1);
  const r12 = judge({ runs: [run(binary('wall_fr8', [['known_one', 'FAILED']])), run(binary('wall_r', [['ok_a', 'ok']]), 0)], known, expected: ['wall_fr8', 'wall_r'] });
  ok('judge: 複数の実行の結果を合わせる(既知の失敗は 1 件、通った件数は 1)', [r12.exitCode, r12.knownFailed, r12.passed], [0, 1, 1]);
  const rep = formatReport(r6, { known });
  ok('report: 既知のビルドの失敗は、その binary が走っていないと書く', rep.some((l) => l.includes('wall_fr8') && l.includes('走らない')), true);
  ok('report: 新しい失敗は、binary と試験の名前を書く', formatReport(r2, { known }).some((l) => l === '新しい失敗: wall_r new_one'), true);

  // CLI(偽の cargo)
  const FAKE_DIR = path.join(TMP, 'bin');
  fs.mkdirSync(FAKE_DIR, { recursive: true });
  fs.writeFileSync(path.join(FAKE_DIR, 'package.json'), '{"type":"commonjs"}\n');
  fs.writeFileSync(path.join(FAKE_DIR, 'cargo'), `#!${process.execPath}
const fs = require('fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.FAKE_LOG, JSON.stringify(args) + '\\n');
const cfg = JSON.parse(process.env.FAKE_CFG);
const sel = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--test') sel.push(args[++i]);
  else if (args[i] === '--lib') sel.push('lib');
  else if (args[i] === '--bin') { i++; sel.push('bin'); }
  else if (args[i] === '--doc') sel.push('doc');
}
const list = sel.length ? sel : cfg.all;
if (cfg.unexplained) { process.stderr.write('error: no test target named \`zz\`\\n'); process.exit(101); }
const item = (l) => l === 'lib' ? 'lib test' : l === 'bin' ? 'bin "fe-lift" test' : l === 'doc' ? 'doc' : 'test "' + l + '"';
const builds = list.filter((l) => (cfg.builds || []).includes(l));
if (builds.length) { for (const l of builds) process.stderr.write('error: could not compile \`fe-lift\` (' + item(l) + ') due to 1 previous error\\n'); process.exit(101); }
const header = (l) => l === 'doc' ? '   Doc-tests fe-lift' : l === 'lib' ? '     Running unittests src/lib.rs (target/x/lib)' : l === 'bin' ? '     Running unittests src/main.rs (target/x/bin)' : '     Running tests/' + l + '.rs (target/x/' + l + ')';
const out = [];
let bad = false;
for (const l of list) {
  if ((cfg.drop || []).includes(l)) continue;
  const fails = (cfg.fails && cfg.fails[l]) || [];
  const passes = (cfg.passes && cfg.passes[l]) || (cfg.all && cfg.all.includes(l) && !fails.length ? ['t_' + l] : []);
  out.push(header(l), 'running ' + (passes.length + fails.length) + ' tests');
  for (const n of passes) out.push('test ' + n + ' ... ok');
  for (const n of fails) out.push('test ' + n + ' ... FAILED');
  if (fails.length) { bad = true; out.push('', 'failures:'); for (const n of fails) out.push('', '---- ' + n + ' stdout ----', 'assertion failed'); out.push('', 'failures:', ...fails.map((n) => '    ' + n)); }
  out.push('test result: ' + (fails.length ? 'FAILED' : 'ok') + '. ' + passes.length + ' passed; ' + fails.length + ' failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s', '');
  if (fails.length) out.push('error: test failed, to rerun pass \`--test ' + l + '\`');
}
process.stdout.write(out.join('\\n') + '\\n');
process.exit(bad ? 101 : 0);
`, { mode: 0o755 });
  let caseNo = 0;
  const cli = (args, cfg, rows = ['wall_fr8 known_one', 'wall_fr8 known_two', 'wall_r gone'], hostRows = []) => {
    caseNo++;
    const known = path.join(TMP, `known-${caseNo}.txt`);
    const hostFile = path.join(TMP, `host-${caseNo}.txt`);
    const log = path.join(TMP, `log-${caseNo}.txt`);
    fs.writeFileSync(known, `${rows.join('\n')}\n`);
    fs.writeFileSync(hostFile, `${hostRows.join('\n')}\n`);
    fs.writeFileSync(log, '');
    const r = spawnSync(process.execPath, [TOOL, ...args, '--known', known, '--host-dependent', hostFile], { cwd: ROOT, encoding: 'utf8', env: { ...process.env, PATH: `${FAKE_DIR}:${process.env.PATH}`, FAKE_LOG: log, FAKE_CFG: JSON.stringify(cfg) } });
    const calls = fs.readFileSync(log, 'utf8').trim().split('\n').filter(Boolean).map((l) => JSON.parse(l));
    return { status: r.status, out: r.stdout, err: r.stderr, calls };
  };
  const a = cli(['--test', 'wall_fr8', '--test', 'wall_r'], { fails: { wall_fr8: ['known_one'] }, passes: { wall_r: ['ok_a'] } });
  ok('cli: 既知の失敗だけなら終了 0・既知の数を書く', [a.status, a.out.includes('既知の失敗 1 件')], [0, true]);
  const b = cli(['--test', 'wall_r', '--test', 'wall_e1'], { fails: { wall_r: ['new_one'] }, passes: { wall_e1: ['ok_b'] } });
  ok('cli: 新しい失敗があれば終了 1 で、その名前を書く', [b.status, b.out.includes('新しい失敗: wall_r new_one')], [1, true]);
  const c = cli(['--test', 'wall_fr8', '--test', 'wall_r'], { builds: ['wall_fr8'], passes: { wall_r: ['ok_a'] } });
  ok('cli: 既知の build の失敗は、残りの試験を --test で再実行し、終了 0', [c.status, c.calls.length, c.calls[1].includes('wall_fr8'), c.calls[1].includes('wall_r')], [0, 2, false, true]);
  ok('cli: 既知の build の失敗を、報告に書く', c.out.includes('既知のビルドの失敗'), true);
  const d = cli(['--test', 'wall_r', '--test', 'wall_e1'], { drop: ['wall_e1'], passes: { wall_r: ['ok_a'] } });
  ok('cli: 走らなかった試験があれば終了 1', [d.status, d.out.includes('実行されなかった')], [1, true]);
  const e = cli(['--test', 'wall_r', '--test', 'wall_e1'], { builds: ['wall_e1'], passes: { wall_r: ['ok_a'] } });
  ok('cli: known に無い build の失敗は終了 1。残りの試験は再実行して、走らせる', [e.status, e.calls.length, e.calls[1].join(' ')], [1, 2, ['test', '--locked', '--manifest-path', 'lift/Cargo.toml', '-p', 'fe-lift', '--no-fail-fast', '--test', 'wall_r'].join(' ')]);
  const f = cli(['--test', 'wall_r'], { unexplained: true });
  ok('cli: 失敗の内訳が読めない cargo の失敗は終了 1', [f.status, f.out.includes('失敗の内訳を読めない')], [1, true]);
  const g = cli(['--test', 'wall_r'], { passes: { wall_r: ['gone'] } }, ['wall_r gone']);
  ok('cli: 直った試験は既定で終了 0 で書く', [g.status, g.out.includes('直った: wall_r gone')], [0, true]);
  const h = cli(['--test', 'wall_r', '--strict-known'], { passes: { wall_r: ['gone'] } }, ['wall_r gone']);
  ok('cli: --strict-known では直った試験で終了 1', h.status, 1);
  const i = cli(['--bogus'], {});
  ok('cli: 知らない引数は終了 2', i.status, 2);
  const cfgAll = fs.readdirSync(TESTS_DIR).filter((n) => n.endsWith('.rs')).map((n) => n.slice(0, -3))
    .concat(fs.readdirSync(TESTS_DIR).filter((n) => fs.existsSync(path.join(TESTS_DIR, n, 'main.rs'))))
    .sort();
  const j = cli([], { all: [...cfgAll, 'lib', 'bin', 'doc'], passes: {} }, []);
  ok('cli: 全体の実行(選択なし)は、最初の 1 回だけで終わり、試験の名前を指定しない', [j.status, j.calls.length, j.calls[0].length], [0, 1, 7]);
  const k = cli([], { all: [...cfgAll, 'lib', 'bin', 'doc'], builds: ['wall_fr8'], passes: {} }, ['wall_fr8 known_one']);
  ok('cli: 全体の実行で build の失敗が既知なら、失敗の binary を除いて残りを再実行する(文書の試験は別の呼び出し)', [k.status, k.calls.length, k.calls[2].includes('--doc'), k.calls[1].includes('wall_fr8'), k.calls[1].includes('--doc')], [0, 3, true, false, false]);

  // host 依存の試験(tools/fe-lift-host-dependent.txt。CPU の種類で結果が変わる試験の一覧)
  const hostDep = loadKnown('# 注釈\n\nwall_r host_one\nwall_fr8 known_two\n', 'fe-lift-host-dependent');
  const hostFailedKeys = (r) => r.hostFailed.map((f) => `${f.label} ${f.name}`);
  const h1 = judge({ runs: [run(binary('wall_r', [['host_one', 'FAILED'], ['ok_a', 'ok']]))], known, hostDependent: hostDep, expected: ['wall_r'] });
  ok('host: host 依存の試験の失敗は、新しい失敗にも既知の失敗にも数えず、終了 0', [h1.exitCode, h1.newFailures.length, h1.knownFailed, hostFailedKeys(h1)], [0, 0, 0, ['wall_r host_one']]);
  const h2 = judge({ runs: [run(binary('wall_fr8', [['known_two', 'ok']]))], known, hostDependent: hostDep, expected: ['wall_fr8'], strictKnown: true });
  ok('host: 既知の一覧にも host 依存の一覧にも在る試験が通っても「直った」にせず、--strict-known でも終了 0', [h2.exitCode, h2.fixed.length, h2.hostFailed.length], [0, 0, 0]);
  const h3 = judge({ runs: [run(binary('wall_fr8', [['known_two', 'FAILED'], ['known_one', 'FAILED']]))], known, hostDependent: hostDep, expected: ['wall_fr8'] });
  ok('host: 既知の一覧と host 依存の一覧の両方に在る失敗は、host 依存として数える', [h3.exitCode, hostFailedKeys(h3), h3.knownFailed], [0, ['wall_fr8 known_two'], 1]);
  const h4 = judge({ runs: [run(binary('wall_r', [['host_one', 'FAILED'], ['new_one', 'FAILED']]))], known, hostDependent: hostDep, expected: ['wall_r'] });
  ok('host: 同じ binary でも一覧に無い失敗は、今まで通り新しい失敗(終了 1)', [h4.exitCode, h4.newFailures.map((f) => `${f.label} ${f.name}`), hostFailedKeys(h4)], [1, ['wall_r new_one'], ['wall_r host_one']]);
  const h5 = judge({ runs: [run(binary('wall_fr8', [['host_one', 'FAILED']]))], known, hostDependent: hostDep, expected: ['wall_fr8'] });
  ok('host: 試験の名前が同じでも binary が違えば別の試験(終了 1)', [h5.exitCode, h5.newFailures.length, hostFailedKeys(h5)], [1, 1, []]);
  const h6 = judge({ runs: [run(binary('wall_r', [['host_one', 'FAILED']], { crash: true }))], known, hostDependent: hostDep, expected: ['wall_r'] });
  ok('host: host 依存の失敗があっても、途中で止まった binary は新しい失敗(終了 1)', [h6.exitCode, h6.newFailures.map((f) => f.kind)], [1, ['crash']]);
  const h7 = judge({ runs: [run(binary('wall_r', [['host_one', 'FAILED']]))], known, expected: ['wall_r'] });
  ok('host: 一覧を渡さなければ、host 依存の失敗も新しい失敗(既定は空の一覧)', [h7.exitCode, h7.newFailures.length], [1, 1]);
  const rep1 = formatReport(h1, { known });
  ok('report: host 依存の失敗は件数と、binary と試験の名前を 1 行ずつ書く', [rep1.includes('ホスト依存の失敗: 1 件'), rep1.includes('    wall_r host_one')], [true, true]);
  ok('report: host 依存の失敗は「新しい失敗」の行に出ない', rep1.some((l) => l.startsWith('新しい失敗')), false);
  ok('report: host 依存の失敗が無くても、件数を 0 件と書く', formatReport(judge({ runs: [run(binary('wall_r', [['ok_a', 'ok']]))], known, expected: ['wall_r'] }), { known }).includes('ホスト依存の失敗: 0 件'), true);
  ok('report: host 依存の試験が通っても「直った」と書かない', formatReport(h2, { known }).some((l) => l.startsWith('直った')), false);
  ok('known: 試験名の無い行は、読んだ一覧の名前を付けて止まる', (() => { try { loadKnown('wall_r\n', 'fe-lift-host-dependent'); return false; } catch (e) { return e.message.includes('fe-lift-host-dependent の行に試験名が無い'); } })(), true);

  const REAL_HOST = path.join(ROOT, 'tools/fe-lift-host-dependent.txt');
  const realHostText = fs.existsSync(REAL_HOST) ? fs.readFileSync(REAL_HOST, 'utf8') : '';
  const realHost = loadKnown(realHostText, 'fe-lift-host-dependent');
  ok('host: 実際の一覧は 8 件(新しい失敗 7 件と int_more2 の 1 件)', realHost.size, 8);
  ok('host: 先頭の注釈に、門を落とさない旨を書く', realHostText.includes('失敗しても門を落とさない。通れば、それでよい'), true);
  const missingTests = [...realHost].filter((key) => {
    const [label, name] = key.split(' ');
    const file = [path.join(TESTS_DIR, `${label}.rs`), path.join(TESTS_DIR, label, 'main.rs')].find((f) => fs.existsSync(f));
    return !file || !fs.readFileSync(file, 'utf8').includes(`fn ${name}(`);
  });
  ok('host: 一覧の試験は、どれも lift/crates/fe-lift/tests/ の binary に在る(名前の書き違いで効かないのを防ぐ)', missingTests, []);

  const hm = spawnSync(process.execPath, [TOOL, '--host-dependent'], { cwd: ROOT, encoding: 'utf8' });
  ok('cli: --host-dependent に値が無ければ終了 2', hm.status, 2);
  const hc1 = cli(['--test', 'wall_r'], { fails: { wall_r: ['host_one'] } }, [], ['wall_r host_one']);
  const lineOf = (out, prefix) => out.split('\n').some((l) => l.startsWith(prefix));
  ok('cli: host 依存の失敗だけなら終了 0 で、件数と名前を書き、「新しい失敗」の行は出さない', [hc1.status, hc1.out.includes('ホスト依存の失敗: 1 件'), hc1.out.includes('    wall_r host_one'), lineOf(hc1.out, '新しい失敗')], [0, true, true, false]);
  const hc2 = cli(['--test', 'wall_fr8', '--strict-known'], { passes: { wall_fr8: ['known_two'] } }, ['wall_fr8 known_two'], ['wall_fr8 known_two']);
  ok('cli: host 依存の試験が通っても、--strict-known で終了 1 にせず、「直った」の行も出さない', [hc2.status, lineOf(hc2.out, '直った')], [0, false]);
} finally {
  fs.rmSync(TMP, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
