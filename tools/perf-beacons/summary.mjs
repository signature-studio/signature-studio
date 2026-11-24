// The measurement beacons that real visitors' browsers sent (decision 0152, 0224), read from a directory of the objects the Worker stores (perf/<UTC date>/<sid>-<seq>.json:
// one line each, { t, remote, perf }). Prints Markdown: one row per bundle version (the engine's sha256 prefix) in two tables (identity; what the page told and what the engine ran),
// the sessions at a device pixel ratio of 1.7 (per day, or all), and the shapes of the records.
// usage: node tools/perf-beacons/summary.mjs [dir] [--day YYYY-MM-DD | --day all | --exits]
//   dir: default lift/out-app/host-extra/perf (the work step puts the objects of the last three days there; nothing is committed)
//   --day: the sessions table is limited to this UTC date (default: today in UTC); all: every date
//   --exits: instead of the tables above, one row per session that ended or failed in the last 48 hours (UTC): an engine.exit, an engine.state other than engine-idle, an error or a failure report
// Only counts and derived numbers are printed: no session id, no remote class, no user agent text, no address, no path or name of a visitor.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const DEFAULT_DIR = 'lift/out-app/host-extra/perf';
const USAGE = 'usage: node tools/perf-beacons/summary.mjs [dir] [--day YYYY-MM-DD | --day all | --exits]';
const EXITS_HOURS = 48;

function fail(msg) {
  console.error(`summary: ${msg}`);
  console.error(USAGE);
  process.exit(2);
}

function parseArgs(argv) {
  let dir = DEFAULT_DIR;
  let day = new Date().toISOString().slice(0, 10);
  let exits = false;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--day') {
      day = argv[++i];
      if (!day) fail('--day needs a value');
    } else if (a === '--exits') exits = true;
    else if (a.startsWith('--')) fail(`unknown option ${a}`);
    else dir = a;
  }
  return { dir, day, exits };
}

function jsonFiles(dir) {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile() && e.name.endsWith('.json')) out.push(p);
    }
  };
  walk(dir);
  return out;
}

// a file that does not parse to a stored beacon is only counted
function readRecords(dir) {
  const recs = [];
  let unreadable = 0;
  for (const f of jsonFiles(dir)) {
    let r = null;
    try { r = JSON.parse(fs.readFileSync(f, 'utf8')); } catch { r = null; }
    if (r && typeof r === 'object' && typeof r.t === 'string' && r.perf && typeof r.perf === 'object') recs.push({ t: r.t, p: r.perf });
    else unreadable++;
  }
  recs.sort((a, b) => (a.t < b.t ? -1 : a.t > b.t ? 1 : 0));
  return { recs, unreadable };
}

const num = (x) => (typeof x === 'number' && Number.isFinite(x) ? x : null);
const str = (x) => (typeof x === 'string' && x !== '' ? x : null);
const median = (a) => {
  const v = a.filter((x) => x !== null).sort((x, y) => x - y);
  if (v.length === 0) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};
const maxOf = (a) => { const v = a.filter((x) => x !== null); return v.length ? Math.max(...v) : null; };
const f1 = (x) => (x === null ? '-' : x < 10 ? x.toFixed(1) : x.toFixed(0));
const when = (t) => `${t.slice(0, 16).replace('T', ' ')}Z`;
const counted = (items) => {
  const m = new Map();
  for (const k of items) m.set(k, (m.get(k) ?? 0) + 1);
  return [...m].sort((a, b) => b[1] - a[1] || String(a[0]).localeCompare(String(b[0])));
};
const listOf = (items, top = 6) => {
  const c = counted(items);
  if (c.length === 0) return '-';
  const shown = c.slice(0, top).map(([k, n]) => `${k} ×${n}`);
  return shown.join(', ') + (c.length > top ? `, 他 ${c.length - top} 種` : '');
};

