// lat1: tables from the result-*.json files of run.mjs. usage: node tools/analyze.mjs RUNS_DIR OUT_DIR
import fs from 'node:fs';
import path from 'node:path';

const runsDir = process.argv[2], outDir = process.argv[3];
const MAXLOAD = process.env.MAXLOAD ? Number(process.env.MAXLOAD) : Infinity;   // only cells that started with a 1-minute load average at or below this (the machine is shared; decision 0194 section 2.4)
fs.mkdirSync(outDir, { recursive: true });
const files = [];
(function walk(d) { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const p = path.join(d, e.name); if (fs.statSync(p).isDirectory()) walk(p); else if (/^result-.*\.json$/.test(e.name)) files.push(p); } })(runsDir);
const runs = files.map((f) => ({ file: f, ...JSON.parse(fs.readFileSync(f, 'utf8')) })).sort((a, b) => a.config.localeCompare(b.config) || a.round - b.round);
for (const r of runs) r.cells = r.cells.filter((c) => !(c.inputs || c.cell.startsWith('background')) || c.load0 <= MAXLOAD);

const pct = (v, p) => { v = v.filter((x) => x !== null && x !== undefined && !Number.isNaN(x)).sort((a, b) => a - b); if (!v.length) return null; return v[Math.min(v.length - 1, Math.max(0, Math.ceil((p / 100) * v.length) - 1))]; };
const f1 = (x) => (x === null || x === undefined ? '-' : x >= 100 ? x.toFixed(0) : x.toFixed(1));
const stat = (v) => ({ n: v.filter((x) => x !== null).length, p50: pct(v, 50), p95: pct(v, 95), max: pct(v, 100) });
const med = (v) => pct(v, 50);

// label each input row of a cell: which role it plays
function label(cellName, inputs) {
  const rows = inputs.map((r) => ({ ...r }));
  let keyIdx = 0, inDrag = false;
  for (const r of rows) {
    const d = r.desc;
    if (cellName.startsWith('c_drag')) {
      if (d.startsWith('button L down')) { r.role = 'down'; inDrag = true; }
      else if (d.startsWith('button L up')) { r.role = 'up'; inDrag = false; }
      else if (d.startsWith('move')) r.role = inDrag ? 'drag-move' : 'position-move';
      else r.role = 'other';
    } else if (cellName === 'b_hover') r.role = d.startsWith('move') ? 'hover-move' : 'other';
    else if (cellName === 'a_click_step') r.role = d.startsWith('button L down') ? 'down' : d.startsWith('button L up') ? 'up' : d.startsWith('move') ? 'position-move' : 'other';
    else if (cellName === 'd_wheel_tempo') r.role = d.startsWith('wheel') ? 'wheel' : d.startsWith('move') ? 'position-move' : 'other';
    else if (cellName === 'f_open_file_menu') r.role = d.startsWith('button L down') ? 'open(down)' : d.startsWith('button L up') ? 'up' : d.startsWith('key down') ? 'Escape(close)' : d.startsWith('move') ? 'position-move' : 'other';
    else if (cellName === 'f_open_context_menu') r.role = d.startsWith('button R down') ? 'open(down)' : d.startsWith('button R up') ? 'up' : d.startsWith('key down') ? 'Escape(close)' : d.startsWith('move') ? 'position-move' : 'other';
    else if (cellName === 'e_key_F8_open_close') r.role = d.startsWith('key down vk 0x77') ? 'F8(open)' : d.startsWith('key down vk 0x1b') ? 'Escape(close)' : 'other';
    else if (cellName === 'e_key_Space_start_stop') { if (d.startsWith('key down vk 0x20')) r.role = keyIdx++ % 2 === 0 ? 'Space(start)' : 'Space(stop)'; else r.role = 'other'; }
    else r.role = 'other';
  }
  return rows;
}

const STAGE_KEYS = ['s0_evt_to_handler', 's0b_handler_to_page_send', 's1_page_to_worker_recv', 's1b_worker_recv_to_inject', 's2_inject_to_first_post', 's2b_inject_to_idle', 's3a_post_to_page_recv', 's3b_recv_to_put', 's4a_put_to_raf', 's4b_raf_to_rafdone', 'total_event_to_put', 'total_event_to_raf', 'total_event_to_rafdone'];

