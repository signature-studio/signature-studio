// The menus' scenarios (Issue #1489): one file per scenario, written from the item lists by this file. Run it from the repository:
//   node tools/e2e/scenarios/menus/_generate.mjs
// The lists: features/generated/menus.tsv (the menu bar's items, from E0) and features/menus.tsv (the submenus' items). The facts below are what the crash-hunt
// (pm-work/findings/crash-hunt-2026-11-25.json, DPR 1 and 1.7) saw: a dialog (a window that is not a drop-down), a submenu (its item count), a stop. They only choose
// which variants are written. What each scenario finds is its own result (the run's ok and results), and a mark (expectFail) is set only for a stop seen in a run.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseTsv } from '../../coverage.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const featuresDir = path.resolve(here, '..', '..', 'features');

/** E0's samples in this folder (kept by hand, not written by this file) */
const SAMPLES = ['file-popup-opens', 'edit-popup-opens', 'down-moves-highlight'];

/** the menu bar's labels, in the order of the bar */
const LABELS = ['File', 'Edit', 'Add', 'Patterns', 'View', 'Options', 'Tools', 'Help'];

/** the items that open a dialog (a window that is not a drop-down) at DPR 1: the OK, Cancel and Esc variants and the repeat are written for each */
const DIALOG = [
  'file-1', 'file-3', 'file-4', 'file-5', 'file-6', 'file-7',
  'file-2-1', 'file-2-2', 'file-2-4', 'file-2-5', 'file-8-1', 'file-9-7', 'file-9-8',
  'add-1', 'add-6', 'add-10', 'add-12', 'add-13', 'add-5-1', 'add-5-5',
  'patterns-1', 'patterns-2', 'patterns-3', 'patterns-6', 'patterns-7', 'patterns-10', 'patterns-11', 'patterns-12', 'patterns-14', 'patterns-17', 'patterns-18', 'patterns-19', 'patterns-20',
  'view-2', 'view-6', 'view-7', 'view-8', 'view-9', 'view-10',
];

/** the items with a submenu, and its number of items (the crash-hunt) */
const SUBMENU = { 'file-2': 5, 'file-8': 1, 'file-9': 13, 'file-9-9': 2, 'file-9-10': 2, 'add-5': 5, 'view-11': 18, 'view-19': 7 };

/** the stops, with the Issue of the defect (crash-hunt: #1415, #1416, #1411), or 'new' for a stop seen in a run with no Issue yet (tools-4: the riff machine window; #1429 is the same window with another address, so not the same defect as far as the run shows). add-14 has a DPR 1.7 choice without the mark */
const STOP = { 'file-9-11': '#1415', 'view-11-1': '#1416', 'add-14': '#1411', 'tools-4': 'new' };

/** the items whose effect is seen at DPR 1.7 only (the crash-hunt saw none at DPR 1): their scenarios run on the random display */
const randomOnly = (slug) => ['view-12', 'view-13', 'view-14'].includes(slug) || slug.startsWith('view-19');

/** the items with no scenario: Exit ends the engine, and the run counts an exit as a stop (lib/detect.mjs), so the item cannot be judged */
const NO_SCENARIO = { 'file-11': 'File > #11 (Exit): the engine exits, and the run counts an exit as a stop (lib/detect.mjs). Issue #1521: Manage files is #10 (after Export, before Exit)' };

/** File > Manage files (Issue #1521): the item between Revert to last backup and Exit, and the OPFS file window it opens. Both at DPR 1 and DPR 2 (lib/menus.mjs) */
const MANAGE_FILES = [
  { name: 'manage-files-order', fn: 'manageFilesOrder', desc: 'File > Manage files: the item is in the File drop-down between Revert to last backup and Exit (menus/manage-files-order).' },
  { name: 'manage-files-opens-window', fn: 'manageFilesOpens', desc: 'File > Manage files: the choice opens the OPFS file window (menus/manage-files-opens-window).' },
];

