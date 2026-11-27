// The built-in plugins of the smoke scenario plugin-database-lists-builtins (Issues #1484, #1429): a plugin is a folder of lift/plugins/ whose plugin.tsv has a 本体
// row; a folder of helpers only (補助, e.g. reverb's Shared\Reverb_x64.dll) has no Plugin database entry (lift/crates/fe-build/src/plugins.rs plugin_db_names).
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { builtinPlugins, builtinPluginsOf } from '../scenarios/smoke/plugin-database-lists-builtins.mjs';

const HEAD = 'key\tguest\trole\tsha256\tsize\thook\targs\tomit\n';
const SHA = 'a'.repeat(64);
const row = (key, guest, role) => `${key}\t${guest}\t${role}\t${SHA}\t10\tnone\t--pdata\t-\n`;
const main = (name) => row(`effects.x.${name.toLowerCase().replace(/ /g, '-')}_x64.dll`, `Plugins\\Fruity\\Effects\\${name}\\${name}_x64.dll`, '本体');
const helper = () => row('reverb_x64.dll', 'Shared\\Reverb_x64.dll', '補助');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const readTsv = (folder) => fs.readFileSync(path.join(repo, 'lift', 'plugins', folder, 'plugin.tsv'), 'utf8');

test('a folder with a 本体 row and a 補助 row is one plugin, named by the folder of its 本体', () => {
  assert.deepEqual(builtinPluginsOf([{ folder: 'x', text: HEAD + main('Alpha') + helper() }]), [{ folder: 'x', name: 'Alpha' }]);
});

test('a folder of helpers only has no plugin entry and does not throw', () => {
  assert.deepEqual(builtinPluginsOf([{ folder: 'reverb', text: HEAD + helper() }]), []);
});

test('a plugin.tsv with the header only (no row) is no plugin and does not throw', () => {
  assert.deepEqual(builtinPluginsOf([{ folder: 'empty', text: HEAD }]), []);
});

test('the blank lines, the # comment lines and CRLF line ends are not rows', () => {
  const text = `# a comment\r\n\r\n${HEAD.trimEnd()}\r\n${main('Beta').trimEnd()}\r\n`;
  assert.deepEqual(builtinPluginsOf([{ folder: 'beta', text }]), [{ folder: 'beta', name: 'Beta' }]);
});

test('a folder with two 本体 rows is one entry (the Plugin database has one entry per plugin)', () => {
  assert.deepEqual(builtinPluginsOf([{ folder: 'two', text: HEAD + main('Alpha') + main('Beta') }]), [{ folder: 'two', name: 'Alpha' }]);
});

test('the plugins are counted in the order of the files, the helper-only folder is left out', () => {
  const files = [{ folder: 'a', text: HEAD + main('Alpha') }, { folder: 'h', text: HEAD + helper() }, { folder: 'b', text: HEAD + main('Beta') + helper() }];
  assert.deepEqual(builtinPluginsOf(files).map((p) => p.name), ['Alpha', 'Beta']);
});

test('the real plugin.tsv files give the entries plugins.rs plugin_db_names gives: Fruity Balance and Sytrus (reverb is a helper)', () => {
  const files = ['fruity-balance', 'sytrus', 'reverb'].map((folder) => ({ folder, text: readTsv(folder) }));
  assert.deepEqual(builtinPluginsOf(files).map((p) => p.name), ['Fruity Balance', 'Sytrus']);
});

test('the real lift/plugins: reverb has no entry, and Sytrus is named by its folder', () => {
  const plugins = builtinPlugins(repo);
  assert.equal(plugins.some((p) => p.folder === 'reverb'), false);
  assert.ok(plugins.some((p) => p.folder === 'sytrus' && p.name === 'Sytrus'));
});
