// The "Performance" panel (decision 0152): the measured values next to the targets P1-P3 of docs/milestones/performance-targets.md, from the same snapshot the beacon sends.
// A value that misses its target is red; a value is green only when it was measured and meets the target. Nothing measured: a dash and "na". The panel never states a target as reached unless a measured number says so.
import { TARGETS, WASM_MEMORY_LIMIT_BYTES, type InputLatencyStats, type IntervalStats } from './stats.ts';
import type { BrowserInfo, ClientInfo } from './browser-info.ts';
import { DISPLAY_MET_HZ, hzClass, type DisplaySnapshot, type HzInfo } from './display.ts';
import { assetLines, sourceSummary, type AssetsReport } from '../bundle/assets-report.ts';

export type RowStatus = 'miss' | 'met' | 'na';
export interface PerfRow { id: string; label: string; value: string; target: string; status: RowStatus }

export interface PanelView {
  /** seconds since the navigation started, while the engine has been started and has no first idle yet; else null */
  waitingSeconds: number | null;
  firstIdleSeconds: number | null;
  updates: IntervalStats | null;
  input: InputLatencyStats;
  underruns: number | null;
  /** decision 0175: shown after the number (for example "last 60 s: 0, device playing 125 s") */
  underrunsNote?: string;
  /** decision 0170: the engine's wasm memory in bytes as of the last idle report; absent or null: not known */
  wasmMemoryBytes?: number | null;
  server: { sent: number; stored: number; accepted: number; lastNote: string | null; /** decision 0192: the server's refusal: why nothing is sent any more; null: sending */ off?: string | null };
  /** decision 0192: the screen-side measurement, the engine-rate comparison, the machine; absent / null: the display meter is not running */
  display?: DisplaySnapshot | null;
  hz?: HzInfo | null;
  client?: ClientInfo | null;
  browser?: BrowserInfo | null;
  /** decision 0209: how the engine ended and its last words (the text the beacon carries as engine.exit.message); null / absent: it has not ended */
  exit?: { reason: string; code: number; message: string | null } | null;
  /** decision 0220: where the start-up assets came from and where the time went (the beacon's `assets` block); null / absent: not loaded yet */
  assets?: AssetsReport | null;
  /** decision 0209: the APIs that answered their documented failure and let the engine go on (the 10 most frequent); absent / empty: none */
}

const f = (x: number): string => (x < 10 ? x.toFixed(1) : x.toFixed(0));
const frameMs = 1000 / TARGETS.updatesPerSecond;