/** the shortcuts. `map` is the menu item that the key duplicates, where the run showed it (F8 opens the same window as View > #6; Ctrl+N opens the Save as window, as File > #4) */
const KEYS = [
  { slug: 'f5', label: 'F5', press: "await g.key('F5', 'F5', 116);" },
  { slug: 'f6', label: 'F6', press: "await g.key('F6', 'F6', 117);" },
  { slug: 'f7', label: 'F7', press: "await g.key('F7', 'F7', 118);" },
  { slug: 'f8', label: 'F8', press: "await g.key('F8', 'F8', 119);", map: ['View', '#6'] },
  { slug: 'f9', label: 'F9', press: "await g.key('F9', 'F9', 120);" },
  { slug: 'f10', label: 'F10', press: "await g.key('F10', 'F10', 121);" },
  { slug: 'f11', label: 'F11', press: "await g.key('F11', 'F11', 122);" },
  { slug: 'f12', label: 'F12', press: "await g.key('F12', 'F12', 123);" },
  { slug: 'alt-f8', label: 'Alt+F8', press: "await altChord(g, 'F8', 'F8', 119);", alt: true },
  { slug: 'ctrl-n', label: 'Ctrl+N', press: "await g.ctrl('KeyN', 'n', 78);", map: ['File', '#4'] },
  { slug: 'ctrl-o', label: 'Ctrl+O', press: "await g.ctrl('KeyO', 'o', 79);" },
  { slug: 'ctrl-s', label: 'Ctrl+S', press: "await g.ctrl('KeyS', 's', 83);" },
  { slug: 'ctrl-p', label: 'Ctrl+P', press: "await g.ctrl('KeyP', 'p', 80);" },
  { slug: 'ctrl-i', label: 'Ctrl+I', press: "await g.ctrl('KeyI', 'i', 73);" },
  { slug: 'ctrl-e', label: 'Ctrl+E', press: "await g.ctrl('KeyE', 'e', 69);" },
  { slug: 'ctrl-f', label: 'Ctrl+F', press: "await g.ctrl('KeyF', 'f', 70);" },
  { slug: 'space', label: 'Space', press: "await g.key('Space', ' ', 32);", restore: "await g.key('Space', ' ', 32);" },
];

const DISPLAY_DPR1 = "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)";
const DISPLAY_RANDOM = 'displays: 4k-random (the crash-hunt saw this effect at DPR 1.7, none at DPR 1)';

/** an item from a row of a menus.tsv: its slug (the ID's last part), the names (`File > #9 > #7` as ['File', '#9', '#7']) */
const entry = (row) => ({ slug: row.id.replace('menus/item/', ''), names: row.name.split(' > ') });

/** every item of the menu bar and of the submenus, in the order of the files */
function itemsOf() {
  const gen = parseTsv(fs.readFileSync(path.join(featuresDir, 'generated', 'menus.tsv'), 'utf8')).filter((r) => r.kind === 'item');
  const sub = parseTsv(fs.readFileSync(path.join(featuresDir, 'menus.tsv'), 'utf8'));
  return [...gen, ...sub].map(entry);
}

