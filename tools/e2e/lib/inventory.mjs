// The lists of the features that the repository holds (Issue #1485): the plugins, the File menu's files, the languages and themes, the page's modules and the lazy modules of the bundle.
// Pure: the texts of the input files in, the rows out. A row is { id, kind, name, source, how, nameSource }: id is `<領域>/<種類>/<名前の slug>` (unique in the whole inventory),
// nameSource is `memory` (the name is read from the data the product holds: a file list, the window table, the guest's memory) or `index` (the name is the item's place, `#n`).
import { slug } from './common.mjs';

export const TSV_HEAD = ['id', 'kind', 'name', 'source', 'how', 'nameSource'];

/** a TSV line of a row (tabs and line breaks in the cells become spaces) */
export function tsvLine(row) {
  return TSV_HEAD.map((k) => String(row[k] ?? '').replace(/[\t\r\n]+/g, ' ')).join('\t');
}

/** the rows as a TSV text (the header line first) */
export function toTsv(rows) {
  return `${TSV_HEAD.join('\t')}\n${rows.map(tsvLine).join('\n')}${rows.length ? '\n' : ''}`;
}

/** an ID that is unique in `seen` (a Map of the IDs given so far): the second is `<id>-2`, and so on */
export function uniqueId(id, seen) {
  if (!seen.has(id)) { seen.set(id, 1); return id; }
  let n = seen.get(id) + 1;
  while (seen.has(`${id}-${n}`)) n++;
  seen.set(id, n);
  seen.set(`${id}-${n}`, 1);
  return `${id}-${n}`;
}

/** the rows of a TSV text (a header line, `#` lines skipped) as objects */
export function tsvRows(text) {
  const lines = String(text).split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
  if (lines.length === 0) return [];
  const head = lines[0].split('\t');
  return lines.slice(1).map((l) => {
    const cells = l.split('\t');
    return Object.fromEntries(head.map((h, i) => [h, cells[i] ?? '']));
  });
}

/** the plugins: the main plugins (本体) of inputs/plugin-classes.tsv (Effects, Generators), the external plugins (inputs/external-plugins.tsv), and the two tools */
export function pluginRows({ classes, external }) {
  const seen = new Map();
  const rows = [];
  const add = (row) => { rows.push({ ...row, id: uniqueId(row.id, seen) }); };
  for (const r of tsvRows(classes)) {
    if (r['本体か補助か'] !== '本体') continue;
    const path = r['パス'];
    const kind = r['種別'];
    const rel = path.replace(/^Plugins\\Fruity\\(Effects|Generators)\\/, '').replace(/\.dll$/i, '');
    add({ id: `plugins/${kind.toLowerCase()}/${slug(rel)}`, kind: `main-${kind.toLowerCase()}`, name: rel.split('\\')[0], source: `inputs/plugin-classes.tsv ${path}`, how: `DLL ${path.split('\\').pop()} (${r['大きさ']} B)`, nameSource: 'memory' });
  }
  for (const r of tsvRows(external)) {
    const name = r.plugin;
    add({ id: `plugins/external/${slug(name)}`, kind: 'external', name, source: 'inputs/external-plugins.tsv', how: `${r.path} (${r.bytes} B)`, nameSource: 'memory' });
  }
  add({ id: 'plugins/tool/plugin-database', kind: 'tool', name: 'Plugin database', source: 'the Add menu / the plugin list (the page has its own window)', how: 'window of the database (the menu item, see menus.tsv)', nameSource: 'index' });
  add({ id: 'plugins/tool/plugin-manager', kind: 'tool', name: 'Plugin manager', source: 'the Options or Tools menu (see menus.tsv)', how: 'the Plugin Manager application (its window opens on the item)', nameSource: 'index' });
  return rows;
}

/** the File menu's files: the templates (Data\Templates\*.flp), the demo songs and projects (Data\Demo projects\*.flp) of inputs/fl-installer-files.tsv, and the File drop-down's items of menus.tsv */
export function fileRows({ installer, menuRows = [] }) {
  const seen = new Map();
  const rows = [];
  const add = (row) => { rows.push({ ...row, id: uniqueId(row.id, seen) }); };
  for (const r of tsvRows(installer)) {
    const g = r.guest;
    if (!/\.flp$/i.test(g)) continue;
    if (g.startsWith('Data\\Templates\\')) {
      const rel = g.slice('Data\\Templates\\'.length).replace(/\.flp$/i, '');
      add({ id: `file/template/${slug(rel)}`, kind: 'template', name: g.split('\\').pop().replace(/\.flp$/i, ''), source: `inputs/fl-installer-files.tsv ${g}`, how: `Data\\Templates, ${r.bytes} B`, nameSource: 'memory' });
    } else if (g.startsWith('Data\\Demo projects\\')) {
      const rel = g.slice('Data\\Demo projects\\'.length).replace(/\.flp$/i, '');
      add({ id: `file/demo/${slug(rel)}`, kind: 'demo', name: g.split('\\').pop().replace(/\.flp$/i, ''), source: `inputs/fl-installer-files.tsv ${g}`, how: `Data\\Demo projects, ${r.bytes} B`, nameSource: 'memory' });
    }
  }
  for (const m of menuRows) {
    if (!m.name.startsWith('File > ')) continue;
    add({ id: `file/menu-item/${slug(m.name)}`, kind: 'menu-item', name: m.name, source: 'menus.tsv (the File drop-down)', how: m.how, nameSource: m.nameSource });
  }
  return rows;
}