// ---- rejoin offline: the answer to an input is the first CHANGING draw (outside the background classes) that the Worker posted between the input's injection and the engine's next idle (+1 ms: the idle's own draws are posted just before that instant);
// a draw within the next 150 ms is a 'late' answer (counted apart); otherwise there is no answer. Decision 0194 S-lat1-resp. (The run's own join took the first changing draw within 1.5 s, which for a release or an Escape found the next periodic redraw.)
const IN_MARGIN = 1.0, LATE_MS = 150;
function rejoin(c, list) {
  for (const r of list) {
    const t0 = r.s0b_handler_to_page_send !== null ? r.tPage - r.s0b_handler_to_page_send - (r.s0_evt_to_handler ?? 0) : r.tPage;
    const tIdle = r.s2b_inject_to_idle !== null ? r.tInject + r.s2b_inject_to_idle : null;
    const cands = c.draws.filter((d) => d.resp && d.post >= r.tInject - 0.01);
    let d = null, kind = 'none';
    const stream = c.cell.startsWith('c_drag') && (r.role === 'drag-move' || r.role === 'down');
    if (stream) {
      // S-lat1-resp2: inside a drag every move is answered by a later cycle's repaint, so the answer is the first changing draw posted after the move's injection (within 500 ms)
      d = cands.find((x) => x.post <= r.tInject + 500) ?? null;
      if (d) kind = 'in';
    } else if (tIdle !== null) {
      d = cands.find((x) => x.post <= tIdle + IN_MARGIN) ?? null;
      if (d) kind = 'in';
      else { d = cands.find((x) => x.post > tIdle + IN_MARGIN && x.post <= tIdle + LATE_MS) ?? null; if (d) kind = 'late'; }
    }
    const any = c.draws.find((x) => x.post >= r.tInject - 0.01);
    r.appStyle = any ? any.put - r.tPage : null;   // what the app's LatencyTracker counts: the first frame of any kind posted after the injection (an unchanged repaint included), page handler -> drawn
    r.resp = kind;
    r.rect2 = d ? d.r : null;
    r.s2_inject_to_first_post = d ? d.post - r.tInject : null;
    r.s3a_post_to_page_recv = d ? d.recv - d.post : null;
    r.s3b_recv_to_put = d ? d.put - d.recv : null;
    r.s4a_put_to_raf = d && d.raf !== null ? d.raf - d.put : null;
    r.total_event_to_put = d ? d.put - t0 : null;
    r.respFromPage = d ? d.put - r.tPage : null;
    r.total_event_to_raf = d && d.raf !== null ? d.raf - t0 : null;
    r.s4b_raf_to_rafdone = null; r.total_event_to_rafdone = null;
  }
}

// ---- flatten: one record per input row ----
const rows = [];
for (const run of runs) {
  for (const c of run.cells) {
    if (!c.inputs) continue;
    const labeled = label(c.cell, c.inputs);
    rejoin(c, labeled);
    for (const r of labeled) rows.push({ config: run.config, round: run.round, phase: c.phase, cell: c.cell, load: c.load0, ...r });
  }
}
const groups = new Map();
for (const r of rows) { const k = [r.config, r.phase, r.cell, r.role].join('|'); if (!groups.has(k)) groups.set(k, []); groups.get(k).push(r); }
const summary = [];
for (const [k, v] of groups) {
  const [config, phase, cell, role] = k.split('|');
  if (role === 'position-move' || role === 'other') continue;
  const resp = v.filter((x) => x.resp === 'in');
  const o = { config, phase, cell, role, n: v.length, nResp: resp.length, nLate: v.filter((x) => x.resp === 'late').length, loadMax: Math.max(...v.map((x) => x.load)), rounds: [...new Set(v.map((x) => x.round))].join(',') };
  for (const sk of STAGE_KEYS) { const s = stat(resp.map((x) => x[sk])); o[sk] = s; }
  o.appStyle = stat(v.map((x) => x.appStyle));
  o.respFromPage = stat(resp.map((x) => x.respFromPage));
  o.popup = stat(v.map((x) => x.popup_total_from_event));
  o.popupInject = stat(v.map((x) => x.popup_inject_to_post));
  o.tInjectToIdle = stat(v.map((x) => x.s2b_inject_to_idle));
  summary.push(o);
}
fs.writeFileSync(path.join(outDir, 'summary.json'), JSON.stringify(summary, null, 1));