export function perfRows(v: PanelView): PerfRow[] {
  const rows: PerfRow[] = [];
  // P1
  if (v.firstIdleSeconds !== null) rows.push({ id: 'p1', label: 'P1 start-up: page opened -> engine first idle', value: `${v.firstIdleSeconds.toFixed(1)} s`, target: `<= ${TARGETS.startupSeconds} s`, status: v.firstIdleSeconds <= TARGETS.startupSeconds ? 'met' : 'miss' });
  else if (v.waitingSeconds !== null) rows.push({ id: 'p1', label: 'P1 start-up: page opened -> engine first idle', value: `not yet, ${v.waitingSeconds.toFixed(1)} s so far`, target: `<= ${TARGETS.startupSeconds} s`, status: v.waitingSeconds > TARGETS.startupSeconds ? 'miss' : 'na' });
  else rows.push({ id: 'p1', label: 'P1 start-up: page opened -> engine first idle', value: '-', target: `<= ${TARGETS.startupSeconds} s`, status: 'na' });
  // decision 0220: where the P1 time went for the four assets (not a target of its own)
  rows.push(...assetRows(v.assets ?? null));
  // P2: updates per second and their spacing
  const u = v.updates;
  rows.push({ id: 'p2-rate', label: 'P2 UI updates per second (last 10 s)', value: u && u.perSecond !== null ? `${f(u.perSecond)} /s (${u.n} updates)` : '-', target: `>= ${TARGETS.updatesPerSecond} /s`, status: u && u.perSecond !== null ? (u.perSecond >= TARGETS.updatesPerSecond ? 'met' : 'miss') : 'na' });
  rows.push({ id: 'p2-interval', label: 'P2 update interval, median / p95', value: u && u.intervalMedianMs !== null && u.intervalP95Ms !== null ? `${f(u.intervalMedianMs)} ms / ${f(u.intervalP95Ms)} ms` : '-', target: `p95 <= ${f(frameMs)} ms`, status: u && u.intervalP95Ms !== null ? (u.intervalP95Ms <= frameMs ? 'met' : 'miss') : 'na' });
  // P2: input -> update (decision 0202: from the DOM event's own timeStamp to the next animation frame after the answer was drawn; version 2's listener -> putImageData is shown small, beside it)
  const i = v.input;
  const e = i.eventToRaf, o = i.listenerToPut, w = i.eventToRafWithLate;
  const oldText = o.n > 0 && o.p95Ms !== null && o.p50Ms !== null ? `; the old number (page listener -> putImageData, which leaves out the browser's waits): p95 ${f(o.p95Ms)} ms, p50 ${f(o.p50Ms)} ms` : '';
  // decision 0205: with ?dedupe=1 the number judged is over the answered inputs AND the late ones whose later frame changed pixels (a visible change that came after the engine's first idle is not hidden); the answered-only number is shown beside it.
  // Without dedupe the page cannot tell a changed frame from a redraw, so the late ones are not in it (and the text says so).
  const withLate = w !== null && w.n > 0;
  const j = withLate ? w : e;
  // an input the engine did not answer with a frame has no value in the answered p95; how many there were, and how many of the processed inputs that is, is shown next to it (a p95 that leaves out most inputs would look good for the wrong reason)
  const valueless = i.late + i.noAnswer;
  const lv = i.lateVisible;
  const lateText = lv !== null
    ? `${i.late} late (a visible change after the engine's first idle${lv.n > 0 && lv.p50Ms !== null && lv.p95Ms !== null ? `, ${f(lv.p50Ms)} ms / ${f(lv.p95Ms)} ms p50 / p95 after the event; in the p95 judged` : ''})`
    : `${i.late} late (no frame in the processing window, a first later frame within 150 ms: with dedupe off the page cannot tell if it changed anything, so these are not in the p95)`;
  const answersText = i.processed > 0 ? `; of ${i.processed} processed ${i.answered} have a value, ${valueless} (${(100 * valueless / i.processed).toFixed(0)}%) have none: ${lateText}, ${i.noAnswer} with no frame soon enough` : '';
  const basisText = withLate && e.p95Ms !== null && e.p50Ms !== null ? `, over the answered inputs and the late visible changes; answered inputs only: p95 ${f(e.p95Ms)} ms, p50 ${f(e.p50Ms)} ms (n=${e.n})` : withLate ? ', over the late visible changes (no answered input in the last window)' : '';
  rows.push({
    id: 'p2-input', label: `P2 input -> next update: event.timeStamp -> next animation frame after the draw, p95 of the last ${j.n}`,
    value: j.p95Ms !== null && j.p50Ms !== null ? `${f(j.p95Ms)} ms (p50 ${f(j.p50Ms)} ms${basisText}; ${i.sent} sent, ${i.pending} pending${answersText}${oldText})` : `- (${i.sent} sent, ${i.pending} pending${answersText}${oldText})`,
    target: `p95 <= ${TARGETS.inputToUpdateP95Ms} ms`, status: j.p95Ms !== null ? (j.p95Ms <= TARGETS.inputToUpdateP95Ms ? 'met' : 'miss') : 'na',
  });
  const st = i.stages;
  rows.push({
    id: 'p2-input-stages', label: 'P2 input -> next update: where the time goes (median of each stage)',
    value: st ? `browser wait for the event ${f(st.p50.s0)} + page ${f(st.p50.s0b)} + to the Worker ${f(st.p50.toWorker)} + engine and Worker ${f(st.p50.worker)} + back to the page ${f(st.p50.fromWorker)} + draw ${f(st.p50.put)} + wait for the next animation frame ${f(st.p50.raf)} ms (${st.n} events; the medians do not add up to the median of the sum)` : '-',
    target: '-', status: 'na',
  });
  // P3
  rows.push({ id: 'p3', label: 'P3 audio under-runs', value: v.underruns === null ? 'not measured (the audio output is off or has not run)' : `${v.underruns}${v.underrunsNote ? ` (${v.underrunsNote})` : ''}`, target: '0', status: v.underruns === null ? 'na' : v.underruns === 0 ? 'met' : 'miss' });
  // decision 0170: the engine's wasm memory against the limit of its memory section. Not one of P1-P3 (no target to meet, so never green or red), but the resource the long playback of P3 runs out of
  const mb = (b: number): string => `${(b / 1048576).toFixed(0)} MB`;
  rows.push({
    id: 'wasm-memory', label: 'Engine memory (wasm), of the limit of its memory section',
    value: v.wasmMemoryBytes ? `${mb(v.wasmMemoryBytes)} (${(100 * v.wasmMemoryBytes / WASM_MEMORY_LIMIT_BYTES).toFixed(0)}% of ${mb(WASM_MEMORY_LIMIT_BYTES)})` : '-',
    target: `< ${mb(WASM_MEMORY_LIMIT_BYTES)}`, status: 'na',
  });
  // decision 0192: the screen, the canvases, the page's responsiveness, the machine
  rows.push(...displayRows(v));
  // decision 0209: why the engine stopped, in its own words (the same text the beacon carries)
  rows.push({ id: 'engine-stop', label: 'Engine stop: how it ended, and its last words (sent to the server with the last beacon)', value: v.exit ? `${v.exit.reason}, exit code ${v.exit.code}: ${v.exit.message ?? '(the engine printed no message)'}` : '-', target: '-', status: 'na' });
  // the beacon itself
  const s = v.server;
  rows.push({ id: 'beacon', label: 'Measurement beacon (POST /__perf, this origin only)', value: s.off ? `送っていません: ${s.off}` : s.sent === 0 ? 'nothing sent yet' : `${s.sent} sent, ${s.accepted} accepted, ${s.stored} stored by the server${s.lastNote ? `; last: ${s.lastNote}` : ''}`, target: '-', status: 'na' });
  return rows;
}

