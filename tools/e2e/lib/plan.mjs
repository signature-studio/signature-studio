// The plan of a run (Issue #1485, #1506): the arguments, the displays and their rows, the budget of a row, the shard, the result of a scenario and the acceptance
// of a scenario and of a row. Pure (no browser), so the tests can use it.
// The display names: 4k-dpr1, 4k-dpr2, 4k-random (as before) and dpr:<number> (e.g. dpr:1.25): the CSS size is 3840 / DPR by 2160 / DPR.
import { areaOf, shardOf, sortById } from './scenario.mjs';

export const DISPLAY = { width: 3840, height: 2160, refreshHzTarget: 120 };
export const FAIL_LOG_LINES = 200;
const BOOLEAN_FLAGS = new Set(['list', 'report-only']);

/**
 * the areas of the IDs of `--only` (`<領域>/<名前>`), when every ID has one of the 13 areas (the scenarios are then the areas' own, with no --area or --scenarios); else null
 */
export function areasOfOnly(only) {
  if (!Array.isArray(only) || only.length === 0) return null;
  const areas = only.map((id) => areaOf(id));
  if (areas.some((a) => a === null)) return null;
  return [...new Set(areas)];
}

/** the arguments: `--name value` pairs, and the flags of BOOLEAN_FLAGS (no value) */
export function parseArgs(argv) {
  const a = {};
  for (let i = 0; i < argv.length; i++) {
    const t = argv[i];
    if (!t.startsWith('--')) throw new Error(`unexpected argument ${t}`);
    const key = t.slice(2);
    if (BOOLEAN_FLAGS.has(key)) { a[key] = true; continue; }
    const v = argv[++i];
    if (v === undefined) throw new Error(`--${key} needs a value`);
    a[key] = v;
  }
  return a;
}

/** a budget: a number with the unit s, m or h (`40m`, `90s`, `2h`) in milliseconds */
export function parseBudget(text) {
  const m = /^(\d+(?:\.\d+)?)(s|m|h)$/.exec(String(text).trim());
  if (!m) throw new Error(`--budget ${text}: use a number with s, m or h (e.g. 40m)`);
  return Math.round(Number(m[1]) * { s: 1000, m: 60000, h: 3600000 }[m[2]]);
}

/** a shard `i/n`: 1 <= i <= n */
export function parseShard(text) {
  const m = /^(\d+)\/(\d+)$/.exec(String(text).trim());
  if (!m) throw new Error(`--shard ${text}: use i/n (e.g. 1/2)`);
  const i = Number(m[1]), n = Number(m[2]);
  if (n < 1 || i < 1 || i > n) throw new Error(`--shard ${text}: 1 <= i <= n`);
  return { i, n };
}

/** a repeat count: an integer from 1 */
export function parseRepeat(text) {
  const n = Number(text);
  if (!Number.isInteger(n) || n < 1) throw new Error(`--repeat ${text}: an integer from 1`);
  return n;
}

/** the rows of one display name: the CSS size of the 3840 x 2160 physical picture at the DPR (the same formula for every name) */
export function displayRows(name, rnd = Math.random) {
  const row = (n, dpr) => ({ name: n, dpr, cssWidth: Math.round(DISPLAY.width / dpr), cssHeight: Math.round(DISPLAY.height / dpr) });
  if (name === '4k-dpr1') return [row(name, 1)];
  if (name === '4k-dpr2') return [row(name, 2)];
  if (name === '4k-random') {
    return [0, 1].map(() => row(name, Math.round((1.01 + rnd() * 1.99) * 100) / 100));
  }
  const d = /^dpr:(\d+(?:\.\d+)?)$/.exec(name);
  if (d) {
    const dpr = Number(d[1]);
    if (!(dpr >= 1 && dpr <= 3)) throw new Error(`display ${name}: the DPR is from 1 to 3`);
    return [row(name, dpr)];
  }
  throw new Error(`no display ${name} (4k-dpr1, 4k-dpr2, 4k-random, dpr:<number>)`);
}

/** the DPR a display name fixes: dpr:<n>, 4k-dpr1 (1), 4k-dpr2 (2); null for 4k-random (its two DPRs are drawn at random) */
export function fixedDpr(name) {
  if (name === '4k-random') return null;
  return displayRows(name)[0].dpr;
}

/** the DPR check of a page: true when the page's DPR is the row's declared one, false when it is not, null when the row has no fixed DPR (4k-random) */
export function dprMatches(name, pageDpr) {
  const want = fixedDpr(name);
  if (want === null) return null;
  return Math.abs(pageDpr - want) < 1e-6;
}

/**
 * the rows of one display name, each with the scenarios of the plan that declare that name (`items` are { sc, repeat }). --displays names the rows only:
 * a scenario runs in a display only when its own displays include it. A row with no scenario is returned with no items (the caller drops it).
 */
export function rowsFor(name, items) {
  const picked = items.filter((it) => it.sc.displays.includes(name));
  return displayRows(name).map((d) => ({ d, items: picked }));
}

/** the acceptance of one row of `n` scenario results (the same rule as judge.jq, which the job runs on the output); n defaults to the row's own count */
export function rowPasses(row, n = row.scenarios?.length) {
  return row.servedHeadersMatch === true && row.error === null && row.scenarios.length === n && row.scenarios.every(scenarioPasses);
}

/** the engine stopped in the scenario: its stop report (engineStops > 0), or the engine stayed in `running` for more than 90 s (engineStuckRunning) */
export function stopped(s) {
  return s.engineStops > 0 || s.engineStuckRunning === true;
}

