#!/usr/bin/env node
// Generates the plugin scenarios and the feature rows (Issue #1493; the marks and the insertion: Issue #1508). Run it from the repository (no arguments):
//   node tools/e2e/scenarios/plugins/_generate.mjs
// Inputs: tools/e2e/features/generated/plugins.tsv (one row per built-in plugin, external plugin and tool: from inputs/plugin-classes.tsv and
// inputs/external-plugins.tsv) and pm-work/created-issues.tsv (the Issue numbers that the marks name; the title is the key).
// Writes, one file per plugin (a plugin is one file; nothing is typed by hand per plugin):
//   builtin-<slug>.mjs    the four steps (window, parameter, audio, engine stop) of each built-in plugin (Effects and Generators)
//   external-<slug>.mjs   the same for each external plugin (Serum 2, OTT, Vital)
//   position-<slug>.mjs   the plugin is put into Mixer Insert 1 from the Plugin database, and its window lies inside the desktop (the displays add a random DPR)
// and tools/e2e/features/plugins.tsv (the feature rows: window, param, audio, stop of each plugin; the tools' and presets' rows, written here).
// The mark (markOf): the first open batch Issue of the table that lists the built-in plugin; the external plugins: #1324; otherwise #1535 (the Plugin database
// draws no row). A closed Issue is never a mark: CLOSED_ISSUES lists the closed ones (update it when an Issue is closed). A lift folder does not decide the mark (#1508).
// The hand-written scenarios of this area (the picker, the Plugin database, the Plugin manager, the presets) are not generated; they are plain files.
// A generated file is removed first when its plugin is no longer in the list (the names of the three prefixes only).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { FEATURE_KINDS, builtinSource, positionSource, kindOf, batchIssuesNaming, pluginRows } from '../../lib/plugins.mjs';

/** the closed Issues (#1508): their numbers are never a mark. Listed here as numbers, in the generator (update it when an Issue is closed). */
const CLOSED_NUMBERS = [1315, 1319, 1336, 1337, 1338, 1400, 1484];
export const CLOSED_ISSUES = CLOSED_NUMBERS.map((n) => `#${n}`);
/** the mark of a plugin whose Plugin database entry is not drawn, with no open batch Issue that lists it (#1508): the classification data (#1535) */
export const INSERT_MARK = '#1535';
/** the mark of the external plugins (Serum 2, OTT, Vital): the open Feature #1324 names the three (#1508) */
export const EXTERNAL_MARK = '#1324';

const STEP_TEXT = {
  window: ['窓が開く(窓の表のクラス・題名に、プラグインの名前が出る)', 'Mixer の挿入 1 の Slot 1 → picker → Plugin database の項目'],
  param: ['パラメータを 1 つ変える(窓の画素の SHA-256 が変わる)', '窓の中を縦にドラッグする(40 画素)'],
  audio: ['音声を処理する(Generators は再生中の非無音の量子が 0 より大きい)', 'Space で再生し、audio.detail().nonSilentQuanta を 2 秒見る'],
  stop: ['engine が止まらない(engine-idle のまま)', '手順の後の engine の状態'],
};

/** the feature rows of the hand-written tools and presets (kind, name, expected, how) */
const TOOL_FEATURES = [
  ['plugins/window/mixer-slot-picker', 'window', 'Mixer の挿入スロットの picker', 'picker の drop-down が開く(More plugins・Plugin database・表示の切替)', 'Insert 1 を 2 回押し(遅い二度押し) → Slot 1 を押す'],
  ['plugins/window/plugin-database', 'window', 'Plugin database(Browser の窓)', 'picker の Plugin database の項目で、Browser が Plugin database を出し、engine が Installed を読む', 'picker の項目(DB_ITEM)'],
  ['plugins/list/plugin-database-entries', 'list', 'Plugin database の件数', 'Installed の一覧の件数が出る(engine の enum-dir の行)', 'Plugin database を開いた後の engine のログ'],
  ['plugins/select/plugin-database-item', 'select', 'Plugin database の項目を選ぶ', '項目を選ぶと、Browser の画素が変わる', 'Installed の行を押す'],
  ['plugins/window/plugin-manager', 'window', 'Plugin Manager の窓', 'Options の項目で PluginManager.exe が起動し、Plugin Manager の窓が出る(#1499)', 'Options の項目を順に選ぶ'],
  ['plugins/stop/plugin-manager', 'stop', 'Plugin Manager の起動後の engine', 'Plugin Manager を起動した後、10 秒 engine-idle のまま', 'Options の項目の後'],
  ['plugins/preset/sytrus-load', 'preset', 'Sytrus のプリセットの読み込み', 'プリセットを読み込み、パラメータの値が変わる', 'Plugin database の Sytrus の項目'],
  ['plugins/preset/fruity-balance-load', 'preset', 'Fruity Balance のプリセットの読み込み', 'プリセットを読み込み、パラメータの値が変わる', 'Plugin database の Fruity Balance の項目'],
  ['plugins/preset/serum-2-load', 'preset', 'Serum 2 のプリセットの読み込み', 'プリセットを読み込み、パラメータの値が変わる', 'Plugin database の Serum 2 の項目'],
];