/** S-cbeacon1-p95-tol: a screen frame is "on time" up to 10% over one frame of the target rate (a 120 Hz display's timestamps jitter by tenths of a millisecond) */
const FRAME_TOLERANCE = 1.1;
/** S-cbeacon1-dropped-1pct */
const DROPPED_MAX_RATIO = 0.01;
/** S-cbeacon1-fresh: of the screen frames, the share that had new canvas content; judged only while something is moving (S-cbeacon1-idle-na: at least this many draws a second) */
const FRESH_MIN_RATIO = 0.95;
const FRESH_MIN_DRAWS_PER_SECOND = 5;
/** S-cbeacon1-draw-budget: half of one frame of the target rate, 4.2 ms (the shown number is the compared number) */
const DRAW_P95_MAX_MS = 4.2;
/** fewest intervals that make a number (below it: a dash) */
const MIN_INTERVALS = 30;

const pct = (x: number): string => `${(100 * x).toFixed(x < 0.1 ? 1 : 0)}%`;
const ms1 = (x: number | null): string => (x === null ? '-' : `${f(x)} ms`);

/** the sentence that reads a rAF rate: what kind of screen it is, or that the page is not being served */
export function screenNote(hz: number | null, peakHz: number | null): string {
  if (hz === null) return '';
  if (peakHz !== null && peakHz > 1.3 * hz) return `画面は約 ${Math.round(peakHz)} Hz ですが、ページが間に合わず rAF は ${hz.toFixed(0)} 回/s です`;
  if (hz < 50) return `rAF が ${hz.toFixed(0)} 回/s しか来ていません(省電力・高負荷・バックグラウンドの可能性。画面の種類は判定できません)`;
  const c = hzClass(hz);
  if (c === null) return `この画面は ${hz.toFixed(0)} Hz 前後です`;
  return c < TARGETS.updatesPerSecond ? `この画面は ${c} Hz 相当です: ${TARGETS.updatesPerSecond} fps は原理的に出せません` : `この画面は ${c} Hz 相当です`;
}