/** the scenarios: { name, covers, displays, expectFail, desc, body, imports, note } (the body is the run's code) */
function scenariosOf() {
  const items = itemsOf();
  const slugs = new Set(items.map((i) => i.slug));
  for (const d of [...DIALOG, ...Object.keys(SUBMENU), ...Object.keys(STOP), ...Object.keys(NO_SCENARIO)]) {
    if (!slugs.has(d)) throw new Error(`the facts name ${d}, which is not an item of the lists`);
  }
  const out = [];
  const add = (s) => out.push(s);
  for (const it of items) {
    if (NO_SCENARIO[it.slug]) continue;
    const names = JSON.stringify(it.names);
    const dpr = randomOnly(it.slug) ? ['4k-random'] : ['4k-dpr1'];
    const note = randomOnly(it.slug) ? DISPLAY_RANDOM : DISPLAY_DPR1;
    const mark = STOP[it.slug] ?? null;
    const stopNote = mark ? [`crash-hunt: the engine stops on this item (${mark})`] : [];
    const base = { covers: [`menus/item/${it.slug}`], displays: dpr, expectFail: mark, note, stopNote, names };
    add({ ...base, name: `${it.slug}-choose`, imports: ['chooseItem'], body: `return withNote(await chooseItem(ctx, ${names}), ${JSON.stringify(note)});`, desc: `${it.names.join(' > ')} (menus/item/${it.slug}): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.` });
    if (it.slug === 'add-14') {
      add({ ...base, name: 'add-14-random', covers: [`menus/item/${it.slug}`], displays: ['4k-random'], expectFail: null, note: DISPLAY_RANDOM, stopNote: [], imports: ['chooseItem'], body: `return withNote(await chooseItem(ctx, ${names}), ${JSON.stringify(DISPLAY_RANDOM)});`, desc: `${it.names.join(' > ')} (menus/item/${it.slug}) at DPR 1.7 (the random display): the choice shows its window (the same item stops the engine at DPR 1: add-14-choose).` });
    }
    if (DIALOG.includes(it.slug)) {
      for (const [how, label] of [['ok', 'OK (Enter)'], ['cancel', 'Cancel (Escape)'], ['esc', 'Esc (a click on the caption, then Escape)']]) {
        add({ ...base, name: `${it.slug}-${how}`, imports: ['dialogAnswer'], body: `return withNote(await dialogAnswer(ctx, ${names}, '${how}'), ${JSON.stringify(note)});`, desc: `${it.names.join(' > ')} opens a window: it is closed by ${label}, and it is gone from the window table after.` });
      }
      add({ ...base, name: `${it.slug}-again`, imports: ['chooseTwice'], body: `return withNote(await chooseTwice(ctx, ${names}), ${JSON.stringify(note)});`, desc: `${it.names.join(' > ')}: the same choice twice (the window closed by Escape between them): the same kind of effect both times.` });
    }
    if (SUBMENU[it.slug]) {
      add({ ...base, name: `${it.slug}-arrow-opens`, expectFail: null, stopNote: [], imports: ['arrowOpens'], body: `return withNote(await arrowOpens(ctx, ${names}), ${JSON.stringify(note)});`, desc: `${it.names.join(' > ')} (${SUBMENU[it.slug]} items in the crash-hunt): ArrowRight on the item opens its submenu (a new drop-down).` });
      add({ ...base, name: `${it.slug}-submenu-escape`, expectFail: null, stopNote: [], imports: ['submenuEscape'], body: `return withNote(await submenuEscape(ctx, ${names}), ${JSON.stringify(note)});`, desc: `${it.names.join(' > ')}: Escape closes only its submenu; the drop-down of its parent stays open.` });
    }
  }
  for (const l of LABELS) {
    const slug = l.toLowerCase();
    add({ name: `${slug}-menu-opens`, covers: [`menus/menu/${slug}`], displays: ['4k-dpr1', '4k-random'], expectFail: null, note: 'displays: 4k-dpr1 and 4k-random (the default)', stopNote: [], imports: ['labelOpens'], body: `return withNote(await labelOpens(ctx, '${l}'), 'displays: 4k-dpr1 and 4k-random (the default)');`, desc: `${l}: a click on the label opens its drop-down (its item count is read from the E0 list).` });
    add({ name: `${slug}-menu-escape`, covers: [`menus/menu/${slug}`], displays: ['4k-dpr1', '4k-random'], expectFail: null, note: 'displays: 4k-dpr1 and 4k-random (the default)', stopNote: [], imports: ['labelEscape'], body: `return withNote(await labelEscape(ctx, '${l}'), 'displays: 4k-dpr1 and 4k-random (the default)');`, desc: `${l}: Escape closes its drop-down (none is left open).` });
  }
  for (let i = 0; i + 1 < LABELS.length; i++) {
    const from = LABELS[i], to = LABELS[i + 1];
    add({ name: `${from.toLowerCase()}-to-${to.toLowerCase()}-switch`, covers: [`menus/menu/${to.toLowerCase()}`], displays: ['4k-dpr1'], expectFail: null, note: DISPLAY_DPR1, stopNote: [], imports: ['labelSwitch'], body: `return withNote(await labelSwitch(ctx, '${from}', '${to}'), ${JSON.stringify(DISPLAY_DPR1)});`, desc: `${from} open, then a click on ${to}: the first drop-down closes; a second click on ${to} opens its drop-down, to the right of ${from}'s.` });
  }
  for (const k of KEYS) {
    const note = DISPLAY_DPR1;
    const alt = k.alt ? ['altChord'] : [];
    if (k.map) {
      const mapped = `${k.map[0].toLowerCase()}-${k.map[1].slice(1)}`;
      add({ name: `key-${k.slug}`, covers: [`menus/item/${mapped}`], displays: ['4k-dpr1'], expectFail: null, note, stopNote: [], imports: ['keyAndMenu', ...alt], body: `return withNote(await keyAndMenu(ctx, '${k.label}', async (g) => { ${k.press} }, ${JSON.stringify(k.map)}), ${JSON.stringify(note)});`, desc: `${k.label} and ${k.map.join(' > ')} (menus/item/${mapped}): the two paths give the same kind of effect (a window, a submenu, a changed window or the title).` });
    } else {
      const restore = k.restore ? `async (g) => { ${k.restore} }` : 'null';
      add({ name: `key-${k.slug}`, covers: [], displays: ['4k-dpr1'], expectFail: null, note, stopNote: [], imports: ['shortcut', ...alt], body: `return withNote(await shortcut(ctx, '${k.label}', async (g) => { ${k.press} }, ${restore}), ${JSON.stringify(note)});`, desc: `${k.label}: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.` });
    }
  }
  for (const m of MANAGE_FILES) {
    const note = 'displays: 4k-dpr1 and 4k-dpr2 (the item is read at both)';
    add({ name: m.name, covers: [], displays: ['4k-dpr1', '4k-dpr2'], expectFail: null, note, stopNote: [], imports: [m.fn], body: `return withNote(await ${m.fn}(ctx), ${JSON.stringify(note)});`, desc: m.desc });
  }
  return out;
}

