// The plugins' steps (Issue #1493; the insertion and the observations: Issue #1508): the shared parts of the plugin scenarios (scenarios/plugins/builtin-*.mjs, external-*.mjs, position-*.mjs, and the tool and preset scenarios).
// A plugin is put in a Mixer insert slot from the Plugin database: a slow double-click selects Insert 1 (its panel with Slot 1..10 appears at the right edge of the Mixer window);
// a click on Slot 1 opens the picker (More plugins..., Plugin database, Categories / Simple / Tree); the Plugin database item opens the Browser's Plugin database (Installed).
// The places are fractions of the Mixer window's size (measured at 4k-dpr1: the window is 3436 x 500 guest px). The UI is drawn at the display's scale, so the places follow the window.
// The insertion is not reachable in this build (#1508, measured at 4k-dpr1): the Installed folder lists names (the engine's enum-dir line) but draws no row, and the
// picker's More plugins... list (Select effect plugin) shows 0 effects. insertFromDatabase reads the rows from the picture and stops there (no row, no entry).
// Pure parts (the row and the source of a generated scenario) come first; the runtime parts use the page's reads (window table, engine log, picture) and real mouse and key events.
import { windowTable, openLids, newWindows, pixelHashRegion, guestText } from './common.mjs';
import { absoluteRect } from './dump.mjs';
import { sleep } from './flow.mjs';
import { ensureMixer } from './input.mjs';

export const FEATURE_KINDS = ['window', 'param', 'audio', 'stop'];
export const INSERT1 = { fx: 125 / 3436, fy: 72 / 500 };
export const SLOT1 = { fx: 3252 / 3436, fy: 70 / 500 };
export const DB_ITEM = { dx: 60, dy: 30 };
/** the Installed row of the Browser's tree, and the area under it where its rows are drawn (guest px at 4k-dpr1) */
export const INSTALLED = { x: 70, y: 183 };
export const INSTALLED_LIST = { x: 0, y: 200, w: 372, h: 160 };
export const POPUP = 'TQuickPopupMenuWindow';
export const MIXER = 'TFXForm';
export const DB_TEXT = /Plugin database\\Installed/;
const ENUM_INSTALLED = /enum-dir: (\d+) ".*Plugin database\\Installed\\\*\.\*"/;