const order = ['a_click_step', 'b_hover', 'c_drag_knob', 'c_drag_fader', 'd_wheel_tempo', 'e_key_Space_start_stop', 'e_key_F8_open_close', 'f_open_file_menu', 'f_open_context_menu'];
const sortS = (a, b) => order.indexOf(a.cell) - order.indexOf(b.cell) || a.role.localeCompare(b.role);
const md = [];
const cfgs = [...new Set(runs.map((r) => r.config))];
md.push('# lat1 tables (generated by tools/analyze.mjs)\n');
md.push(`cells filtered by load0 <= ${MAXLOAD}\n`);
md.push(`runs: ${runs.map((r) => `${r.config}-r${r.round}${r.ok ? '' : ' (INCOMPLETE: ' + (r.error || '').slice(0, 60) + ')'}`).join(', ')}\n`);

for (const phase of ['idle', 'play']) {
  for (const cfg of cfgs) {
    const S = summary.filter((s) => s.config === cfg && s.phase === phase).sort(sortS);
    if (!S.length) continue;
    md.push(`\n## ${cfg} / ${phase}: from the input event to the first changing draw (ms; event.timeStamp -> putImageData end | -> next rAF callback start)\n`);
    md.push('| cell | role | n | answered in the processing window | late (<150 ms after) | event->put p50 | p95 | max | event->rAF p50 | p95 | max | load max |');
    md.push('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|');
    for (const s of S) md.push(`| ${s.cell} | ${s.role} | ${s.n} | ${s.nResp} | ${s.nLate} | ${f1(s.total_event_to_put.p50)} | ${f1(s.total_event_to_put.p95)} | ${f1(s.total_event_to_put.max)} | ${f1(s.total_event_to_raf.p50)} | ${f1(s.total_event_to_raf.p95)} | ${f1(s.total_event_to_raf.max)} | ${s.loadMax} |`);
  }
}

// stage medians
for (const phase of ['idle', 'play']) {
  for (const cfg of cfgs) {
    const S = summary.filter((s) => s.config === cfg && s.phase === phase).sort(sortS);
    if (!S.length) continue;
    md.push(`\n## stage medians (ms), ${cfg} / ${phase}\n`);
    md.push('| cell | role | evt->handler | handler->send | page->Worker recv | recv->inject | inject->first post | inject->idle | post->page recv | recv->put | put->rAF | rAF->task | total evt->put | total evt->rAF |');
    md.push('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|');
    for (const s of S) md.push(`| ${s.cell} | ${s.role} | ${STAGE_KEYS.slice(0, 10).map((k) => f1(s[k].p50)).join(' | ')} | ${f1(s.total_event_to_put.p50)} | ${f1(s.total_event_to_raf.p50)} |`);
  }
}