/** the file's text */
function render(s) {
  const id = `menus/${s.name}`;
  const imports = [...new Set([...s.imports, 'withNote'])].sort();
  const lines = [
    `// ${id}: ${s.desc}`,
    ...(s.stopNote && s.stopNote.length ? s.stopNote.map((n) => `// ${n}`) : []),
    `import { defineScenario } from '../../lib/scenario.mjs';`,
    `import { ${imports.join(', ')} } from '../../lib/menus.mjs';`,
    '',
    'export default defineScenario({',
    `  id: '${id}',`,
    `  covers: [${s.covers.map((c) => `'${c}'`).join(', ')}],`,
    `  displays: [${s.displays.map((d) => `'${d}'`).join(', ')}],`,
    `  expectFail: ${s.expectFail ? `'${s.expectFail}'` : 'null'},`,
    '  async run(ctx) {',
    `    ${s.body}`,
    '  },',
    '});',
    '',
  ];
  return lines.join('\n');
}

function main() {
  const list = scenariosOf();
  const names = new Set();
  for (const s of list) {
    if (names.has(s.name)) throw new Error(`two scenarios named ${s.name}`);
    names.add(s.name);
  }
  // the three samples of E0 (file-popup-opens, edit-popup-opens, down-moves-highlight) are not written here: they stay as they are
  for (const f of fs.readdirSync(here)) if (f.endsWith('.mjs') && !f.startsWith('_') && !SAMPLES.includes(f.slice(0, -4))) fs.rmSync(path.join(here, f));
  for (const s of list) fs.writeFileSync(path.join(here, `${s.name}.mjs`), render(s));
  process.stdout.write(`${list.length} scenarios written to ${path.relative(process.cwd(), here) || '.'}\n`);
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) main();

export { scenariosOf, DIALOG, SUBMENU, STOP, KEYS, NO_SCENARIO };