/** the scale the page's ratio gives Windows: round(ratio × 100) percent, and DPI = MulDiv(96, percent, 100) (rounded to the nearest, halves up) */
export function dpiOf(dpr) {
  const pct = Math.round(dpr * 100);
  return Math.floor((2 * 96 * pct + 100) / 200);
}
const dpiClass = (dpi) => (dpi === 96 ? '96 相当' : dpi >= 160 && dpi < 180 ? '169 相当' : 'その他');

/** the browser's name and major version: the page's own parse when there is one (client.browser), else the user agent's words (not printed) */
function browserOf(p) {
  const c = p.client?.browser;
  if (c && str(c.name)) {
    const major = num(c.major) ?? (str(c.version) ? Number(String(c.version).split('.')[0]) : null);
    return major !== null ? `${c.name} ${major}` : c.name;
  }
  const ua = str(p.browser?.userAgent) ?? '';
  const m = /Edg(?:e|A|iOS)?\/(\d+)/.exec(ua) ?? /OPR\/(\d+)/.exec(ua) ?? /Firefox\/(\d+)/.exec(ua) ?? /(?:HeadlessChrome|Chrome|CriOS)\/(\d+)/.exec(ua) ?? /Version\/(\d+).*Safari\//.exec(ua);
  const name = /Edg(?:e|A|iOS)?\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox' : /(?:HeadlessChrome|Chrome|CriOS)\//.test(ua) ? 'Chrome' : /Version\/.*Safari\//.test(ua) ? 'Safari' : '不明';
  return m ? `${name} ${m[1]}` : name;
}

const dprOf = (p) => num(p.guestScreen?.dpr);
const guestOf = (p) => (num(p.guestScreen?.width) !== null && num(p.guestScreen?.height) !== null ? { w: p.guestScreen.width, h: p.guestScreen.height } : null);
const roomOf = (p) => (num(p.guestScreen?.roomWidth) !== null && num(p.guestScreen?.roomHeight) !== null ? { w: p.guestScreen.roomWidth, h: p.guestScreen.roomHeight } : null);
const firstIdleOf = (p) => num(p.startup?.firstIdleSeconds);
const underrunsOf = (p) => num(p.audio?.underruns);
const p95Of = (p) => num(p.input?.eventToRaf?.p95Ms);

// the sessions of a list of records (in time order): one list per page load, grouped by its random token (never printed)
function sessionsOf(recs) {
  const by = new Map();
  recs.forEach((r, i) => {
    const key = str(r.p.sid) ?? `(無し)#${i}`;
    if (!by.has(key)) by.set(key, []);
    by.get(key).push(r);
  });
  return [...by.values()];
}

// a window size would be a key that names a window and its width, height or size (engineRun.windowSeconds is a time span; the page's client.viewport is the browser's, not the engine's window)
const WINDOW_SIZE_KEY = /window.*(width|height|size)|(width|height|size).*window|^uiWindows$/i;
function hasWindowSizeKey(o, depth = 0) {
  if (!o || typeof o !== 'object' || depth > 4) return false;
  return Object.entries(o).some(([k, v]) => WINDOW_SIZE_KEY.test(k) || hasWindowSizeKey(v, depth + 1));
}

