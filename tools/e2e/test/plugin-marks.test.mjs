// The marks and the observations of the plugin scenarios (Issue #1508): a built-in plugin takes the open batch Issue that lists it, else the insertion mark #1535
// (the Plugin database draws no row: #1535); the external plugins take #1324; a closed Issue is never a mark, and no scenario file of plugins/ marks one or 'new'.
// Every position scenario puts its plugin into the insert first. The generator's functions (scenarios/plugins/_generate.mjs) are pure: importing it does not write the files.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { CLOSED_ISSUES, INSERT_MARK, EXTERNAL_MARK, markOf } from '../scenarios/plugins/_generate.mjs';
import { installedCount, batchIssuesNaming } from '../lib/plugins.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..', '..');
const issuesTsv = fs.readFileSync(path.join(repo, 'pm-work', 'created-issues.tsv'), 'utf8');
const pluginsDir = path.join(repo, 'tools', 'e2e', 'scenarios', 'plugins');
const CLOSED_MARK = new RegExp(`expectFail: .#(${CLOSED_ISSUES.map((n) => n.slice(1)).join('|')}).`);
const titleOf = (n) => (issuesTsv.split(/\r?\n/).map((l) => l.split('\t')).find((f) => f[1] === n) ?? [])[2] ?? '';

test('the closed Issues are the seven numbers that are closed (#1315 #1319 #1336 #1337 #1338 #1400 #1484)', () => {
  assert.deepEqual([...CLOSED_ISSUES].sort(), ['#1315', '#1319', '#1336', '#1337', '#1338', '#1400', '#1484']);
});

test('a built-in plugin takes the open batch Issue that lists it (its effects or instruments batch)', () => {
  assert.equal(markOf({ name: 'Fruity Center', kind: 'effects', issuesTsv }), '#1438');
  assert.equal(markOf({ name: 'Fruity Wrapper', kind: 'effects', issuesTsv }), '#1451');
  assert.equal(markOf({ name: 'Fruity Wrapper', kind: 'generators', issuesTsv }), '#1471');
  assert.equal(markOf({ name: 'Wasp XT', kind: 'generators', issuesTsv }), '#1477');
  assert.equal(markOf({ name: 'Edison', kind: 'effects', issuesTsv }), '#1435');
});

test('a closed batch Issue is never the mark: Fruity Center takes #1438 while it is open, and the insertion mark once #1438 is closed', () => {
  assert.equal(markOf({ name: 'Fruity Center', kind: 'effects', issuesTsv, closed: ['#1438'] }), INSERT_MARK);
});

test('a row that names one plugin is not a batch: Fruity Balance (#1342 splits its code), Fruity Filter (#1319 closed) and Sytrus take the insertion mark #1535', () => {
  assert.match(titleOf('#1342'), /Fruity Balance を/);
  assert.equal(markOf({ name: 'Fruity Balance', kind: 'effects', issuesTsv }), INSERT_MARK);
  assert.equal(markOf({ name: 'Fruity Filter', kind: 'effects', issuesTsv }), INSERT_MARK);
  assert.equal(markOf({ name: 'Fruity Delay 2', kind: 'effects', issuesTsv }), INSERT_MARK);
  assert.equal(markOf({ name: 'Sytrus', kind: 'generators', issuesTsv }), INSERT_MARK);
});

test('the external plugins take the open Feature #1324, which names Serum 2, OTT and Vital (not the closed #1336-#1338)', () => {
  assert.match(titleOf('#1324'), /Serum 2・OTT・Vital/);
  for (const name of ['Serum 2', 'OTT', 'Vital']) assert.equal(markOf({ name, kind: 'external', issuesTsv }), EXTERNAL_MARK);
  assert.equal(EXTERNAL_MARK, '#1324');
  assert.equal(CLOSED_ISSUES.includes(EXTERNAL_MARK), false);
});