const KIND_TAG = { effects: 'effects-', generators: 'generators-', external: '' };

/** the mark of a plugin: the first open batch Issue of the table that lists it (batchIssuesNaming), else INSERT_MARK (#1535: the entry is not drawn).
 *  A closed Issue (`closed`) is never the mark. A lift folder (lift/plugins/<名前>/) does not decide it: the Plugin database draws no row in this build (#1508).
 *  The external plugins take EXTERNAL_MARK (#1324). */
export function markOf({ name, kind, issuesTsv, closed = CLOSED_ISSUES }) {
  if (kind === 'external') return EXTERNAL_MARK;
  const open = batchIssuesNaming(name, kind, issuesTsv).filter((n) => !closed.includes(n));
  return open[0] ?? INSERT_MARK;
}

function main() {
  const here = path.dirname(fileURLToPath(import.meta.url));
  const e2e = path.resolve(here, '..', '..');
  const repo = path.resolve(e2e, '..', '..');
  const generatedTsv = fs.readFileSync(path.join(e2e, 'features', 'generated', 'plugins.tsv'), 'utf8');
  const issuesTsv = fs.readFileSync(path.join(repo, 'pm-work', 'created-issues.tsv'), 'utf8');
  const rows = pluginRows(generatedTsv).filter((r) => kindOf(r) !== null).map((r) => ({
    id: r.id, kind: kindOf(r), slug: KIND_TAG[kindOf(r)] + r.id.split('/').pop(), name: r.name, dll: r.how, source: r.source,
  }));
  const seen = new Set();
  for (const r of rows) {
    if (seen.has(r.slug)) throw new Error(`two plugins share the slug ${r.slug}`);
    seen.add(r.slug);
  }

  fs.mkdirSync(here, { recursive: true });
  for (const f of fs.readdirSync(here)) if (/^(builtin|external|position)-.*\.mjs$/.test(f)) fs.rmSync(path.join(here, f));

  const feature = [];
  const marks = { batch: 0, insert: 0, external: 0 };
  let written = 0;
  for (const r of rows) {
    const external = r.kind === 'external';
    const mark = markOf({ name: r.name, kind: r.kind, issuesTsv });
    if (mark === INSERT_MARK) marks.insert++;
    else if (mark === EXTERNAL_MARK) marks.external++;
    else marks.batch++;
    const covers = [r.id, ...FEATURE_KINDS.map((k) => `plugins/${k}/${r.slug}`)];
    const prefix = external ? 'external' : 'builtin';
    fs.writeFileSync(path.join(here, `${prefix}-${r.slug}.mjs`), builtinSource({ id: `plugins/${prefix}-${r.slug}`, slug: r.slug, name: r.name, kind: r.kind, mark, dll: r.dll, covers }));
    fs.writeFileSync(path.join(here, `position-${r.slug}.mjs`), positionSource({ id: `plugins/position-${r.slug}`, name: r.name, mark, covers: [`plugins/window/${r.slug}`] }));
    written += 2;
    for (const k of FEATURE_KINDS) {
      feature.push([`plugins/${k}/${r.slug}`, k, r.name, STEP_TEXT[k][0], STEP_TEXT[k][1]]);
    }
  }
  const tsv = [['id', 'kind', 'name', 'expected', 'how'], ...feature, ...TOOL_FEATURES.map((t) => [t[0], t[1], t[2], t[3], t[4]])];
  fs.writeFileSync(path.join(e2e, 'features', 'plugins.tsv'), `${tsv.map((c) => c.join('\t')).join('\n')}\n`);
  process.stdout.write(`plugins: ${rows.length} (effects ${rows.filter((r) => r.kind === 'effects').length}, generators ${rows.filter((r) => r.kind === 'generators').length}, external ${rows.filter((r) => r.kind === 'external').length}); files ${written}; feature rows ${tsv.length - 1}\n`);
  process.stdout.write(`marks: batch Issue ${marks.batch}, ${INSERT_MARK} ${marks.insert}, ${EXTERNAL_MARK} ${marks.external}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
