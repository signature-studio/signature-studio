// The inventory's pure parts (lib/inventory.mjs) and the common helpers that its ids and menu paths use (lib/common.mjs): no browser.
import test from 'node:test';
import assert from 'node:assert/strict';
import { deliveryRows, fileRows, pageRows, pluginRows, resourceRows, summarize, toTsv, uniqueId } from '../lib/inventory.mjs';
import { itemIndices, menuLabelPlace, slug } from '../lib/common.mjs';

test('uniqueId appends -2, -3 to a repeated id', () => {
  const seen = new Map();
  assert.deepEqual(['a/b', 'a/b', 'a/b', 'a/c'].map((id) => uniqueId(id, seen)), ['a/b', 'a/b-2', 'a/b-3', 'a/c']);
});

test('toTsv writes the header and one line per row, without tabs inside a cell', () => {
  const t = toTsv([{ id: 'x/y/z', kind: 'k', name: 'a\tb', source: 's', how: 'h', nameSource: 'index' }]);
  assert.equal(t, 'id\tkind\tname\tsource\thow\tnameSource\nx/y/z\tk\ta b\ts\th\tindex\n');
});

test('plugins: the main plugins only, the externals and the two tools', () => {
  const classes = 'パス\t大きさ\tSHA-256\t種別\t本体か補助か\n'
    + 'Plugins\\Fruity\\Effects\\Fruity Balance\\Fruity Balance_x64.dll\t1\t\tEffects\t本体\n'
    + 'Plugins\\Fruity\\Effects\\Distructor\\Module_x64.dll\t1\t\tEffects\t補助\n'
    + 'Plugins\\Fruity\\Generators\\Sytrus\\Sytrus_x64.dll\t1\t\tGenerators\t本体\n';
  const external = 'plugin\tpath\tbytes\tsha256\timports\nOTT\tOTT.vst3\t1\t\t\n';
  const rows = pluginRows({ classes, external });
  assert.deepEqual(rows.map((r) => r.id), [
    'plugins/effects/fruity-balance-fruity-balance-x64',
    'plugins/generators/sytrus-sytrus-x64',
    'plugins/external/ott',
    'plugins/tool/plugin-database',
    'plugins/tool/plugin-manager',
  ]);
  assert.equal(rows.length, 5);
});

test('file: the templates and the demo projects (.flp), and the File drop-down items of the menus', () => {
  const installer = 'guest\tbytes\tsha256\tkind\timports\n'
    + 'Data\\Templates\\Minimal\\Empty\\Empty.flp\t1\t\tother\t\n'
    + 'Data\\Templates\\Minimal\\Empty\\Empty.nfo\t1\t\tother\t\n'
    + 'Data\\Demo projects\\Demo songs\\Song.flp\t1\t\tother\t\n';
  const menus = [{ id: 'menus/item/file-1', name: 'File > #1', how: 'Down x 1', nameSource: 'index' }, { id: 'menus/item/edit-1', name: 'Edit > #1', how: '', nameSource: 'index' }];
  const rows = fileRows({ installer, menuRows: menus });
  assert.deepEqual(rows.map((r) => r.id), ['file/template/minimal-empty-empty', 'file/demo/demo-songs-song', 'file/menu-item/file-1']);
});

test('resources: the languages (.moe) and the themes (one per .flstheme file, in any folder)', () => {
  const installer = 'guest\tbytes\tsha256\tkind\timports\n'
    + 'System\\Languages\\ja.moe\t1\t\tother\t\n'
    + 'System\\Languages\\ja.svg\t1\t\tother\t\n'
    + 'Artwork\\Themes\\Dark.flstheme\t1\t\tother\t\n'
    + 'Artwork\\Themes\\DEDZII\\DEDZII.flstheme\t1\t\tother\t\n'
    + 'Artwork\\Themes\\DEDZII\\DEDZII Light.flstheme\t1\t\tother\t\n'
    + 'Artwork\\Themes\\DEDZII\\x.png\t1\t\tother\t\n'
    + 'Artwork\\Themes\\thmDefault.png\t1\t\tother\t\n';
  assert.deepEqual(resourceRows({ installer }).map((r) => r.id), ['settings/language/ja', 'settings/theme/dedzii', 'settings/theme/dedzii-light', 'settings/theme/dark']);
});

test('page and delivery rows: the page modules with their files, the lazy modules of the manifest', () => {
  const page = pageRows({ exists: (p) => p !== 'app/src/perf/beacon.ts' });
  assert.equal(page.length, 8);
  assert.equal(page.find((r) => r.id === 'page/module/beacon').how, 'the file is NOT in the repository');
  const d = deliveryRows({ manifest: { lazy: [{ path: 'fruity_filter_x64.7000000.dll', size: 3, sha256: 'abcdef0123456789ff' }] } });
  assert.deepEqual(d.map((r) => r.id), ['delivery/lazy/fruity-filter-x64-7000000-dll']);
});

test('summarize counts the rows per area and the ones named by an index', () => {
  const s = summarize({ a: [{ id: 'menus/menu/file', nameSource: 'index' }, { id: 'menus/item/file-1', nameSource: 'index' }], b: [{ id: 'page/module/x', nameSource: 'index' }] });
  assert.deepEqual(s, { total: 3, perArea: { menus: 2, page: 1 }, unnamed: { menus: 2, page: 1 } });
});

test('slug and the menu helpers', () => {
  assert.equal(slug('Browser extra search folders'), 'browser-extra-search-folders');
  assert.equal(slug('!!'), 'x');
  assert.deepEqual(itemIndices('menus/item/file-3-2'), [3, 2]);
  assert.equal(itemIndices('menus/menu/file'), null);
  assert.deepEqual(menuLabelPlace({ x: 4, y: 4, w: 293, h: 33 }, 'File'), { x: 4 + 0.051 * 293, y: 4 + 0.45 * 33 });
  assert.throws(() => menuLabelPlace({ x: 0, y: 0, w: 1, h: 1 }, 'Nope'), /menu label/);
});