test('the insertion mark is an open Task of the table: #1535 is the Plugin database row (the classification data)', () => {
  assert.equal(INSERT_MARK, '#1535');
  assert.match(titleOf('#1535'), /Plugin database の Installed/);
  assert.equal(CLOSED_ISSUES.includes(INSERT_MARK), false);
});

test('a name the table does not name takes the insertion mark #1535, not new', () => {
  assert.equal(markOf({ name: 'No Such Plugin', kind: 'effects', issuesTsv }), INSERT_MARK);
});

test('batchIssuesNaming lists the batch rows that list a plugin, in the table order (closed ones included)', () => {
  assert.deepEqual(batchIssuesNaming('Fruity Center', 'effects', issuesTsv), ['#1438']);
  assert.deepEqual(batchIssuesNaming('Fruity Balance', 'effects', issuesTsv), []);
  assert.deepEqual(batchIssuesNaming('Serum 2', 'external', issuesTsv), []);
});

test('installedCount reads the number of names of the Installed listing (the engine line; the largest of the lines)', () => {
  const line = '[err] enum-dir: 3 "C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Presets\\Plugin database\\Installed\\*.*"';
  assert.equal(installedCount([line, '[err] missing-path: GetFileAttributesW 2 "C:\\x\\Plugin database\\Installed.nfo"']), 3);
  assert.equal(installedCount([line, line.replace('enum-dir: 3', 'enum-dir: 2')]), 3);
  assert.equal(installedCount(['[err] enum-dir: 3 "C:\\x\\Plugin database\\Effects\\*.*"']), null);
  assert.equal(installedCount([]), null);
});

test('no scenario file of plugins/ marks a closed Issue (the generated and the hand-written files)', () => {
  const bad = fs.readdirSync(pluginsDir).filter((f) => f.endsWith('.mjs')).filter((f) => CLOSED_MARK.test(fs.readFileSync(path.join(pluginsDir, f), 'utf8')));
  assert.deepEqual(bad, []);
});

test("no generated or preset scenario of plugins/ takes the mark 'new' (an insertion failure is #1535, an external one #1324)", () => {
  const bad = fs.readdirSync(pluginsDir).filter((f) => /^(builtin|external|position|preset)-.*\.mjs$/.test(f)).filter((f) => /expectFail: ['"]new['"]/.test(fs.readFileSync(path.join(pluginsDir, f), 'utf8')));
  assert.deepEqual(bad, []);
});

test('the hand-written presets take the marks of their plugin: Fruity Balance and Sytrus #1535, Serum 2 #1324', () => {
  const mark = (f) => (fs.readFileSync(path.join(pluginsDir, f), 'utf8').match(/expectFail: '([^']+)'/) ?? [])[1];
  assert.equal(mark('preset-fruity-balance-load.mjs'), INSERT_MARK);
  assert.equal(mark('preset-sytrus-load.mjs'), INSERT_MARK);
  assert.equal(mark('preset-serum-2-load.mjs'), EXTERNAL_MARK);
});

test('no hand-written scenario of plugins/ reads the Plugin database log from the start of the page (databaseLines needs the start)', () => {
  const bad = fs.readdirSync(pluginsDir).filter((f) => f.endsWith('.mjs') && !f.startsWith('_')).filter((f) => /databaseLines\(guest\)/.test(fs.readFileSync(path.join(pluginsDir, f), 'utf8')));
  assert.deepEqual(bad, []);
});

test('every position scenario puts its plugin into the insert from the Plugin database before it looks for the window', () => {
  const files = fs.readdirSync(pluginsDir).filter((f) => /^position-.*\.mjs$/.test(f));
  assert.ok(files.length > 100, `${files.length} position scenarios`);
  const bad = files.filter((f) => {
    const s = fs.readFileSync(path.join(pluginsDir, f), 'utf8');
    return !/insertFromDatabase\(guest/.test(s) || s.indexOf('insertFromDatabase(guest') > s.indexOf('pluginInDesktop(guest');
  });
  assert.deepEqual(bad, []);
});