// config comparison: p50 and p95 event->rAF, per cell/role, idle and play, with the per-round p50
md.push('\n## configuration comparison (event -> next rAF, ms): p50 / p95 [per-round p50]\n');
for (const phase of ['idle', 'play']) {
  md.push(`\n### ${phase}\n`);
  md.push(`| cell | role | ${cfgs.map((c) => c).join(' | ')} |`);
  md.push(`|---|---|${cfgs.map(() => '---').join('|')}|`);
  const keys = [...new Set(summary.filter((s) => s.phase === phase).map((s) => s.cell + '|' + s.role))].sort((a, b) => { const [ca, ra] = a.split('|'), [cb, rb] = b.split('|'); return order.indexOf(ca) - order.indexOf(cb) || ra.localeCompare(rb); });
  for (const k of keys) {
    const [cell, role] = k.split('|');
    const cells = cfgs.map((cfg) => {
      const s = summary.find((x) => x.config === cfg && x.phase === phase && x.cell === cell && x.role === role);
      if (!s) return '-';
      const per = [...new Set(rows.filter((r) => r.config === cfg && r.phase === phase && r.cell === cell && r.role === role).map((r) => r.round))].map((rd) => f1(med(rows.filter((r) => r.config === cfg && r.phase === phase && r.cell === cell && r.role === role && r.round === rd).map((r) => r.total_event_to_raf))));
      return `${f1(s.total_event_to_raf.p50)} / ${f1(s.total_event_to_raf.p95)} (n ${s.nResp}/${s.n}) [${per.join(', ')}]`;
    });
    md.push(`| ${cell} | ${role} | ${cells.join(' | ')} |`);
  }
}

// guest side: inject -> idle, inject -> first post, for the same comparison
md.push('\n## the guest side (ms): inject -> engine idle again p50 / p95, and inject -> first changing frame posted p50 / p95\n');
for (const phase of ['idle', 'play']) {
  md.push(`\n### ${phase}\n`);
  md.push(`| cell | role | ${cfgs.map((c) => c).join(' | ')} |`);
  md.push(`|---|---|${cfgs.map(() => '---').join('|')}|`);
  const keys = [...new Set(summary.filter((s) => s.phase === phase).map((s) => s.cell + '|' + s.role))].sort((a, b) => { const [ca, ra] = a.split('|'), [cb, rb] = b.split('|'); return order.indexOf(ca) - order.indexOf(cb) || ra.localeCompare(rb); });
  for (const k of keys) {
    const [cell, role] = k.split('|');
    const cells = cfgs.map((cfg) => {
      const s = summary.find((x) => x.config === cfg && x.phase === phase && x.cell === cell && x.role === role);
      if (!s) return '-';
      return `idle ${f1(s.tInjectToIdle.p50)} / ${f1(s.tInjectToIdle.p95)}; post ${f1(s.s2_inject_to_first_post.p50)} / ${f1(s.s2_inject_to_first_post.p95)}`;
    });
    md.push(`| ${cell} | ${role} | ${cells.join(' | ')} |`);
  }
}

// tails of the stages
md.push('\n## stage tails (p95 / max, ms): where the slow ones lose their time\n');
md.push('| config | phase | cell | role | S0 evt->handler | S1 page->Worker | S2 inject->first post | S4a put->rAF | total evt->rAF |');
md.push('|---|---|---|---|---|---|---|---|---|');
for (const s of summary.slice().sort((a, b) => a.config.localeCompare(b.config) || a.phase.localeCompare(b.phase) || sortS(a, b))) {
  const t = (k) => `${f1(s[k].p95)} / ${f1(s[k].max)}`;
  md.push(`| ${s.config} | ${s.phase} | ${s.cell} | ${s.role} | ${t('s0_evt_to_handler')} | ${t('s1_page_to_worker_recv')} | ${t('s2_inject_to_first_post')} | ${t('s4a_put_to_raf')} | ${t('total_event_to_raf')} |`);
}

// the app's own measure
md.push('\n## the app\'s own measure (LatencyTracker: page handler -> the first frame of ANY kind posted after the injection, an unchanged repaint included) against the first CHANGING draw (event -> put), ms p50 / p95\n');
md.push('| config | phase | cell | role | n | app-style (handler -> first frame of any kind) p50 / p95 / max | first CHANGING draw, same start (handler) p50 / p95 / max | first changing draw from the event timeStamp p50 / p95 / max |');
md.push('|---|---|---|---|---:|---|---|---|');
for (const s of summary.filter((x) => ['down', 'hover-move', 'drag-move', 'wheel', 'Space(start)', 'open(down)', 'F8(open)'].includes(x.role)).sort((a, b) => a.config.localeCompare(b.config) || a.phase.localeCompare(b.phase) || sortS(a, b))) md.push(`| ${s.config} | ${s.phase} | ${s.cell} | ${s.role} | ${s.n} | ${f1(s.appStyle.p50)} / ${f1(s.appStyle.p95)} / ${f1(s.appStyle.max)} | ${f1(s.respFromPage.p50)} / ${f1(s.respFromPage.p95)} / ${f1(s.respFromPage.max)} | ${f1(s.total_event_to_put.p50)} / ${f1(s.total_event_to_put.p95)} / ${f1(s.total_event_to_put.max)} |`);