function versionRow(version, recs) {
  const sess = sessionsOf(recs);
  const firstIdles = sess.map((rs) => rs.map((r) => firstIdleOf(r.p)).find((x) => x !== null) ?? null).filter((x) => x !== null);
  const underruns = sess.map((rs) => rs.map((r) => underrunsOf(r.p)).filter((x) => x !== null).pop() ?? null).filter((x) => x !== null);
  const p95 = recs.map((r) => p95Of(r.p));
  const exits = recs.filter((r) => r.p.engine && r.p.engine.exit && typeof r.p.engine.exit === 'object');
  return {
    version,
    records: recs.length,
    sessions: sess.length,
    first: when(recs[0].t),
    last: when(recs[recs.length - 1].t),
    browsers: listOf(recs.map((r) => browserOf(r.p)), 4),
    failures: recs.filter((r) => r.p.failure).length,
    dpr: listOf(recs.map((r) => (dprOf(r.p) === null ? '(無し)' : String(dprOf(r.p)))), 4),
    dpi: listOf(recs.filter((r) => dprOf(r.p) !== null).map((r) => String(dpiOf(dprOf(r.p)))), 4),
    dpiField: listOf(recs.map((r) => (r.p.guestScreen ? (num(r.p.guestScreen.dpi) !== null ? String(r.p.guestScreen.dpi) : 'null') : '(無し)')), 4),
    guest: listOf(recs.filter((r) => guestOf(r.p)).map((r) => `${guestOf(r.p).w}×${guestOf(r.p).h}`), 4),
    room: listOf(recs.filter((r) => roomOf(r.p)).map((r) => `${roomOf(r.p).w}×${roomOf(r.p).h}`), 4),
    firstIdle: { median: median(firstIdles), max: maxOf(firstIdles), n: firstIdles.length, over10: firstIdles.filter((x) => x > 10).length },
    underruns: { max: maxOf(underruns), n: underruns.length },
    input: { median: median(p95), n: p95.filter((x) => x !== null).length },
    states: listOf(recs.map((r) => str(r.p.engine?.state) ?? '(無し)'), 6),
    exits: exits.length ? listOf(exits.map((r) => `${str(r.p.engine.exit.reason) ?? '(無し)'} (code ${num(r.p.engine.exit.code) ?? '-'})`), 4) : '-',
  };
}

function sessionRow(i, rs) {
  const withGs = rs.filter((r) => r.p.guestScreen);
  const last = withGs[withGs.length - 1] ?? null;
  const dpr = last ? dprOf(last.p) : null;
  const dpi = dpr !== null ? dpiOf(dpr) : null;
  const g = last ? guestOf(last.p) : null;
  const room = last ? roomOf(last.p) : null;
  const phys = g && room && dpr !== null ? (Math.abs(g.w - Math.round(room.w * dpr)) <= 1 && Math.abs(g.h - Math.round(room.h * dpr)) <= 1 ? 'はい' : 'いいえ') : '-';
  const reported = withGs.filter((r) => r.p.guestScreen.reported === true).length;
  const firstIdle = rs.map((r) => firstIdleOf(r.p)).find((x) => x !== null) ?? null;
  const under = rs.map((r) => underrunsOf(r.p)).filter((x) => x !== null).pop() ?? null;
  return [
    `#${i + 1}`, when(rs[0].t), when(rs[rs.length - 1].t), rs.length, rs[rs.length - 1].p.bundle?.version ?? '(無し)', browserOf(rs[rs.length - 1].p),
    dpr === null ? '-' : String(dpr), dpi === null ? '-' : String(dpi), dpi === null ? '-' : dpiClass(dpi),
    g ? `${g.w}×${g.h}` : '-', room ? `${room.w}×${room.h}` : '-', room && dpr !== null ? `${Math.round(room.w * dpr)}×${Math.round(room.h * dpr)}` : '-', phys,
    `${reported}/${withGs.length}`, f1(firstIdle), under === null ? '-' : String(under),
  ];
}

function table(head, rows) {
  const line = (cells) => `| ${cells.map((c) => String(c)).join(' | ')} |`;
  return [line(head), line(head.map(() => '---')), ...rows.map(line)].join('\n');
}

function shapeOf(p) {
  const on = (k) => (p[k] ? '有' : '無');
  return `desk ${on('desk')} / engineRun ${on('engineRun')} / files ${on('files')} / assets ${on('assets')}`;
}

/** a record that says how the engine ended or failed: an engine.exit, an engine.state other than engine-idle (a failure report has no engine block, so no state: it counts too), an error or a failure */
export function isExitRecord(p) {
  return !!(p.engine?.exit && typeof p.engine.exit === 'object') || str(p.engine?.state) !== 'engine-idle' || !!p.error || !!p.failure;
}