/** decision 0220: one row for the sources of the start-up assets and one per asset with the read / hash / copy / compile times */
export function assetRows(a: AssetsReport | null): PerfRow[] {
  const label = 'Start-up assets: where the bytes came from (network, the HTTP cache, or the page\'s own stored copy)';
  if (!a || a.items.length === 0) return [{ id: 'assets', label, value: a ? `nothing recorded (stored copy: ${a.store}, hash: ${a.hash === 'subtle' ? 'WebCrypto' : 'JS'})` : '-', target: '-', status: 'na' }];
  const st = a.storage ? `; browser storage ${a.storage.usedMiB ?? '?'} of ${a.storage.quotaMiB ?? '?'} MiB used, ${a.storage.persisted === null ? 'persistence unknown' : a.storage.persisted ? 'protected from eviction' : 'may be evicted when the disk runs low'}` : '';
  const cl = a.clean ? `; old files removed ${a.clean.removed}, kept ${a.clean.kept}${a.clean.busy ? `, in use ${a.clean.busy}` : ''}` : '';
  const rows: PerfRow[] = [{ id: 'assets', label, value: `${sourceSummary(a)} (stored copy: ${a.store}, hash: ${a.hash === 'subtle' ? 'WebCrypto' : 'JS'})${st}${cl}`, target: '-', status: 'na' }];
  assetLines(a).forEach((line, i) => rows.push({ id: `assets-${a.items[i]?.kind ?? i}`, label: `Start-up asset ${a.items[i]?.kind ?? ''}`, value: line, target: '-', status: 'na' }));
  return rows;
}