// popups
md.push('\n## windows that open (the first changing draw on a window other than the main one, for the inputs that open one; ms from the input event; closing inputs are not listed: the next repetition\'s window would be taken for their answer)\n');
md.push('| config | phase | cell | role | n | with popup | event->popup put p50 | p95 | max | inject->popup post p50 | p95 |');
md.push('|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|');
for (const s of summary.filter((x) => (x.role === 'open(down)' || x.role === 'F8(open)') && x.popup.n > 0).sort((a, b) => a.config.localeCompare(b.config) || a.phase.localeCompare(b.phase) || sortS(a, b))) md.push(`| ${s.config} | ${s.phase} | ${s.cell} | ${s.role} | ${s.n} | ${s.popup.n} | ${f1(s.popup.p50)} | ${f1(s.popup.p95)} | ${f1(s.popup.max)} | ${f1(s.popupInject.p50)} | ${f1(s.popupInject.p95)} |`);

// background and CPU
md.push('\n## background (no input) and Worker CPU\n');
md.push('| config | round | phase | draws/s | changing draws/s | Worker cores | page main cores | all Chrome cores | load |');
md.push('|---|---:|---|---:|---:|---:|---:|---:|---:|');
for (const run of runs) for (const c of run.cells.filter((x) => x.cell.startsWith('background'))) md.push(`| ${run.config} | ${run.round} | ${c.phase} | ${c.drawsPerS} | ${c.changingPerS} | ${(c.cpu.workerS / c.wallS).toFixed(3)} | ${(c.cpu.mainS / c.wallS).toFixed(3)} | ${(c.cpu.allS / c.wallS).toFixed(3)} | ${c.load0} |`);

// marginal Worker CPU per input: (cell Worker CPU - background rate * cell wall) / inputs that are primary
md.push('\n## Worker CPU per cell (marginal over the background rate of the same run and phase)\n');
md.push('| config | round | phase | cell | wall s | Worker CPU s | marginal CPU s | primary inputs | marginal CPU ms per primary input | page main CPU s |');
md.push('|---|---:|---|---|---:|---:|---:|---:|---:|---:|');
const primary = { a_click_step: 'down', b_hover: 'hover-move', c_drag_knob: 'drag-move', c_drag_fader: 'drag-move', d_wheel_tempo: 'wheel', e_key_Space_start_stop: 'Space(start)', e_key_F8_open_close: 'F8(open)', f_open_file_menu: 'open(down)', f_open_context_menu: 'open(down)' };
for (const run of runs) {
  const bgc = Object.fromEntries(run.cells.filter((x) => x.cell.startsWith('background')).map((x) => [x.phase, x.cpu.workerS / x.wallS]));
  const bgm = Object.fromEntries(run.cells.filter((x) => x.cell.startsWith('background')).map((x) => [x.phase, x.cpu.mainS / x.wallS]));
  for (const c of run.cells.filter((x) => x.inputs)) {
    const nPrim = label(c.cell, c.inputs).filter((r) => r.role === primary[c.cell]).length;
    const marg = c.cpu.workerS - (bgc[c.phase] ?? 0) * c.wallS;
    md.push(`| ${run.config} | ${run.round} | ${c.phase} | ${c.cell} | ${c.wallS} | ${c.cpu.workerS} | ${marg.toFixed(2)} | ${nPrim} | ${nPrim ? ((marg / nPrim) * 1000).toFixed(0) : '-'} | ${c.cpu.mainS} |`);
  }
}