/** the languages (System\Languages\<code>.moe) and the themes (Artwork\Themes\<name>.flstheme, one theme per file, in any folder) of inputs/fl-installer-files.tsv */
export function resourceRows({ installer }) {
  const seen = new Map();
  const rows = [];
  const add = (row) => { rows.push({ ...row, id: uniqueId(row.id, seen) }); };
  const themes = new Set();
  for (const r of tsvRows(installer)) {
    const g = r.guest;
    let m = /^System\\Languages\\([a-z]{2})\.moe$/i.exec(g);
    if (m) {
      add({ id: `settings/language/${slug(m[1])}`, kind: 'language', name: m[1].toLowerCase(), source: `inputs/fl-installer-files.tsv ${g}`, how: `System\\Languages, ${r.bytes} B`, nameSource: 'memory' });
      continue;
    }
    const t = /^Artwork\\Themes\\(?:.+\\)?([^\\]+)\.flstheme$/i.exec(g);
    if (t) themes.add(t[1]);
  }
  for (const name of [...themes].sort()) {
    add({ id: `settings/theme/${slug(name)}`, kind: 'theme', name, source: `inputs/fl-installer-files.tsv Artwork\\Themes\\${name}.flstheme`, how: 'the .flstheme file of the theme (its name is the file name)', nameSource: 'memory' });
  }
  return rows;
}

/** the page's modules (app/src): each function of the page, with the file that holds it (checked against `exists`) */
export function pageRows({ exists }) {
  const mods = [
    ['clipboard', 'the clipboard (copy and paste text between the guest and the browser)', 'app/src/engine/clipboard.ts'],
    ['cursor', 'the guest cursor drawn by the page', 'app/src/engine/cursor-picture.ts'],
    ['file-select', 'the file box and the file manager of the Open and Save', 'app/src/files/select.ts'],
    ['persist', 'the stored copy of the bundle and the registry (kept across a reload)', 'app/src/engine/regstore.ts'],
    ['audio-start', 'the audio output: the AudioContext and its worklet', 'app/src/audio/page-audio.ts'],
    ['resize', 'the picture follows the window (the guest screen of the page size)', 'app/src/engine/desk.ts'],
    ['beacon', 'the measurement beacons of the start and the run', 'app/src/perf/beacon.ts'],
    ['delivery', 'the bundle files (manifest, lazy modules, the engine) from the delivery', 'app/src/bundle/loader.ts'],
  ];
  const seen = new Map();
  return mods.map(([name, desc, file]) => ({
    id: uniqueId(`page/module/${slug(name)}`, seen), kind: 'module', name: desc, source: file, how: exists(file) ? 'the file is in the repository' : 'the file is NOT in the repository', nameSource: 'index',
  }));
}

/** the lazy modules of the bundle's manifest (manifest.lazy): one per module (path, size, the SHA-256 of the manifest) */
export function deliveryRows({ manifest }) {
  const seen = new Map();
  return (manifest.lazy ?? []).map((l) => ({
    id: uniqueId(`delivery/lazy/${slug(l.path)}`, seen), kind: 'lazy', name: l.path, source: 'the bundle manifest.json (lazy)', how: `${l.size} B, sha256 ${String(l.sha256).slice(0, 16)}`, nameSource: 'memory',
  }));
}

/** the summary: the total, and each area's count */
export function summarize(rowsByFile) {
  const perArea = {};
  let total = 0;
  const unnamed = {};
  for (const rows of Object.values(rowsByFile)) {
    for (const r of rows) {
      const area = r.id.split('/')[0];
      perArea[area] = (perArea[area] ?? 0) + 1;
      total++;
      if (r.nameSource === 'index') unnamed[area] = (unnamed[area] ?? 0) + 1;
    }
  }
  return { total, perArea, unnamed };
}