export function displayRows(v: PanelView): PerfRow[] {
  const rows: PerfRow[] = [];
  const d = v.display ?? null;
  const na = (id: string, label: string, target: string, value = '-'): PerfRow => ({ id, label, value, target, status: 'na' });
  if (!d || !d.rafSupported) {
    rows.push(na('display-hz', '画面の更新周波数(rAF の間隔から推定)', `>= ${DISPLAY_MET_HZ} Hz`, d ? 'requestAnimationFrame を使えません' : '測っていません'));
  } else {
    const r = d.raf.recent10s;
    const enough = r.n >= MIN_INTERVALS;
    rows.push({
      id: 'display-hz', label: '画面の更新周波数(rAF の間隔の中央値から推定、直近 10 秒)',
      value: enough && r.hz !== null ? `${r.hz.toFixed(1)} Hz(間隔の中央値 ${ms1(r.medianMs)}) — ${screenNote(r.hz, r.peakHz)}` : `- (間隔 ${r.n} 個。${MIN_INTERVALS} 個で出します)`,
      target: `>= ${DISPLAY_MET_HZ} Hz(120 fps の前提)`, status: enough && r.hz !== null ? (r.hz >= DISPLAY_MET_HZ ? 'met' : 'miss') : 'na',
    });
    rows.push({
      id: 'raf-rate', label: 'ページが 1 秒に処理した rAF の回数(直近 10 秒。隠れていた時間は除く)',
      value: r.perSecond !== null && r.n > 0 ? `${f(r.perSecond)} /s(間隔 ${r.n} 個)` : '-', target: `>= ${DISPLAY_MET_HZ} /s`,
      status: r.perSecond !== null && r.n > 0 ? (r.perSecond >= DISPLAY_MET_HZ ? 'met' : 'miss') : 'na',
    });
    rows.push({
      id: 'raf-gaps', label: 'rAF の間隔: p95 / 最大(遅いフレーム)',
      value: enough ? `p95 ${ms1(r.p95Ms)} / 最大 ${ms1(r.maxMs)}` : '-', target: `p95 <= ${f(frameMs)} ms`,
      status: enough && r.p95Ms !== null ? (r.p95Ms <= frameMs * FRAME_TOLERANCE ? 'met' : 'miss') : 'na',
    });
    rows.push({
      id: 'raf-dropped', label: '落ちたフレーム(間隔が中央値の 1.5 倍を超えた回数、直近 10 秒)',
      value: enough ? `${r.slow} / ${r.n}(${pct(r.slow / r.n)});12.5 ms 超 ${r.over12p5}` : '-', target: `<= ${(DROPPED_MAX_RATIO * 100).toFixed(0)}%`,
      status: enough ? (r.slow / r.n <= DROPPED_MAX_RATIO ? 'met' : 'miss') : 'na',
    });
  }
  // decision 0202: how the screen meter asks for animation frames
  if (d && d.mode === 'burst' && d.burst) {
    rows.push(na('display-mode', '画面の測定(rAF)の方式', '-', `間欠: 起動時・最初の idle・以後 ${Math.round(d.burst.periodMs / 1000)} 秒ごとに ${(d.burst.onMs / 1000).toFixed(0)} 秒だけ rAF を回します(いままで ${d.burst.count} 回)。バースト外は rAF を要求しません(回し続けるとブラウザがポインタ移動の配信をフレームまで遅らせ、入力の遅延を悪くするため: 決定 0202)。rAF の行の『直近 10 秒』は『直近のバースト』です`));
  } else if (d) rows.push(na('display-mode', '画面の測定(rAF)の方式', '-', '連続: rAF を回し続けます'));
  // the canvases
  if (!d || !d.rafSupported) rows.push(na('canvas-fps', 'canvas が実際に更新された画面フレーム/s', `>= ${TARGETS.updatesPerSecond} /s`));
  else if (!d.canvas) rows.push(na('canvas-fps', 'canvas が実際に更新された画面フレーム/s', `>= ${TARGETS.updatesPerSecond} /s`, '表示方式 A: Worker が描くので、ページからは見えません'));
  else {
    const c = d.canvas.recent10s;
    const enough = c.ticks >= MIN_INTERVALS && c.freshPerSecond !== null;
    rows.push({
      id: 'canvas-fps', label: 'canvas が実際に更新された画面フレーム/s(直近 10 秒)',
      value: enough ? `${f(c.freshPerSecond as number)} /s(1 フレームに平均 ${(c.drawsPerFreshTick ?? 0).toFixed(1)} 回描画;窓の矩形 ${f(c.drawsPerSecond ?? 0)} /s;受信 ${d.canvas.received} 回・描画 ${d.canvas.drawn} 回)` : '-',
      target: `>= ${TARGETS.updatesPerSecond} /s`, status: enough ? ((c.freshPerSecond as number) >= DISPLAY_MET_HZ ? 'met' : 'miss') : 'na',
    });
    const moving = enough && (c.drawsPerSecond ?? 0) >= FRESH_MIN_DRAWS_PER_SECOND;
    rows.push({
      id: 'canvas-fresh', label: 'rAF フレームのうち canvas に新しい絵があった割合(直近 10 秒)',
      value: enough && c.freshRatio !== null ? `${pct(c.freshRatio)}(${c.freshTicks} / ${c.ticks})${moving ? '' : ' — 動いていないので判定しません'}` : '-',
      target: `>= ${(FRESH_MIN_RATIO * 100).toFixed(0)}%(動いている画面で)`, status: moving && c.freshRatio !== null ? (c.freshRatio >= FRESH_MIN_RATIO ? 'met' : 'miss') : 'na',
    });
    const dm = c.drawMs, tm = c.transitMs;
    rows.push({
      id: 'canvas-draw', label: 'Worker のフレームをページが受け取ってから canvas へ描き終えるまで(p50 / p95、直近 10 秒)',
      value: dm ? `p50 ${ms1(dm.p50)} / p95 ${ms1(dm.p95)} / 最大 ${ms1(dm.max)}(${dm.n} 件)${tm ? `;Worker が post してから受け取るまで p50 ${ms1(tm.p50)} / p95 ${ms1(tm.p95)}(2 つの時計の差、目安)` : ''}` : '-',
      target: `p95 <= ${f(DRAW_P95_MAX_MS)} ms`, status: dm && dm.p95 !== null ? (dm.p95 <= DRAW_P95_MAX_MS ? 'met' : 'miss') : 'na',
    });
  }
  // the page's responsiveness
  if (d) {
    const lt = d.longTasks;
    rows.push({
      id: 'longtasks', label: '長いタスク(メインスレッドが 50 ms を超えて塞がった回数、最初の idle 以降)',
      value: !lt.supported ? 'このブラウザでは測れません' : lt.sinceIdle === null ? `最初の idle の前: ${lt.total} 回(最大 ${ms1(lt.maxMs)})` : `${lt.sinceIdle} 回(最大 ${ms1(lt.maxSinceIdleMs)};直近 30 秒 ${lt.recent30s} 回;起動からの合計 ${lt.total} 回)`,
      target: '0 回', status: !lt.supported || lt.sinceIdle === null ? 'na' : lt.sinceIdle === 0 ? 'met' : 'miss',
    });
    const ev = d.events;
    rows.push({
      id: 'slow-events', label: '遅い入力イベント(入力から次の描画まで 16 ms 以上かかったもの。それより短いものは API が報告しません)',
      value: !ev.supported ? 'このブラウザでは測れません' : `${ev.slow} 件 / 送った入力 ${ev.inputsSent} 件${ev.delayMs ? `;入力遅延 p50 ${ms1(ev.delayMs.p50)} / p95 ${ms1(ev.delayMs.p95)}` : ''}${ev.firstInput ? `;最初の入力の遅延 ${ms1(ev.firstInput.delayMs)}` : ''}`,
      target: '0 件', status: !ev.supported ? 'na' : ev.slow === 0 ? 'met' : 'miss',
    });
  }
  // the engine's rate against the screen
  const h = v.hz ?? null;
  rows.push(h ? {
    id: 'hz-fit', label: 'エンジンの描画周波数と画面の更新周波数が合っているか',
    value: `${h.engineHz !== null ? `、エンジン ${h.engineHz} Hz` : ''}${h.autoMeasuredHz !== null ? `(auto の測定 ${h.autoMeasuredHz.toFixed(1)} Hz)` : ''} / 画面 rAF ${h.rafHz !== null ? `${h.rafHz.toFixed(1)} Hz` : '-'} — ${h.note}`,
    target: '一致', status: h.fit === 'match' ? 'met' : h.fit === 'unknown' ? 'na' : 'miss',
  } : na('hz-fit', 'エンジンの描画周波数と画面の更新周波数が合っているか', '一致'));
  if (d) {
    const vis = d.visibility;
    rows.push(na('visibility', '見えていた時間 / 隠れていた時間(隠れている間は rAF が止まるので fps に入れません)', '-', `見えていた ${(vis.visibleMs / 1000).toFixed(0)} s / 隠れていた ${(vis.hiddenMs / 1000).toFixed(0)} s(切り替え ${vis.changes} 回、いまは${vis.visibleNow ? '見えている' : '隠れている'})`));
  }
  const c = v.client ?? null, b = v.browser ?? null;
  if (c || b) {
    const parts: string[] = [];
    if (c) parts.push(`${c.browser.name} ${c.browser.version ?? '?'}${c.browser.os ? `(${c.browser.os}${c.browser.headless ? '、headless' : ''})` : ''}`);
    if (b) parts.push(`論理コア ${b.hardwareConcurrency ?? '?'}(物理コア数は Web API で取れません)`, `メモリの目安 ${b.deviceMemoryGB ?? '?'} GB`);
    if (b?.screen) parts.push(`画面 ${b.screen.width}x${b.screen.height} 拡大率 ${b.devicePixelRatio ?? '?'}${c?.colorDepth ? `、${c.colorDepth} bit` : ''}`);
    if (c?.viewport) parts.push(`ページの表示領域 ${c.viewport.width}x${c.viewport.height}`);
    if (b) parts.push(`GPU ${b.webglRenderer ?? '(取れません)'}`, `cross-origin isolated ${b.crossOriginIsolated ?? '?'}`);
    rows.push(na('machine', 'このブラウザ・この機械(サーバーのログに送られる値)', '-', parts.join(' · ')));
  }
  return rows;
}