// drag stream
md.push('\n## continuous drag (press, 8 moves up and 8 back at 16 ms, release): the stream\n');
md.push('| cell | config | phase | drags | moves | moves answered by a changing draw within 500 ms | per-move event->put p50 / p95 / max (ms) | per-move event->rAF p50 / p95 (ms) | changing draws during a drag (median) | last move injected -> last changing draw posted within the next 150 ms, median / p95 / max (ms) |');
md.push('|---|---|---|---:|---:|---:|---|---|---:|---|');
for (const cellName of ['c_drag_knob', 'c_drag_fader']) for (const cfg of cfgs) for (const phase of ['idle', 'play']) {
  const lat = [], latR = [], tail = [], perDrag = [];
  let drags = 0, moves = 0, ans = 0;
  for (const run of runs.filter((r) => r.config === cfg)) {
    const c = run.cells.find((x) => x.cell === cellName && x.phase === phase);
    if (!c) continue;
    const ins = label(c.cell, c.inputs);
    rejoin(c, ins);
    let cur = null; const list = [];
    for (const r of ins) { if (r.role === 'down') { cur = { down: r, moves: [] }; list.push(cur); } else if (r.role === 'drag-move' && cur) cur.moves.push(r); else if (r.role === 'up' && cur) { cur.up = r; cur = null; } }
    for (const d of list) {
      drags++; moves += d.moves.length;
      for (const m of d.moves) if (m.total_event_to_put !== null) { ans++; lat.push(m.total_event_to_put); if (m.total_event_to_raf !== null) latR.push(m.total_event_to_raf); }
      const tStart = d.down.tInject, lastMove = d.moves[d.moves.length - 1] ?? d.down;
      const mine = c.draws.filter((x) => x.resp && x.post >= tStart - 1 && x.post <= lastMove.tInject + 150);
      perDrag.push(mine.filter((x) => x.post <= lastMove.tInject + 5).length);
      if (mine.length) tail.push(mine[mine.length - 1].post - lastMove.tInject);
    }
  }
  if (drags) md.push(`| ${cellName} | ${cfg} | ${phase} | ${drags} | ${moves} | ${ans} | ${f1(pct(lat, 50))} / ${f1(pct(lat, 95))} / ${f1(pct(lat, 100))} | ${f1(pct(latR, 50))} / ${f1(pct(latR, 95))} | ${f1(med(perDrag))} | ${f1(pct(tail, 50))} / ${f1(pct(tail, 95))} / ${f1(pct(tail, 100))} |`);
}

// frame cadence of the background (no input) stretches: what the page can see of the frame rate (decision 0194 section on 120 fps)
md.push('\n## frame cadence without input (the Worker posts frames; the page draws them), from the background cells\n');
md.push('| config | round | phase | s | draws/s (all) | gap Worker-post p50 / p95 / max (ms) | changing draws/s | gap between CHANGING draws p50 / p95 / max (ms) | recv->put p50 / p95 / max (ms) | post->recv p50 / p95 / max (ms) |');
md.push('|---|---:|---|---:|---:|---|---:|---|---|---|');
for (const run of runs) for (const c of run.cells.filter((x) => x.cell.startsWith('background') && x.draws2)) {
  const d = c.draws2.slice().sort((a, b) => a.post - b.post);
  const gaps = (arr) => { const g = []; for (let i = 1; i < arr.length; i++) g.push(arr[i].post - arr[i - 1].post); return g; };
  const ch = d.filter((x) => x.chg > 0);
  const q = (v) => `${f1(pct(v, 50))} / ${f1(pct(v, 95))} / ${f1(pct(v, 100))}`;
  md.push(`| ${run.config} | ${run.round} | ${c.phase} | ${c.wallS} | ${(d.length / c.wallS).toFixed(1)} | ${q(gaps(d))} | ${(ch.length / c.wallS).toFixed(1)} | ${q(gaps(ch))} | ${q(d.map((x) => x.put - x.recv))} | ${q(d.map((x) => x.recv - x.post))} |`);
}
fs.writeFileSync(path.join(outDir, 'tables.md'), md.join('\n') + '\n');
console.log(`${runs.length} runs, ${rows.length} input rows, ${summary.length} groups -> ${outDir}/tables.md`);
