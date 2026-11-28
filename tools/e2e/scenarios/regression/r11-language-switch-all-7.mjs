// R11 (#1408) language-switch-all-7. Report (L-063, #1383, #1395): the language is changed in Settings > General (the restart prompt "Language change" is
// correct); after the page is reloaded the chosen language is kept. Expectation, for each of the 7 languages in turn (de, es, fr, ja, ko, vi, zh): the General
// page's Language list chooses it; the engine writes it (window-API trace: RegSetValueExW "Program language"), the restart prompt appears; after the reload the
// engine is idle and reads the same language back (RegQueryValueExW "Program Language" -> found, the same value).
import { table, tops, under, settingsPageOf, newTops, startDefault, openSettings, engineLines, traceOn, kOf, sleep, result } from './common.mjs';
import { waitStable } from '../../lib/flow.mjs';

const SCENARIO = {
  name: 'language-switch-all-7',
  report: 'R11',
  expect: 'Settings > General > Language: each of de, es, fr, ja, ko, vi, zh chosen in turn; the restart prompt appears; after the reload the engine is idle and reads the same language back',
  // #1395 (open): the engine stops after the reload (the start in a language other than English, the symptom of the Issue's title), declared as the stop;
  // the Browser face's console errors (about:srcdoc, Blocked script execution) recur after #1419 closed, and no open Issue names them: new
  expectFail: ['#1395', 'new'],
  expectStop: '#1395',
};
export const LANGUAGES = ['de', 'es', 'fr', 'ja', 'ko', 'vi', 'zh'];
const NAME_OF = {
  de: /Deutsch|German/i, es: /Español|Espanol|Spanish/i, fr: /Français|Francais|French/i, ja: /日本語|Japanese/i,
  ko: /한국어|Korean/i, vi: /Tiếng Việt|Tieng Viet|Vietnamese/i, zh: /中文|简体|Chinese/i,
};
const RESTART_TITLE = 'Language change';
const SET_LINE = /window-api: RegSetValueExW "[^"]*" "Program language" type=1 size=\d+ value="([^"]*)"/i;
const FOUND_LINE = /window-api: RegQueryValueExW "[^"]*" "Program Language" -> found type=1 size=\d+ value="([^"]*)"/i;
const CONTROL = { dx: 12, dy: 20, w: 174, h: 22 };
const TRACE_ON = '(function () { const t = setInterval(() => { if (window.__app) { clearInterval(t); window.__app.windowApiTrace(true); } }, 5); })();';

const lastValue = (lines, re) => { const m = lines.map((l) => re.exec(l)).filter(Boolean); return m.length ? m.at(-1)[1].replace(/\?$/, '') : null; };

/** the General page: the settings window opened and the tab slot pressed (the slot found in English is kept for the other languages) */
async function generalAt(guest, slot) {
  const st = await openSettings(guest);
  if (!st) return null;
  await sleep(500);
  const t = await table(guest);
  const s = tops(t).find((w) => w.lid === st.lid);
  const bar = s ? under(t, s.lid).find((w) => w.cls === 'TQuickSheetSelector') : null;
  if (!bar) return null;
  await guest.click(bar.ax + ((slot + 0.5) * bar.w) / 10, bar.ay + bar.h / 2);
  await sleep(700);
  return settingsPageOf(await table(guest), s);
}

export default {
  ...SCENARIO,
  async run({ guest, page, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await traceOn(guest);
    await page.addInitScript(TRACE_ON);
    const notes = [...start.notes];
    const rows = [];
    let slot = null;
    // a language that cannot be run is recorded and the page is reloaded, so that the next language is still tried (all seven are run)
    const giveUp = async (code, why) => {
      rows.push({ code, error: why });
      notes.push(`${code}: ${why}`);
      log(notes.at(-1));
      await page.reload({ waitUntil: 'load', timeout: 120000 });
      await startDefault(guest, log);
    };
    for (const code of LANGUAGES) {
      let general = null;
      if (slot === null) {
        for (let s = 0; s < 10 && !general; s++) {
          const p = await generalAt(guest, s);
          if (p && p.text === 'General') { general = p; slot = s; }
        }
      } else {
        general = await generalAt(guest, slot);
      }
      if (!general) { await giveUp(code, 'the General page was not reached'); continue; }
      const t = await table(guest);
      const panel = under(t, general.lid).find((w) => w.text && /^Language/.test(w.text));
      if (!panel) { await giveUp(code, 'no Language panel'); continue; }
      const before = t.map((w) => w.lid);
      const kk = await kOf(guest);
      await guest.click(panel.ax + (CONTROL.dx + CONTROL.w / 2) * kk, panel.ay + (CONTROL.dy + CONTROL.h / 2) * kk);
      await sleep(1500);
      const items = under(await table(guest), general.lid).filter((w) => !before.includes(w.lid) && w.vis && w.text);
      const listed = (await table(guest)).filter((w) => !before.includes(w.lid) && w.vis && w.text);
      const target = [...items, ...listed].find((w) => NAME_OF[code].test(w.text)) ?? null;
      if (target) {
        await guest.click(target.ax + target.w / 2, target.ay + target.h / 2);
      } else {
        for (let i = 0; i <= LANGUAGES.indexOf(code); i++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(200); }
        await guest.key('Enter', 'Enter', 13);
      }
      await sleep(2500);
      const restart = newTops(await table(guest), before).some((w) => w.text === RESTART_TITLE);
      const written = lastValue(await engineLines(guest), SET_LINE);
      await page.reload({ waitUntil: 'load', timeout: 120000 });
      const again = await startDefault(guest, log);
      const reloadState = (await guest.state())?.state ?? null;
      const found = lastValue(await engineLines(guest), FOUND_LINE);
      const kept = found !== null && found === code;
      const row = { code, target: target ? target.text : null, written, restart, reloadState, reloadOk: again.ok, found, kept };
      rows.push(row);
      notes.push(`${code}: list ${target ? `"${target.text}"` : 'by keys'}, written ${written}, restart prompt ${restart}, reload ${reloadState}, read back ${found} (kept ${kept})`);
      log(notes.at(-1));
      // a stopped engine (the reload did not reach the idle state) is reloaded once more, so that the next language starts from a running engine
      if (!again.ok) {
        await page.reload({ waitUntil: 'load', timeout: 120000 });
        const back = await startDefault(guest, log);
        notes.push(`${code}: after the stop, the page reloaded: ${back.ok ? 'engine-idle' : back.state}`);
        log(notes.at(-1));
      }
    }
    const ok = rows.length === LANGUAGES.length && rows.every((r) => !r.error && r.restart && r.kept && r.reloadState === 'engine-idle' && r.written === r.code);
    return result(ok, (await guest.state())?.state ?? null, notes, { rows });
  },
};
