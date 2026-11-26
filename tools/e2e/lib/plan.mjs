// The plan of a run (Issue #1485): the arguments, the displays, the budget, the shard and the acceptance of a row. Pure (no browser), so the tests can use it.
// The display names: 4k-dpr1, 4k-dpr2, 4k-random (as before) and dpr:<number> (e.g. dpr:1.25): the CSS size is 3840 / DPR by 2160 / DPR.
import { shardOf, sortById } from './scenario.mjs';

export const DISPLAY = { width: 3840, height: 2160, refreshHzTarget: 120 };
const BOOLEAN_FLAGS = new Set(['list', 'report-only']);

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

/** the acceptance of one row of `n` scenario results (the same rule as judge.jq, which the job runs on the output); n defaults to the row's own count */
export function rowPasses(row, n = row.scenarios?.length) {
  return row.servedHeadersMatch === true && row.error === null && row.scenarios.length === n
    && row.scenarios.every((s) => s.skipped !== true && (s.expectFail !== null
      ? s.pageErrors === 0 && s.consoleErrors === 0 && s.failedRequests === 0
      : s.ok === true && s.pageErrors === 0 && s.consoleErrors === 0 && s.stuckToasts === 0 && s.failedRequests === 0));
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

/** a budget clock: `expired()` is true once the budget has passed (no budget: never) */
export function budgetClock(ms, now = () => Date.now()) {
  const t0 = now();
  return { expired: () => ms !== null && now() - t0 >= ms, elapsedMs: () => now() - t0 };
}