/** the built-in and external rows of the generated plugins table (features/generated/plugins.tsv): main-effects, main-generators, external; `tool` rows are not plugins */
export function pluginRows(tsvText) {
  const lines = String(tsvText).split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
  const head = lines[0].split('\t');
  return lines.slice(1).map((l) => Object.fromEntries(l.split('\t').map((c, i) => [head[i], c]))).filter((r) => /^plugins\//.test(r.id));
}

/** the plugin's kind for the scenario: effects, generators, or external (null for the tools) */
export function kindOf(row) {
  if (row.kind === 'main-effects') return 'effects';
  if (row.kind === 'main-generators') return 'generators';
  if (row.kind === 'external') return 'external';
  return null;
}

/** the Issue numbers (`#n`) of the table's batch rows that list a built-in plugin, in the table's order: `内蔵エフェクト N: a・b を…` (effects) and
 *  `内蔵インストゥルメント N: …` (generators). A row that names one plugin only is not a batch. Closed Issues are included: the caller leaves them out */
export function batchIssuesNaming(name, kind, issuesTsv) {
  const rows = String(issuesTsv).split(/\r?\n/).slice(1).map((l) => l.split('\t')).filter((f) => f.length >= 3);
  const batch = kind === 'effects' ? /内蔵エフェクト \d+: (.*?) を/ : kind === 'generators' ? /内蔵インストゥルメント \d+: (.*?) を/ : null;
  if (!batch) return [];
  const listed = (title) => {
    const m = batch.exec(title);
    return m !== null && m[1].split('・').map((s) => s.trim()).includes(name);
  };
  return rows.filter((f) => listed(f[2])).map((f) => `#${f[1].replace('#', '')}`);
}

/** the rectangle of the Mixer window (guest px), or null when it is not shown */
export async function mixerRect(guest) {
  const t = await windowTable(guest);
  const m = t.children.find((c) => c.cls === MIXER && c.vis);
  return m ? absoluteRect(t, m.lid) : null;
}

/** two clicks in a row on a guest px (a slow double-click: the engine reads the second press as a double-click) */
export async function slowDoubleClick(guest, gx, gy) {
  const c = await guest.pointAt(gx, gy);
  await guest.move(c);
  for (const k of [1, 2]) {
    await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
    await sleep(80);
    await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
    await sleep(150);
  }
  await sleep(1200);
}

/** the newest drop-down (the picker) after Slot 1 of Insert 1 is clicked; null when no picker opened */
export async function openSlotPicker(guest) {
  await guest.focusDesktop();
  await guest.escape();
  await sleep(500);
  await ensureMixer(guest);
  const r = await mixerRect(guest);
  if (!r) return { error: 'the Mixer (TFXForm) is not shown', pop: null };
  await slowDoubleClick(guest, r.x + Math.round(INSERT1.fx * r.w), r.y + Math.round(INSERT1.fy * r.h));
  const base = await openLids(guest);
  await guest.click(r.x + Math.round(SLOT1.fx * r.w), r.y + Math.round(SLOT1.fy * r.h));
  await sleep(1200);
  const { pops } = await newWindows(guest, base);
  return { error: null, pop: pops.at(-1) ?? null };
}

/** opens the Plugin database from the picker (its item): the Browser shows Plugin database > Installed; returns whether the picker was there */
export async function openPluginDatabase(guest) {
  const { pop, error } = await openSlotPicker(guest);
  if (error || !pop) return { opened: false, error: error ?? 'no picker' };
  await guest.click(pop.x + DB_ITEM.dx, pop.y + DB_ITEM.dy);
  await sleep(2000);
  return { opened: true, error: null };
}

/** the engine's lines about the Plugin database's Installed folder (the listings and the missing paths), from the line `from` on: the scenario's start (the log is kept for the whole page) */
export async function databaseLines(guest, from) {
  if (!Number.isInteger(from)) throw new Error('databaseLines needs the engine log length at the start of the scenario (from)');
  const lines = await guest.ev(`window.__app.engineLog().slice(${from})`);
  return lines.filter((l) => DB_TEXT.test(l));
}

/** the number of names the Installed folder's listing returned (the largest enum-dir count of the lines); null when there is no listing */
export function installedCount(lines) {
  const counts = lines.map((l) => ENUM_INSTALLED.exec(l)).filter(Boolean).map((m) => Number(m[1]));
  return counts.length ? Math.max(...counts) : null;
}

/** the number of names the Installed folder's category folder (Effects or Generators) listed: its enum-dir line (the largest count); null when there is no listing (Issue #1535) */
export function categoryCount(lines, category) {
  const re = new RegExp(`enum-dir: (\\d+) ".*Plugin database\\\\Installed\\\\${category}\\\\\\*\\.\\*"`, 'i');
  const counts = lines.map((l) => re.exec(l)).filter(Boolean).map((m) => Number(m[1]));
  return counts.length ? Math.max(...counts) : null;
}

/** the plugin's entry in the Plugin database: the engine's lines name it (its folder or its file under Installed). The engine's lines have no names (#1508): a false here says nothing */
export function databaseHas(lines, name) {
  return lines.some((l) => l.includes(name) && DB_TEXT.test(l));
}

/** whether the Installed folder draws rows: the folder is selected (a click on its row) and opened (Right); the rows are drawn under it, so the picture there changes */
export async function installedRowsShown(guest) {
  const before = await pixelHashRegion(guest, INSTALLED_LIST);
  await guest.click(INSTALLED.x, INSTALLED.y);
  await sleep(400);
  await guest.key('ArrowRight', 'ArrowRight', 39);
  await sleep(1200);
  const after = await pixelHashRegion(guest, INSTALLED_LIST);
  return before !== after;
}

/** opens the Installed folder (its row: a click, then Right) in the Browser's Plugin database, and waits for its listing (Issue #1535) */
export async function openInstalled(guest) {
  await guest.click(INSTALLED.x, INSTALLED.y);
  await sleep(400);
  await guest.key('ArrowRight', 'ArrowRight', 39);
  await sleep(1500);
}

/** whether "Directory is empty!" is in the guest's memory (guestText). The engine's text table holds it whether or not the hint line shows it: a true here says nothing (Issue #1535) */
export async function hintSaysEmpty(guest) {
  return (await guestText(guest, ['Directory is empty!'])).best !== null;
}

/**
 * puts a plugin of the Plugin database into the selected Mixer insert (#1508): the picker's Plugin database item opens the Browser's Installed folder, and the
 * entry is chosen on its row. The rows are read from the picture, and a name only from a drawn row. Returns { ok, notes, results }; results.insert and results.window
 * are written after the choice (null when there is none). In this build the rows are not drawn: ok is false and the notes say so.
 */
export async function insertFromDatabase(guest, name) {
  const from = await guest.ev('window.__app.engineLog().length');
  const db = await openPluginDatabase(guest);
  const results = { count: null, named: { [name]: false }, insert: null, window: null };
  if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`], results };
  const lines = await databaseLines(guest, from);
  results.count = installedCount(lines);
  const rows = await installedRowsShown(guest);
  results.named = { [name]: rows };
  const listed = results.count === null ? 'no listing of Installed in the engine log since the scenario started (the listing is written once per page)' : `the Installed folder lists ${results.count} name(s) (the engine's enum-dir line)`;
  const notes = [listed, `the rows under Installed: ${rows ? 'drawn' : 'not drawn'}`];
  if (!rows) return { ok: false, notes: [...notes, `no entry for ${name} can be chosen: the Plugin database draws no row`], results };
  return { ok: false, notes: [...notes, `the row of ${name} is not located (its place is not measured yet)`], results };
}

/** the plugin's window in the window table (a top-level window or a child, by its title or caption naming the plugin); null when there is none */
export async function pluginWindow(guest, name) {
  const t = await windowTable(guest);
  const top = t.top.find((w) => w.title.includes(name));
  if (top) return { cls: top.cls, title: top.title, rect: { x: top.x, y: top.y, width: top.w, height: top.h } };
  const child = t.children.find((c) => c.vis && c.text.includes(name));
  if (child) { const r = absoluteRect(t, child.lid); return { cls: child.cls, title: child.text, rect: r }; }
  return null;
}

/** the audio's non-silent quanta while the output plays for `ms` (Space starts the play; the count is audio.detail().nonSilentQuanta) */
export async function nonSilentWhilePlaying(guest, ms) {
  const d0 = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
  if (d0 === null) return { available: false, nonSilent: 0 };
  await guest.key('Space', ' ', 32);
  await sleep(ms);
  const d1 = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
  await guest.key('Space', ' ', 32);
  return { available: true, nonSilent: d1.nonSilentQuanta - d0.nonSilentQuanta, quanta: d1.quantaWhilePlaying - d0.quantaWhilePlaying };
}

/**
 * the steps of a plugin (the scenario's run): the plugin is put into the insert from the Plugin database (insertFromDatabase: stops here when no entry can be
 * chosen); the window opens (by the plugin's name); a parameter changes (a drag inside the window: the picture's hash of the window changes); the audio is processed
 * (Generators: non-silent quanta while playing; Effects: the output's picture of the chain is not checked here, notes only); the engine stays engine-idle.
 * Returns { ok, engineState, notes, results }. The steps after the insertion run only when the entry was chosen, and the ones after the window only when the window opened.
 */
export async function pluginSteps(guest, { name, kind }) {
  const ins = await insertFromDatabase(guest, name);
  const notes = [...ins.notes];
  const results = { ...ins.results, window: null, param: null, audio: null, stop: null };
  if (!ins.ok) return { ok: false, engineState: (await guest.state())?.state ?? null, notes, results };
  const w = await pluginWindow(guest, name);
  results.window = w ? { cls: w.cls, title: w.title } : null;
  if (!w) {
    notes.push(`no window named ${name} after the entry was chosen`);
    return { ok: false, engineState: (await guest.state())?.state ?? null, notes, results };
  }
  const cx = w.rect.x + Math.round(w.rect.width * 0.3), cy = w.rect.y + Math.round(w.rect.height * 0.5);
  const before = await pixelHashRegion(guest, { x: w.rect.x, y: w.rect.y, w: w.rect.width, h: w.rect.height });
  await guest.drag({ x: cx, y: cy }, { x: cx, y: cy - 40 }, 10);
  await sleep(600);
  const after = await pixelHashRegion(guest, { x: w.rect.x, y: w.rect.y, w: w.rect.width, h: w.rect.height });
  results.param = { changed: before !== after };
  const audio = await nonSilentWhilePlaying(guest, 2000);
  results.audio = audio;
  if (kind === 'generators') notes.push(`non-silent quanta while playing: ${audio.nonSilent}`);
  const state = (await guest.state())?.state ?? null;
  results.stop = { engineState: state };
  const audioOk = kind === 'generators' ? audio.nonSilent > 0 : true;
  return { ok: results.param.changed && audioOk && state === 'engine-idle', engineState: state, notes, results };
}

/** the picture of a plugin's window position at the display (the window is in the desktop): used by the position scenarios */
export async function pluginInDesktop(guest, name) {
  const w = await pluginWindow(guest, name);
  if (!w) return { inside: false, notes: [`no window named ${name}`] };
  const d = await guest.desk();
  const inside = w.rect.x >= 0 && w.rect.y >= 0 && w.rect.x + w.rect.width <= d.gw && w.rect.y + w.rect.height <= d.gh;
  return { inside, notes: [`${w.cls} at ${w.rect.x},${w.rect.y} ${w.rect.width}x${w.rect.height} (desktop ${d.gw}x${d.gh})`] };
}

/** the scenario source of one plugin (the generated file's text): the same shape for every plugin; the names and the marks are the only parts that differ */
export function builtinSource({ id, slug, name, kind, mark, dll, covers }) {
  const cov = covers.map((c) => `'${c}'`).join(', ');
  return `// ${id}: ${name} (${kind}, ${dll}) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: '${id}',
  covers: [${cov}],
  expectFail: ${mark === null ? 'null' : `'${mark}'`},
  async run({ guest }) {
    return pluginSteps(guest, { name: ${JSON.stringify(name)}, kind: '${kind}' });
  },
});
`;
}

/** the position scenario's source: the plugin is put into the insert from the Plugin database, and its window lies inside the desktop at the display's DPR (the random DPR is added by displays) */
export function positionSource({ id, name, mark, covers }) {
  const cov = covers.map((c) => `'${c}'`).join(', ');
  return `// ${id}: the window of ${name} lies inside the desktop, after the plugin is put into Mixer Insert 1 from the Plugin database (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: '${id}',
  covers: [${cov}],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: ${mark === null ? 'null' : `'${mark}'`},
  async run({ guest }) {
    const ins = await insertFromDatabase(guest, ${JSON.stringify(name)});
    if (!ins.ok) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ins.notes, results: ins.results };
    const r = await pluginInDesktop(guest, ${JSON.stringify(name)});
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: [...ins.notes, ...r.notes], results: ins.results };
  },
});
`;
}
