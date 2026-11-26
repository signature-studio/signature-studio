// The scenario registry (Issue #1485). A scenario is a file of tools/e2e/scenarios/<area>/ that default-exports defineScenario({...}); run.mjs finds the files itself.
// Pure: no browser, no page, so that the tests (tools/e2e/test) and --list can use it without Playwright.
export const AREAS = ['page', 'input', 'file', 'menus', 'windows', 'audio', 'settings', 'plugins', 'topbar', 'resilience', 'visual', 'perf', 'delivery'];
export const DEFAULT_DISPLAYS = ['4k-dpr1', '4k-random'];
const SCENARIO_ID = /^([a-z]+)\/([a-z0-9][a-z0-9-]*)$/;
const FEATURE_ID = /^([a-z]+)\/[^/\s]+\/[^/\s]+$/;
const MARK = /^#\d+$/;

/** one mark: `#<Issue の番号>` or 'new' (a defect found by this scenario, not yet an Issue) */
const oneMark = (m) => {
  if (m === 'new' || (typeof m === 'string' && MARK.test(m))) return m;
  throw new Error(`expectFail ${JSON.stringify(m)}: use null, '#<Issue の番号>', 'new' or a list of them`);
};

/** the mark of a known defect: null (none), one mark, or a list of marks (a scenario with two known defects: each one is a mark) */
export function checkMark(expectFail) {
  if (expectFail === null || expectFail === undefined) return null;
  if (Array.isArray(expectFail)) {
    if (expectFail.length === 0) throw new Error('expectFail: an empty list (use null)');
    return expectFail.map(oneMark);
  }
  return oneMark(expectFail);
}

/** a scenario's marks as a list: none (null), one mark, or a list of marks */
export const marksOf = (e) => (e === null || e === undefined ? [] : [e].flat());

/** the marks of a set of scenarios: each mark is 'new' or an issue of the set's allowed list (a mark outside that list is refused) */
export function checkMarks(scenarios, allowed) {
  for (const sc of scenarios) {
    for (const m of marksOf(sc.expectFail)) {
      if (m === 'new') continue;
      if (!MARK.test(String(m)) || !Array.isArray(allowed) || !allowed.includes(m)) {
        throw new Error(`scenario ${sc.name ?? sc.id}: expectFail ${JSON.stringify(m)} is not one of the allowed marks (${(allowed ?? []).join(', ')}) or new`);
      }
    }
  }
}

/** the area of a scenario ID (`<領域>/<名前>`) or a feature ID (`<領域>/<種類>/<名前>`); null when the area is not one of the 13 */
export function areaOf(id) {
  const m = /^([a-z]+)\//.exec(String(id));
  return m && AREAS.includes(m[1]) ? m[1] : null;
}

/**
 * defineScenario({ id, area?, covers, displays, expectFail, run(ctx) }).
 * id is `<領域>/<名前>` (area is its first part; `area`, when given, must agree). covers: the feature IDs the scenario checks (`<領域>/<種類>/<名前>`).
 * displays: the display names (default DEFAULT_DISPLAYS). expectFail: null, '#<Issue>' or 'new'. run(ctx) returns { ok, notes, results }.
 */
export function defineScenario(s) {
  if (!s || typeof s !== 'object') throw new Error('defineScenario needs an object');
  const m = SCENARIO_ID.exec(String(s.id ?? ''));
  if (!m) throw new Error(`scenario id ${JSON.stringify(s.id)}: use <領域>/<名前> (lower case, digits, hyphens)`);
  if (!AREAS.includes(m[1])) throw new Error(`scenario ${s.id}: area ${m[1]} is not one of ${AREAS.join(', ')}`);
  if (s.area !== undefined && s.area !== m[1]) throw new Error(`scenario ${s.id}: area ${s.area} does not match the id`);
  const covers = s.covers ?? [];
  if (!Array.isArray(covers)) throw new Error(`scenario ${s.id}: covers is an array of feature IDs`);
  for (const c of covers) {
    const f = FEATURE_ID.exec(String(c));
    if (!f || !AREAS.includes(f[1])) throw new Error(`scenario ${s.id}: cover ${JSON.stringify(c)} must be a feature ID (<領域>/<種類>/<名前>, the area one of the 13)`);
  }
  if (new Set(covers).size !== covers.length) throw new Error(`scenario ${s.id}: covers repeats a feature`);
  const displays = s.displays ?? DEFAULT_DISPLAYS;
  if (!Array.isArray(displays) || displays.length === 0) throw new Error(`scenario ${s.id}: displays is a non-empty array`);
  if (typeof s.run !== 'function') throw new Error(`scenario ${s.id}: run(ctx) is a function`);
  return { kind: 'scenario', id: s.id, name: s.id, area: m[1], covers: [...covers], displays: [...displays], expectFail: checkMark(s.expectFail), run: s.run };
}

/** a scenario of the older set form ({ name, expectFail, run, beforeLoad?, expect? }; scenarios/<set>/index.mjs, set = smoke, browser-face or regression): it has no covers, and each runs in a page of its own */
export function legacyScenario(s, set = 'smoke') {
  return {
    kind: 'legacy', id: s.name, name: s.name, area: set, covers: [], displays: ['4k-dpr1', '4k-dpr2', '4k-random'], expectFail: checkMark(s.expectFail ?? null),
    expect: s.expect ?? null, beforeLoad: s.beforeLoad, run: s.run,
  };
}

/** the sort order of the shards and of the output: the IDs in ascending string order */
export function sortById(list) {
  return [...list].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** the scenarios of one shard `i/n` (1-based): the sorted list's index % n === i - 1 (deterministic) */
export function shardOf(list, i, n) {
  return sortById(list).filter((_, k) => k % n === i - 1);
}
