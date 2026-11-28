// The calls that the engine answers without a model (lift/runtime/fle_env.c fle_warn, Issue #1418), as the measurement beacon carries them: `warns`, the number of calls per API name,
// the top WARNS_TOP. The engine prints its counts as one line, "fle-warns: {"<API>":<calls>,...}", at its idle reports and before a stop; the page keeps the last such line of its
// log. Names and counts only: nothing that the guest passed in, no path, no text.
import { PERF_WARNS_TOP, warnsValid as sharedWarnsValid } from '../../shared/perf.ts';

export const WARNS_TOP = PERF_WARNS_TOP;
export const WARNS_LINE = 'fle-warns: ';
const API_NAME = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;

/** the `warns` of a beacon: the counts of the last counts line in the log, the most calls first (then by name), at most WARNS_TOP; {} when the log has none. Never throws. */
export function warnsOf(lines: readonly string[]): Record<string, number> {
  for (let i = lines.length - 1; i >= 0; i--) {
    const l = (lines[i] as string).replace(/^\[err\] ?/, '');
    if (!l.startsWith(WARNS_LINE)) continue;
    let o: unknown;
    try { o = JSON.parse(l.slice(WARNS_LINE.length)); } catch { return {}; }
    if (typeof o !== 'object' || o === null || Array.isArray(o)) return {};
    const entries: [string, number][] = [];
    for (const [k, v] of Object.entries(o as Record<string, unknown>)) if (API_NAME.test(k) && typeof v === 'number' && Number.isSafeInteger(v) && v >= 0) entries.push([k, v]);
    entries.sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    return Object.fromEntries(entries.slice(0, WARNS_TOP));
  }
  return {};
}

/** whether a `warns` value has the shape the server accepts (shared/perf.ts) */
export const warnsValid = sharedWarnsValid;