/** the page's words as the table prints them: the page has removed the personal parts already; this takes out what could still name a person (an address, a mail address, a path under a user's folder) and what breaks a table. The engine's own paths (C:\Windows, C:\Program Files) stay. */
const VM_PATH = /^[A-Za-z]:\\(?:Windows|Program Files(?: \(x86\))?|ProgramData)(?:\\|$)/i;
const WIN_PATH = /(?:[A-Za-z]:\\|\\\\[^\s\\/:]+\\)(?:[^\\"'<>|*?;:,]+\\)*[^\s\\"'<>|*?;:,]*/g;
const POSIX_PATH = /(?<![\w.:\\<])\/(?:[^\s/"'<>|*?;,:]+\/)+[^\s/"'<>|*?;,:]*/g;
export function scrubWords(text) {
  return String(text)
    .replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, ' ')
    .replace(/\b[a-z][a-z0-9+.-]*:\/\/[^\s"'<>]+/gi, '<url>')
    .replace(/[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+/g, '<email>')
    .replace(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g, '<ip>')
    .replace(WIN_PATH, (p) => (VM_PATH.test(p) && !/^[A-Za-z]:\\Users(?:\\|$)/i.test(p) ? p : '<path>'))
    .replace(POSIX_PATH, (p) => (/^\/(?:proc|dev|sys)(?:\/|$)/.test(p) ? p : '<path>'))
    .replace(/\|/g, '\\|')
    .replace(/ {2,}/g, ' ')
    .trim()
    .slice(0, 400);
}

/** the last value that `pick` gives for the records of a session, in time order (null: none of them has it) */
const lastWith = (rs, pick) => {
  for (let i = rs.length - 1; i >= 0; i--) {
    const v = pick(rs[i].p);
    if (v !== undefined && v !== null) return v;
  }
  return null;
};
const objOf = (x) => (x && typeof x === 'object' ? x : null);

/** one row per session: the first and the last instant, the version and the device pixel ratio (the last one that has it), the count of records, and the last value of each field the table shows */
function exitRow(rs) {
  const last = rs[rs.length - 1];
  const dpr = lastWith(rs, (p) => dprOf(p));
  const exit = lastWith(rs, (p) => objOf(p.engine?.exit));
  const err = lastWith(rs, (p) => objOf(p.error));
  const fail = lastWith(rs, (p) => objOf(p.failure));
  const state = lastWith(rs, (p) => str(p.engine?.state));
  const top = Object.entries(objOf(last.p.timeline) ?? {}).filter(([, ms]) => num(ms) !== null).sort((a, b) => b[1] - a[1])[0];
  return [
    when(rs[0].t), when(last.t), last.p.bundle?.version ?? '(無し)',
    dpr === null ? '-' : `${dpr} (${dpiClass(dpiOf(dpr))})`,
    rs.length,
    state ?? '(無し)',
    exit ? str(exit.reason) ?? '-' : '-',
    exit ? num(exit.code) ?? '-' : '-',
    exit && str(exit.message) ? scrubWords(exit.message) : '-',
    err ? scrubWords(str(err.what) ?? '-') : fail ? `failure: ${str(fail.kind) ?? '-'}` : '-',
    err ? scrubWords(str(err.where) ?? '-') : fail && str(fail.where) ? scrubWords(fail.where) : '-',
    num(last.p.pageMs) === null ? '-' : f1(last.p.pageMs / 1000),
    top ? `${top[0]} (${f1(top[1] / 1000)} s)` : '-',
  ];
}

/** the sessions that ended or failed in the `EXITS_HOURS` before `nowMs` (UTC): a count line, the kinds of the engine's exits, and one row per session (the sid is never printed) */
export function exitsReport(recs, nowMs = Date.now()) {
  const since = nowMs - EXITS_HOURS * 3_600_000;
  const win = recs.filter((r) => Date.parse(r.t) >= since);
  const flagged = win.filter((r) => isExitRecord(r.p));
  const sessions = sessionsOf(win).filter((rs) => rs.some((r) => isExitRecord(r.p)));
  const kinds = counted(flagged.filter((r) => objOf(r.p.engine?.exit)).map((r) => `${str(r.p.engine.exit.reason) ?? '(無し)'} (code ${num(r.p.engine.exit.code) ?? '-'})`));
  const out = [];
  out.push(`# 実機のビーコン: 終了と失敗(直近 ${EXITS_HOURS} 時間、UTC)`);
  out.push('');
  out.push(`- 期間(UTC): ${when(new Date(since).toISOString())} から ${when(new Date(nowMs).toISOString())} まで(直近 ${EXITS_HOURS} 時間)。記録: ${win.length} 件`);
  out.push(`- 終了・失敗の記録: ${flagged.length} 件(engine.exit が在る記録、engine.state が engine-idle でない記録、error か failure が在る記録)、セッション: ${sessions.length}`);
  out.push(`- engine.exit の内訳(記録ごと): ${kinds.length ? kinds.map(([k, n]) => `${k} ×${n}`).join(', ') : '-'}`);
  out.push(`- error の記録: ${flagged.filter((r) => r.p.error).length} 件、failure の記録: ${flagged.filter((r) => r.p.failure).length} 件`);
  out.push('');
  out.push('1 行が 1 セッション(読み込みごとの乱数で束ねた。乱数は出さない)。各項目は、そのセッションの最後の記録の値(項目ごとに、値の在る最後の記録)。pageMs は最後の記録の、起動からの時間(秒)。timeline の最後の印は、最後の記録で最も遅い印。');
  out.push('');
  out.push(table(['最初 (UTC)', '最後 (UTC)', '版 (bundle.version)', 'DPR の区分', '記録数', 'engine.state', 'engine.exit.reason', 'engine.exit.code', 'engine.exit.message', 'error.what', 'error.where', 'pageMs (秒)', 'timeline の最後の印'],
    sessions.map(exitRow)));
  return out.join('\n');
}

function main() {
  const { dir, day, exits } = parseArgs(process.argv.slice(2));
  if (!fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) fail(`not a directory: ${dir}`);
  const { recs, unreadable } = readRecords(dir);
  if (exits) {
    console.log(exitsReport(recs));
    return;
  }
  const out = [];
  const sessionCount = new Set(recs.map((r, i) => str(r.p.sid) ?? `(無し)#${i}`)).size;
  const headless = recs.filter((r) => r.p.client?.browser?.headless === true).length;
  const byDay = counted(recs.map((r) => r.t.slice(0, 10)));
  out.push('# 実機のビーコン: 集計');
  out.push('');
  out.push(`- 記録: ${recs.length} 件、セッション(読み込み): ${sessionCount}、読めなかった: ${unreadable} 件(件数だけ)、ヘッドレスの記録: ${headless} 件`);
  out.push(`- 期間(UTC): ${recs.length ? `${when(recs[0].t)} から ${when(recs[recs.length - 1].t)} まで` : '記録なし'}`);
  out.push(`- 日ごとの記録: ${byDay.length ? byDay.map(([d, n]) => `${d} ${n} 件`).join(', ') : '-'}`);
  out.push(`- FL の窓の大きさ(キー名に window と width・height・size を含む項目、または uiWindows(Issue #1364)のある記録): ${recs.filter((r) => hasWindowSizeKey(r.p)).length} 件(ブラウザの表示域 client.viewport は、窓の大きさではない)`);
  out.push('');

  const versions = [...new Map(recs.map((r) => [r.p.bundle?.version ?? '(無し)', true])).keys()];
  const rows = versions.map((v) => versionRow(v, recs.filter((r) => (r.p.bundle?.version ?? '(無し)') === v)));

  out.push('## 版ごと: 記録・セッション・時刻・ブラウザ');
  out.push('');
  out.push(table(['版 (bundle.version)', '記録', 'セッション', '最初 (UTC)', '最後 (UTC)', 'ブラウザ', '失敗の記録'],
    rows.map((x) => [x.version, x.records, x.sessions, x.first, x.last, x.browsers, x.failures])));
  out.push('');
  out.push('## 版ごと: DPR・DPI・画面・起動・途切れ・入力・engine の状態');
  out.push('');
  out.push('DPI は DPR から MulDiv(96, round(DPR × 100), 100) で求めた値。起動は、セッションごとの最初の first-idle の値。途切れは、セッションごとの最後の値の最大。入力は、記録ごとの eventToRaf.p95Ms の中央値。');
  out.push('');
  out.push(table(['版', 'DPR (記録数)', 'DPI (DPR から)', 'dpi の項目', 'guest 画面 (記録数)', '部屋 (CSS)', '起動 中央値 / 最大 (s)', '途切れ 最大', '入力 p95 中央値 (ms)', 'engine.state (記録数)', 'engine.exit (記録数)'],
    rows.map((x) => [x.version, x.dpr, x.dpi, x.dpiField, x.guest, x.room,
      `${f1(x.firstIdle.median)} / ${f1(x.firstIdle.max)} (${x.firstIdle.n} セッション、10 秒超 ${x.firstIdle.over10})`,
      x.underruns.n ? `${x.underruns.max} (${x.underruns.n} セッション)` : '計測なし',
      x.input.n ? `${f1(x.input.median)} (${x.input.n} 記録)` : '-', x.states, x.exits])));
  out.push('');

  const low = recs.filter((r) => dprOf(r.p) !== null && dprOf(r.p) >= 1.7 && dprOf(r.p) < 1.8);
  const lowSess = sessionsOf(low).filter((rs) => rs.some((r) => r.p.guestScreen));
  const scope = day === 'all' ? '全部の日' : `UTC ${day}`;
  const today = lowSess.filter((rs) => day === 'all' || rs[0].t.slice(0, 10) === day);
  out.push(`## DPR 1.7 台のセッション(${scope}。${today.length} セッション)`);
  out.push('');
  out.push('(b) DPI の区分: 96 相当は DPI 96(100%)、169 相当は DPI 160〜179。(c) 物理画素か: guest の画面が、部屋 × DPR の四捨五入に、縦横とも 1 画素以内で近いか。reported は、engine が画面の大きさを返した記録の数。');
  out.push('');
  if (today.length === 0) out.push('該当する記録が無い。');
  else out.push(table(['#', '最初 (UTC)', '最後 (UTC)', '記録', '版', 'ブラウザ', 'DPR', 'DPI (DPR から)', 'DPI の区分', 'guest 画面', '部屋 (CSS)', '部屋 × DPR', '物理画素か', 'reported', '起動 (s)', '途切れ'],
    today.map((rs, i) => sessionRow(i, rs))));
  out.push('');

  out.push('## 記録の形(項目の有無だけ。desk は engine の応答、files は利用の有無で変わるので、版の判定には使わない)');
  out.push('');
  const shapes = counted(recs.map((r) => `${shapeOf(r.p)}${r.p.failure ? ' / failure' : ''}`));
  out.push(table(['項目の有無', '記録', '版', '最初 (UTC)', '最後 (UTC)'], shapes.map(([k, n]) => {
    const rs = recs.filter((r) => `${shapeOf(r.p)}${r.p.failure ? ' / failure' : ''}` === k);
    return [k, n, [...new Set(rs.map((r) => r.p.bundle?.version ?? '(無し)'))].join(', '), when(rs[0].t), when(rs[rs.length - 1].t)];
  })));
  out.push('');
  out.push('注: 版(bundle.version)は engine の web/engine.wasm の sha256 の先頭 12 桁。app だけを変えた版では変わらない(記録の形の違いは、app の版の手がかり)。');
  console.log(out.join('\n'));
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) main();