/** the scenario's run threw (its error is not null: the page's evaluate failed, e.g. Target crashed); such a result is never accepted (Issue #1528) */
export const errored = (s) => (s.error ?? null) !== null;

/**
 * the acceptance of one scenario result. A skipped scenario fails; an errored one fails (marked or not: a mark does not hide an error); a page whose DPR is not
 * the declared one fails (dprOk false), marked or not. An unmarked scenario (expectFail null) is ok with no counter. A marked one (a known defect) is judged by
 * its counters only: 0 pageErrors, consoleErrors and failedRequests (its ok and its stuckToasts are not judged), and no engine stop unless it declares the stop
 * (expectStop: '#<Issue>' or 'new').
 */
export function scenarioPasses(s) {
  if (s.skipped === true || s.dprOk === false || errored(s)) return false;
  const quiet = s.pageErrors === 0 && s.consoleErrors === 0 && s.failedRequests === 0;
  if ((s.expectFail ?? null) !== null) return quiet && ((s.expectStop ?? null) !== null || !stopped(s));
  return s.ok === true && quiet && s.stuckToasts === 0;
}

/**
 * a scenario result that gets the engine log's tail (failLog): an errored one, an unmarked one that fails the acceptance, or a marked one whose engine stopped (a known
 * failure without a stop has no log, so the JSON stays small). A skipped one is not run, so it has no log.
 */
export function needsFailLog(s) {
  if (s.skipped === true) return false;
  if (errored(s)) return true;
  if ((s.expectFail ?? null) === null) return !scenarioPasses(s);
  return stopped(s);
}

/** the last FAIL_LOG_LINES lines of a log (the lines since the scenario started, in the order the page keeps them) */
export function tailLines(lines, n = FAIL_LOG_LINES) {
  return lines.slice(Math.max(0, lines.length - n));
}

const PAGE_GONE = /Target crashed|Page crashed|Session closed|has been closed/;

/**
 * the page of a shard is gone after a scenario (Issue #1528): its renderer stopped (the page's crash event, or Target crashed in the scenario's error), or the page
 * is closed. The page is then made again for the next scenario; an error that is only the scenario's own (a timeout, a missing window) leaves the page as it is.
 */
export function pageGone({ crashed = false, closed = false, error = null } = {}) {
  return crashed === true || closed === true || (error !== null && PAGE_GONE.test(String(error)));
}

/** a scenario that did not run (its page did not start): an error, not ok, its counts 0; it fails the acceptance whether marked or not */
export function erroredScenario(sc, repeat, d, error, ms = 0) {
  return { ...scenarioResult(sc, repeat, d), ok: false, error: String(error).slice(0, 800), ms };
}

/** the result of one scenario in a row, before it runs: the fields of the JSON (the counts are 0, ok is false until the scenario has run) */
export function scenarioResult(sc, repeat, d) {
  return {
    id: sc.id, name: sc.name ?? sc.id, area: sc.area, covers: sc.covers ?? [], repeat, displays: sc.displays ?? null,
    expectFail: sc.expectFail ?? null, expectStop: sc.expectStop ?? null, expect: sc.expect ?? null,
    ok: false, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0, dprOk: null, ms: 0,
    engineState: null, engineStops: 0, engineStuckRunning: false, engineRunningMaxMs: 0, engineStop: null, dumpRetries: 0, beginFrames: 0, beginFrameErrors: 0,
    error: null, notes: [], details: [], display: d.name,
  };
}

/**
 * the exit of a run over its rows, each with the expected count of its scenario lines. Without reportOnly: every row passes rowPasses.
 * With reportOnly (--report-only: a run that reports the results and does not judge them): the lines are all there and every row's error is null; the scenario results do not count.
 */
export function runPasses(rows, expected, { reportOnly = false } = {}) {
  if (rows.length !== expected.length) return false;
  if (!reportOnly) return rows.every((r, i) => rowPasses(r, expected[i]));
  return rows.every((r, i) => r.error === null && r.scenarios?.length === expected[i]);
}

/**
 * the scenarios of a run, in the order they are run: `all` (the scenarios of the set, each with an id), filtered by `only` (IDs or names), sorted,
 * cut to the shard, and repeated `repeat` times. Returns [{ sc, repeat }]. Throws when `only` names no scenario of the set.
 */
export function planScenarios(all, { only = null, shard = null, repeat = 1 } = {}) {
  let list = all;
  if (only !== null) {
    if (only.length === 0 || only.some((n) => !all.some((s) => s.id === n || s.name === n))) {
      throw new Error(`--only: ${only.join(',')} is not a scenario of the set (${all.map((s) => s.id).join(', ')})`);
    }
    list = all.filter((s) => only.includes(s.id) || only.includes(s.name));
  }
  list = shard === null ? sortById(list) : shardOf(list, shard.i, shard.n);
  const out = [];
  for (let r = 1; r <= repeat; r++) for (const sc of list) out.push({ sc, repeat: r });
  return out;
}

/**
 * a budget clock (one per display row): `expired()` is true once the budget has passed (no budget: never); `remainingMs()` is the time left (Infinity with no budget);
 * `canStart(needMs)` is true when the budget is not past and at least `needMs` is left (a scenario that needs 30 min does not start with less left).
 */
export function budgetClock(ms, now = () => Date.now()) {
  const t0 = now();
  const elapsed = () => now() - t0;
  return {
    expired: () => ms !== null && elapsed() >= ms,
    elapsedMs: elapsed,
    remainingMs: () => (ms === null ? Infinity : ms - elapsed()),
    canStart: (needMs = 0) => ms === null || (elapsed() < ms && ms - elapsed() >= needMs),
  };
}