/** the paragraphs shown above the table: whose numbers these are, what is sent where, what is not, how to stop it (decision 0192) */
export function perfNotice(): string[] {
  return [
    'この表の値は、いまこのページを開いているあなたのブラウザ・あなたの機械で実測した値です。同じ値が、このページを配信しているサーバーに送られます(手元の vite のサーバーならサーバーのログ perf-log.jsonl、Cloudflare Workers なら R2 の保存先 perf/)。送る頻度は、最初の idle で 1 回、その後 30 秒ごとに最大 20 回、終了時に 1 回です。',
    '送る項目: 起動の各段階の時刻、窓の更新の回数と間隔、入力から更新までの遅延、画面の更新周波数(requestAnimationFrame の間隔)、canvas の更新と描画にかかった時間、長いタスクと遅い入力の回数、ページが見えていた時間、描画周波数、ブラウザの名前と版・OS 名・論理コア数・メモリの目安・画面と表示領域の大きさ・色深度・拡大率・GPU の名前(取れれば)・cross-origin isolated・エンジンの wasm メモリ・起動に使う 4 つのデータの由来(ネットワーク/ブラウザの HTTP キャッシュ/このページの保存コピー)と転送バイト数と読み・照合・コピー・書き込みの時間・保存領域の使用量と上限と永続化の状態(名前・パスは送りません)・エンジンが止まったときのエンジン自身の最後のメッセージ(最大 400 文字。ホームディレクトリ・ファイルの名前・メールアドレス・URL・IP アドレスは送る前に除いています)・止まらずに文書どおりの失敗を返した API の名前と回数(上位 10 件。ソフトストップの版のエンジンだけ)。',
    '送らないもの: IP アドレス(サーバーは相手が tailnet か手元かの種類だけを記録します)、Cookie、入力した文字、URL、ファイルの中身。送り先はこのページを配信したサーバーだけで、外部への通信はありません。物理コア数はブラウザから取れないので送れません(論理コア数だけ)。',
  ];
}

export function renderPerfPanel(root: HTMLElement, rows: readonly PerfRow[], notice: readonly string[] = []): void {
  const doc = root.ownerDocument;
  const table = doc.createElement('table');
  table.className = 'perf-table';
  const head = doc.createElement('tr');
  for (const t of ['measured', 'value', 'target']) { const th = doc.createElement('th'); th.textContent = t; head.append(th); }
  table.append(head);
  for (const r of rows) {
    const tr = doc.createElement('tr');
    tr.dataset.row = r.id;
    tr.dataset.status = r.status;
    for (const t of [r.label, r.value, r.target]) { const td = doc.createElement('td'); td.textContent = t; tr.append(td); }
    table.append(tr);
  }
  const kids: Node[] = [];
  if (notice.length) { const n = doc.createElement('div'); n.className = 'perf-notice'; for (const t of notice) { const p = doc.createElement('p'); p.textContent = t; n.append(p); } kids.push(n); }
  kids.push(table);
  root.replaceChildren(...kids);
}
