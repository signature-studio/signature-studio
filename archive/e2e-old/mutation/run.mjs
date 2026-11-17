// Mutation check: for each mutation, copy of the work tree -> change ONE thing -> run the tests -> they must fail -> change it back (the file is restored from the pristine copy).
// The mutations are made in a COPY of app/ (MUT_DIR, default <work>/mut/app, node_modules symlinked); the real tree is never touched.
// usage: node test/mutation/run.mjs [--only id,id] [--list]
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { APP, workDirOf } from '../helpers/wrangler.mjs';

const argv = process.argv.slice(2);
const only = argv.includes('--only') ? argv[argv.indexOf('--only') + 1].split(',') : null;
const onlyLayers = argv.includes('--layers') ? argv[argv.indexOf('--layers') + 1].split(',') : null;
const repoRoot = path.resolve(APP, '..');
const work = workDirOf('mut');
const copy = path.join(work, 'app');
const logsDir = process.env.APP_LOGS || process.env.APP0_LOGS || path.join(work, 'logs');
fs.mkdirSync(logsDir, { recursive: true });

// layers: unit = vitest (test/unit), worker = wrangler dev --local + vitest (test/worker), e2e = Chrome against `vite preview` (and the Worker)
const M = [
  { id: 'range-end-off-by-one', file: 'shared/range.ts', find: 'const end = b === \'\' ? size - 1 : Math.min(Number(b), size - 1);', replace: 'const end = b === \'\' ? size - 1 : Math.min(Number(b) + 1, size - 1);', layers: ['unit', 'worker'] },
  { id: 'coep-removed', file: 'shared/headers.ts', find: "  'Cross-Origin-Embedder-Policy': 'require-corp',\n", replace: '', layers: ['unit', 'worker'] },
  { id: 'jwt-aud-not-checked', file: 'worker/access.ts', find: "if (!auds.includes(cfg.aud)) throw new AccessError('the token is for another application (aud)');", replace: '', layers: ['unit', 'worker'] },
  { id: 'jwt-exp-not-checked', file: 'worker/access.ts', find: "if (typeof payload.exp !== 'number' || payload.exp + LEEWAY_S < nowS) throw new AccessError('the token has expired');", replace: '', layers: ['unit', 'worker'] },
  { id: 'manifest-sha-mismatch-ignored', file: 'src/bundle/loader.ts', find: 'if (got !== ref.sha256) {', replace: 'if (false) {', layers: ['unit'] },
  { id: 'if-none-match-ignored-by-worker', file: 'worker/index.ts', find: "if (ifNoneMatchHit(request.headers.get('If-None-Match'), head.httpEtag)) return new Response(null, { status: 304, headers: h });", replace: '', layers: ['worker'] },
  { id: 'r2-content-encoding-not-sent', file: 'worker/index.ts', find: "if (enc) h.set('Content-Encoding', enc);", replace: '', layers: ['worker'] },
  { id: 'worker-416-not-returned', file: 'worker/index.ts', find: "if (range.kind === 'unsatisfiable') {", replace: "if (false as boolean) {", layers: ['worker'] },
  { id: 'x-robots-tag-removed', file: 'shared/headers.ts', find: "  'X-Robots-Tag': 'noindex, nofollow',\n", replace: '', layers: ['unit', 'worker'] },
  { id: 'sha256-padding-bit', file: 'src/bundle/sha256.ts', find: 'pad[this.blockLen] = 0x80;', replace: 'pad[this.blockLen] = 0x81;', layers: ['unit'] },
  { id: 'pack-offset-off-by-one', file: 'src/bundle/pack.ts', find: 'const data = bytes.subarray(payload + offset, payload + offset + size);', replace: 'const data = bytes.subarray(payload + offset + 1, payload + offset + size + 1);', layers: ['unit'] },
  { id: 'flseedp1-data-region-off-by-one', file: 'src/bundle/pack.ts', find: 'const path = e.path as string, off = base + (e.offset as number);', replace: 'const path = e.path as string, off = base + (e.offset as number) + 1;', layers: ['unit'] },
  { id: 'external-url-allowed-in-manifest', file: 'src/bundle/schema.ts', find: "if (/^[A-Za-z][A-Za-z0-9+.-]*:/.test(s)) return false;\n  if (s.startsWith('//') || s.includes('\\\\') || /[\\u0000-\\u001f]/.test(s)) return false;", replace: '', layers: ['unit'] },
  // ---- F1 (decision 0137): the real engine's adapter, the start-up display, the display scaling and the input mapping ----
  { id: 'fruity-env-not-set', file: 'src/engine/fruity.ts', find: 'for (const [k, v] of Object.entries(manifest.engine.env)) opts.ENV[k] = v;', replace: '', layers: ['unit'] },
  { id: 'fruity-exit-code-lost', file: 'src/engine/fruity.ts', find: "onExit: (code) => { this.exitCode = code; host.mark('exit', { code }); },", replace: "onExit: () => { this.exitCode = 0; host.mark('exit', { code: 0 }); },", layers: ['unit'] },
  { id: 'jspi-detection-removed', file: 'src/engine/features.ts', find: "const jspi = !!wa && has(wa.Suspending) && has(wa.promising);", replace: 'const jspi = true;', layers: ['unit'] },
  { id: 'glue-sha-not-checked', file: 'src/main.ts', find: "fetchAsset(fopts.glue, 'engine glue', opts),", replace: "fetch(fopts.glue.url).then(async (r) => new Uint8Array(await r.arrayBuffer())),", layers: ['e2e'], e2eOnly: 'err-engine-badsha-glue' },
  { id: 'stop-wall-not-named', file: 'src/ui/stop-message.ts', find: "const wall = f.runtimeMessage.startsWith('fle:');", replace: 'const wall = false;', layers: ['unit'] },
  { id: 'stop-message-forgets-the-last-input', file: 'src/ui/stop-message.ts', find: "if (f.lastInput) lines.push(", replace: "if (false as boolean && f.lastInput) lines.push(", layers: ['unit'] },
  { id: 'stage-first-pixel-never-done', file: 'src/ui/phases.ts', find: "const firstPixel = row('first-pixel', 'First pixel', eStart, first,", replace: "const firstPixel = row('first-pixel', 'First pixel', eStart, null,", layers: ['unit'] },
  { id: 'pointer-ignores-css-scale', file: 'src/input/tracker.ts', find: 'const fx = (clientX - rect.left) / rect.width, fy = (clientY - rect.top) / rect.height;', replace: 'const fx = (clientX - rect.left) / screen.width, fy = (clientY - rect.top) / screen.height;', layers: ['unit'] },
  { id: 'log-parser-takes-the-first-window-table', file: 'src/engine/fruity-log.ts', find: "if (WINDOW_HEADER.test(l)) { windows = []; out.windows = windows; continue; }", replace: "if (WINDOW_HEADER.test(l)) { if (windows) continue; windows = []; out.windows = windows; continue; }", layers: ['unit'] },
  { id: 'manifest-root-allows-dotdot', file: 'src/bundle/manifest.ts', find: "pattern: '^/(?!\\\\.{1,2}(/|$))[A-Za-z0-9._-]+(/(?!\\\\.{1,2}(/|$))[A-Za-z0-9._-]+)*$'", replace: "pattern: '^/[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$'", layers: ['unit'] },
  // ---- F2 (decision 0141): coordinates, buttons, keys, focus loss, the idle driver, the desktop layout, the cursor, the engine's window list ----
  { id: 'screen-y-flipped', file: 'src/input/tracker.ts', find: 'const fx = (clientX - rect.left) / rect.width, fy = (clientY - rect.top) / rect.height;', replace: 'const fx = (clientX - rect.left) / rect.width, fy = 1 - (clientY - rect.top) / rect.height;', layers: ['unit'] },
  { id: 'mouse-buttons-right-and-middle-swapped', file: 'src/input/tracker.ts', find: 'const MASK_BUTTONS: readonly [number, number][] = [[1, BTN.L], [2, BTN.R], [4, BTN.M]];', replace: 'const MASK_BUTTONS: readonly [number, number][] = [[1, BTN.L], [2, BTN.M], [4, BTN.R]];', layers: ['unit'] },
  { id: 'shift-key-not-in-the-table', file: 'src/input/keymap.ts', find: "add('ShiftLeft', 0xa0, 0x2a); ", replace: '', layers: ['unit'] },
  { id: 'blur-leaves-keys-down', file: 'src/input/tracker.ts', find: "for (const [, k] of [...this.held.entries()].reverse()) this.emit({ k: 'key', kind: IN.KUP, vk: k.vk, scan: k.scan, ext: k.ext ? 1 : 0 });", replace: '', layers: ['unit'] },
  { id: 'cancel-leaves-buttons-down', file: 'src/input/tracker.ts', find: 'pointerCancel(): void { this.pointer({ x: Math.max(0, this.lastX), y: Math.max(0, this.lastY), buttons: 0 }); }', replace: 'pointerCancel(): void { /* nothing */ }', layers: ['unit'] },
  { id: 'wheel-direction-flipped', file: 'src/input/tracker.ts', find: 'this.rest += -notches * 120;', replace: 'this.rest += notches * 120;', layers: ['unit'] },
  { id: 'extended-flag-lost-on-arrows', file: 'src/input/keymap.ts', find: "add('ArrowLeft', 0x25, 0x4b, true); add('ArrowRight', 0x27, 0x4d, true);", replace: "add('ArrowLeft', 0x25, 0x4b, false); add('ArrowRight', 0x27, 0x4d, true);", layers: ['unit'] },
  { id: 'tab-is-forwarded-and-suppressed', file: 'src/input/keymap.ts', find: "if (e.code === 'Tab' && !opts.sendTab) return { forward: false, prevent: false, why: 'tab-leaves-the-desktop' };", replace: '', layers: ['unit'] },
  { id: 'f5-is-left-to-the-browser', file: 'src/input/keymap.ts', find: "  if (e.code === 'KeyW' && (e.ctrlKey || e.metaKey))", replace: "  if (e.code === 'F5') return { forward: false, prevent: false, why: 'reload' };\n  if (e.code === 'KeyW' && (e.ctrlKey || e.metaKey))", layers: ['unit'] },
  { id: 'consecutive-moves-not-merged', file: 'src/engine/idle-driver.ts', find: "if (last && ev.k === 'mouse' && ev.kind === IN.MOVE && last.ev.k === 'mouse' && last.ev.kind === IN.MOVE) {", replace: 'if (false as boolean) {', layers: ['unit'] },
  { id: 'clock-may-pass-the-deadline', file: 'src/engine/idle-driver.ts', find: 'if (i.next > 0 && t > i.next) t = i.next;', replace: '', layers: ['unit'] },
  { id: 'input-before-the-first-idle-is-queued', file: 'src/engine/idle-driver.ts', find: 'if (!this.started) {\n      this.stats.dropped++;', replace: 'if (false as boolean) {\n      this.stats.dropped++;', layers: ['unit'] },
  { id: 'deadline-timer-too-early', file: 'src/engine/idle-driver.ts', find: 'const waitMs = (i.next - t) / TICKS_PER_MS / this.rate;', replace: 'const waitMs = (i.next - t) / TICKS_PER_MS;', layers: ['unit'] },
  { id: 'catchup-delivers-deadlines-before-the-wall-clock-reaches-them', file: 'src/engine/idle-driver.ts', find: 'if (i.next <= t) {', replace: 'if (true as boolean) {', layers: ['unit'] },
  { id: 'catchup-lag-never-cut', file: 'src/engine/idle-driver.ts', find: 'if (raw > now + this.maxLagTicks) {', replace: 'if (false as boolean) {', layers: ['unit'] },
  { id: 'catchup-turn-budget-ignored', file: 'src/engine/idle-driver.ts', find: 'if (w - this.turnStart < this.turnBudgetMs) {', replace: 'if (true as boolean) {', layers: ['unit'] },
  { id: 'catchup-lead-forgiven-not-kept', file: 'src/engine/idle-driver.ts', find: 'else if (raw < now - this.maxLeadTicks) { t = now - this.maxLeadTicks;', replace: 'else if (raw < now) { t = now;', layers: ['unit'] },
  { id: 'catchup-anchor-not-moved-when-cut', file: 'src/engine/idle-driver.ts', find: 'if (t !== raw) this.anchor = { wall: w, ticks: t };', replace: '', layers: ['unit'] },
  { id: 'catchup-wait-from-idle-start-not-from-target', file: 'src/engine/idle-driver.ts', find: 'const waitMs = (i.next - t) / TICKS_PER_MS / this.rate;', replace: 'const waitMs = (i.next - i.now) / TICKS_PER_MS / this.rate;', layers: ['unit'] },
  { id: 'catchup-stale-bounce-resumes', file: 'src/engine/idle-driver.ts', find: '      const go = (): void => {\n        if (!this.idle || this.idle.n !== n) return;', replace: '      const go = (): void => {\n        if (!this.idle) return;', layers: ['unit'] },
  { id: 'report-idles-not-throttled', file: 'src/engine/report-throttle.ts', find: 'if (n !== 1 && at - this.lastIdleAt < this.minGapMs) return false;', replace: '', layers: ['unit'] },
  { id: 'report-resume-after-resume-not-suppressed', file: 'src/engine/report-throttle.ts', find: "if (why === 'deadline' && !this.pageSaysIdle) return false;", replace: '', layers: ['unit'] },
  // ---- decision 0182 (p2a): ?hz=, the refresh rate of the virtual display, and the table stamp ----
  { id: 'hz-only-one-variable-passed', file: 'src/engine/hz.ts', find: 'return { FLE_DWM_HZ: String(hz), FLE_DISPLAYSETTINGS_FREQUENCY: String(hz) };', replace: 'return { FLE_DWM_HZ: String(hz) };', layers: ['unit'] },
  { id: 'hz-round-by-difference-not-ratio', file: 'src/engine/hz.ts', find: 'const d = Math.abs(Math.log(measured / s));', replace: 'const d = Math.abs(measured - s);', layers: ['unit'] },
  { id: 'hz-mean-not-median', file: 'src/engine/hz.ts', find: 'const median = sorted.length % 2 ? (sorted[mid] as number) : ((sorted[mid - 1] as number) + (sorted[mid] as number)) / 2;', replace: 'const median = used.reduce((a, b) => a + b, 0) / used.length;', layers: ['unit'] },
  // ---- decision 0187: the exact omission of an unchanged window rectangle (?dedupe=1) ----
  // ---- decision 0201: a lost / restored 2D canvas context asks for every window to be sent whole again (ctx-resync.ts, Presenter.forgetSeen, SurfaceSync.resync, client.ts watchLoss, worker.ts resync) ----
  { id: 'ctx-resync-contextlost-not-cancelled', file: 'src/engine/ctx-resync.ts', find: "target.addEventListener('contextlost', (ev) => { ev.preventDefault(); hooks.lost(); });", replace: "target.addEventListener('contextlost', () => { hooks.lost(); });", layers: ['unit'] },
  { id: 'ctx-resync-contextrestored-ignored', file: 'src/engine/ctx-resync.ts', find: "target.addEventListener('contextrestored', () => hooks.restored());", replace: '', layers: ['unit'] },
  { id: 'appview-seq-ignored-by-the-reader', file: 'src/engine/appview.ts', find: 'if (sinceSeq !== undefined && dv.getUint32(16, true) === sinceSeq) return null;', replace: '', layers: ['unit'] },
  { id: 'latency-frames-seen-before-input-done-not-credited', file: 'src/ui/latency.ts', find: '      this.creditSeen(r);\n', replace: '', layers: ['unit'] },
  { id: 'latency-frame-judged-by-receipt-time-not-post-time', file: 'src/ui/latency.ts', find: 'if (postedAt < r.tInject) continue;', replace: 'if (at < r.tInject) continue;', layers: ['unit'] },
  { id: 'latency-credit-judged-by-receipt-time-not-post-time', file: 'src/ui/latency.ts', find: 'if (f.postedAt < tInject || f.postedAt > end) continue;', replace: 'if (f.at < tInject || f.at > end) continue;', layers: ['unit'] },
  { id: 'latency-frame-before-the-injection-counted', file: 'src/ui/latency.ts', find: 'if (f.postedAt < tInject || f.postedAt > end) continue;', replace: 'if (f.postedAt > end) continue;', layers: ['unit'] },
  // ---- decision 0205: the answer is the frame of the processing window; an input without one has no value and is counted ----
  { id: 'latency-credit-ignores-the-end-of-the-window', file: 'src/ui/latency.ts', find: 'if (f.postedAt < tInject || f.postedAt > end) continue;', replace: 'if (f.postedAt < tInject) continue;', layers: ['unit'] },
  { id: 'latency-late-frame-credited-by-frame()', file: 'src/ui/latency.ts', find: 'if (postedAt <= r.tIdle + IN_WINDOW_MARGIN_MS && (r.outcome === null || r.outcome === \'window\')) {', replace: 'if (true as boolean && (r.outcome === null || r.outcome === \'window\')) {', layers: ['unit'] },
  { id: 'latency-window-margin-zero', file: 'src/ui/latency.ts', find: 'export const IN_WINDOW_MARGIN_MS = 1;', replace: 'export const IN_WINDOW_MARGIN_MS = 0;', layers: ['unit'] },
  { id: 'latency-window-margin-50-ms', file: 'src/ui/latency.ts', find: 'export const IN_WINDOW_MARGIN_MS = 1;', replace: 'export const IN_WINDOW_MARGIN_MS = 50;', layers: ['unit'] },
  { id: 'latency-moves-never-held', file: 'src/ui/latency.ts', find: 'else if (ev.kind === IN.MOVE) held = this.buttons.size > 0;', replace: 'else if (ev.kind === IN.MOVE) held = false;', layers: ['unit'] },
  { id: 'latency-press-not-held', file: 'src/ui/latency.ts', find: 'if (ev.kind === IN.BDOWN) { this.buttons.add(ev.data); held = true; }', replace: 'if (ev.kind === IN.BDOWN) { this.buttons.add(ev.data); }', layers: ['unit'] },
  { id: 'latency-release-leaves-the-button-down', file: 'src/ui/latency.ts', find: 'else if (ev.kind === IN.BUP) this.buttons.delete(ev.data);', replace: 'else if (ev.kind === IN.BUP) { /* forgotten */ }', layers: ['unit'] },
  { id: 'latency-stream-wait-unbounded', file: 'src/ui/latency.ts', find: 'if (r.held && f.postedAt <= r.tInject + STREAM_MS) {', replace: 'if (r.held) {', layers: ['unit'] },
  { id: 'latency-late-frame-counted-as-the-answer', file: 'src/ui/latency.ts', find: "} else if (!r.held && f.postedAt <= end + LATE_MS) { o = 'late'; r.lateFrame = f; }", replace: "} else if (!r.held && f.postedAt <= end + LATE_MS) { r.frames[f.lid] = f.at; r.answers[f.lid] = f; o = 'stream'; }", layers: ['unit'] },
  { id: 'latency-unanswered-never-counted', file: 'src/ui/latency.ts', find: 'else this.noAnswerCount++;', replace: 'else void 0;', layers: ['unit'] },
  { id: 'latency-waiting-input-never-given-up', file: 'src/ui/latency.ts', find: 'if (now > until) this.decide(r, \'none\');', replace: '', layers: ['unit'] },
  { id: 'latency-trimmed-waiting-input-not-counted', file: 'src/ui/latency.ts', find: 'if (old) { this.bySeq.delete(old.seq); this.decide(old, \'none\'); }', replace: 'if (old) { this.bySeq.delete(old.seq); }', layers: ['unit'] },
  { id: 'latency-held-wait-ends-with-the-late-wait', file: 'src/ui/latency.ts', find: 'const until = r.held ? r.tInject + STREAM_MS : r.tIdle + IN_WINDOW_MARGIN_MS + LATE_MS;', replace: 'const until = r.tIdle + IN_WINDOW_MARGIN_MS + LATE_MS;', layers: ['unit'] },
  { id: 'latency-totals-hide-the-unanswered', file: 'src/ui/latency.ts', find: 'answered: o.answered, late: o.late, noAnswer: o.noAnswer };', replace: 'answered: o.answered, late: o.late, noAnswer: 0 };', layers: ['unit'] },
  { id: 'panel-p2-row-hides-the-unanswered', file: 'src/perf/panel.ts', find: "const answersText = i.processed > 0 ? ", replace: "const answersText = false as boolean ? ", layers: ['unit'] },
  { id: 'beacon-counts-may-exceed-processed', file: 'shared/perf.ts', find: "v.answered + v.late + v.noAnswer > v.processed) out.push(", replace: "v.answered + v.late + v.noAnswer > v.processed + 1e9) out.push(", layers: ['unit'] },
  { id: 'beacon-counts-not-required', file: 'shared/perf.ts', find: "for (const k of ['sent', 'processed', 'dropped', 'pending', 'answered', 'late', 'noAnswer']) num(", replace: "for (const k of ['sent', 'processed', 'dropped', 'pending']) num(", layers: ['unit'] },
  { id: 'summary-hides-the-unanswered', file: 'shared/perf-summary.mjs', find: "const none = c ? `; of ${c.processed}", replace: "const none = false ? `; of ${c.processed}", layers: ['unit'] },
  { id: 'latency-late-frame-not-kept', file: 'src/ui/latency.ts', find: "{ o = 'late'; r.lateFrame = f; }", replace: "{ o = 'late'; }", layers: ['unit'] },
  { id: 'stats-late-values-in-the-answered-p95', file: 'src/perf/stats.ts', find: "if (l !== null && framesAreChanges) { lateV.push(l); both.push(l); }", replace: "if (l !== null && framesAreChanges) { lateV.push(l); both.push(l); fresh.push(l); staged.push(null); }", layers: ['unit'] },
  { id: 'stats-late-visible-left-out-of-the-verdict-set', file: 'src/perf/stats.ts', find: "if (l !== null && framesAreChanges) { lateV.push(l); both.push(l); }", replace: "if (l !== null && framesAreChanges) { lateV.push(l); }", layers: ['unit'] },
  { id: 'stats-late-statistics-always-there', file: 'src/perf/stats.ts', find: "lateVisible: framesAreChanges ? latencyStat(lateV, last) : null, eventToRafWithLate: framesAreChanges ? latencyStat(both, last) : null,", replace: "lateVisible: latencyStat(lateV, last), eventToRafWithLate: latencyStat(both, last),", layers: ['unit'] },
  { id: 'panel-verdict-on-the-answered-inputs-only', file: 'src/perf/panel.ts', find: "const j = withLate ? w : e;", replace: "const j = e;", layers: ['unit'] },
  { id: 'summary-verdict-on-the-answered-inputs-only', file: 'shared/perf-summary.mjs', find: "const eventToRaf = withLate ?? answeredOnly;", replace: "const eventToRaf = answeredOnly;", layers: ['unit'] },
  { id: 'beacon-late-statistics-may-disagree', file: 'shared/perf.ts', find: "if ((v.lateVisible === null) !== (v.eventToRafWithLate === null)) out.push(", replace: "if (false as boolean) out.push(", layers: ['unit'] },
  { id: 'beacon-late-statistic-may-be-absent', file: 'shared/perf.ts', find: "if (nullable && s === null) continue;", replace: "if (nullable && (s === null || s === undefined)) continue;", layers: ['unit'] },
  { id: 'latency-frames-are-changes-in-check-mode', file: 'src/ui/latency.ts', find: "export const framesAreChangesFor = (dedupe: boolean | 'check'): boolean => dedupe === true;", replace: "export const framesAreChangesFor = (dedupe: boolean | 'check'): boolean => dedupe !== false;", layers: ['unit'] },
  { id: 'beacon-wasm-fraction-wrong-divisor', file: 'src/perf/beacon.ts', find: 'wasmFraction: i.wasmMemoryBytes ? i.wasmMemoryBytes / WASM_MEMORY_LIMIT_BYTES : null', replace: 'wasmFraction: i.wasmMemoryBytes ? i.wasmMemoryBytes / (WASM_MEMORY_LIMIT_BYTES * 2) : null', layers: ['unit'] },
  { id: 'panel-wasm-memory-row-given-a-verdict', file: 'src/perf/panel.ts', find: "target: `< ${mb(WASM_MEMORY_LIMIT_BYTES)}`, status: 'na',", replace: "target: `< ${mb(WASM_MEMORY_LIMIT_BYTES)}`, status: 'met',", layers: ['unit'] },
  { id: 'wasm-memory-limit-constant-halved', file: 'src/perf/stats.ts', find: 'export const WASM_MEMORY_LIMIT_BYTES = 2048 * 1024 * 1024;', replace: 'export const WASM_MEMORY_LIMIT_BYTES = 1024 * 1024 * 1024;', layers: ['unit'] },
  { id: 'sizewe-cursor-is-vertical', file: 'src/ui/cursor.ts', find: "32644: 'ew-resize',", replace: "32644: 'ns-resize',", layers: ['unit'] },
  { id: 'appview-visible-and-enabled-bits-swapped', file: 'src/engine/appview.ts', find: 'visible: (flags & W_VISIBLE) !== 0, enabled: (flags & W_ENABLED) !== 0,', replace: 'visible: (flags & W_ENABLED) !== 0, enabled: (flags & W_VISIBLE) !== 0,', layers: ['unit'] },
  { id: 'appview-window-array-starts-at-the-wrong-offset', file: 'src/engine/appview.ts', find: 'export const AV_HEADER_BYTES = 68;', replace: 'export const AV_HEADER_BYTES = 64;', layers: ['unit'] },
  { id: 'dropped-events-counted-as-pending', file: 'src/ui/latency.ts', find: 'get pending(): number { return this.sentCount - this.doneCount - this.droppedCount; }', replace: 'get pending(): number { return this.sentCount - this.doneCount; }', layers: ['unit'] },
  { id: 'engine-called-idle-while-computing', file: 'src/ui/engine-status.ts', find: 'c.lastIdleAt >= c.lastResumeAt', replace: 'c.lastIdleAt <= c.lastResumeAt', layers: ['unit'] },
  { id: 'input-accepted-while-starting', file: 'src/ui/engine-status.ts', find: "text: `Engine: starting (${secs(now - c.startedAt)}) - input is ignored until its first idle`, inputAccepted: false }", replace: "text: `Engine: starting (${secs(now - c.startedAt)}) - input is ignored until its first idle`, inputAccepted: true }", layers: ['unit'] },
  { id: 'worker-drops-all-input', file: 'src/engine/worker.ts', find: "ring.drain((r) => driver?.submit(r.ev, r.seq, r.tPage))", replace: "ring.drain(() => undefined)", layers: ['e2e'], e2eOnly: 'engine-desk' },
  // ---- decision 0145: layered windows (surface table version 2, the premultiplied conversion, the colour key, the constant alpha, the staging canvas, not-shown layers) ----
  // ---- decision 0150: the main window is found by what it is (main-window.ts), not by a window number ----
  // ---- decision 0152: the default manifest path /assets/manifest.json is for the real engine only (shared/manifest-guard.ts, tools/assets-middleware.ts, worker/index.ts, src/bundle/loader.ts) ----
  { id: "guard-unknown-kind-passes", file: "shared/manifest-guard.ts", find: "if (!REAL_ENGINE_KINDS.includes(kind)) return { ok: false, status: 409, body: notRealEngineRefusal(kind) };", replace: "", layers: ['unit', 'worker'] },
  { id: "guard-missing-kind-passes", file: "shared/manifest-guard.ts", find: "if (typeof kind !== 'string') return { ok: false, status: 409, body: notRealEngineRefusal(null) };", replace: "if (typeof kind !== 'string') return { ok: true, kind: '' };", layers: ['unit', 'worker'] },
  { id: "guard-broken-json-passes", file: "shared/manifest-guard.ts", find: "return { ok: false, status: 500, body: brokenManifestBody(`JSON として読めません${pos ? `: 位置 ${pos}` : ''}`) }; }", replace: "return { ok: true, kind: '' }; }", layers: ['unit', 'worker'] },
  { id: "guard-case-not-folded", file: "shared/manifest-guard.ts", find: "return out.join('/').toLowerCase();\n}", replace: "return out.join('/');\n}", layers: ['unit', 'worker'] },
  { id: "guard-dot-segments-not-resolved", file: "shared/manifest-guard.ts", find: "if (seg === '' || seg === '.') continue;", replace: "if (seg === '') continue;", layers: ['unit'] },
  { id: "guard-parent-segments-not-resolved", file: "shared/manifest-guard.ts", find: "if (seg === '..') { out.pop(); continue; }", replace: "", layers: ['unit'] },
  { id: "guard-empty-segments-not-resolved", file: "shared/manifest-guard.ts", find: "if (seg === '' || seg === '.') continue;", replace: "if (seg === '.') continue;", layers: ['unit'] },
  { id: "guard-unicode-not-normalized", file: "shared/manifest-guard.ts", find: "const s = decodedRel.normalize('NFKC').replace(/\\\\/g, '/');", replace: "const s = decodedRel.replace(/\\\\/g, '/');", layers: ['unit'] },
  { id: "guard-trailing-dots-kept", file: "shared/manifest-guard.ts", find: "out[last] = (out[last] as string).replace(/[. ]+$/, '');", replace: "", layers: ['unit'] },
  { id: "guard-allow-list-also-takes-the-standin", file: "shared/manifest-guard.ts", find: "export const REAL_ENGINE_KINDS: readonly string[] = ['emscripten-fruity-v1'];", replace: "export const REAL_ENGINE_KINDS: readonly string[] = ['emscripten-fruity-v1', 'surface-table-v1'];", layers: ['unit', 'worker'] },
  { id: "middleware-path-not-percent-decoded-for-the-guard", file: "tools/assets-middleware.ts", find: "const guarded = decoded !== null && isDefaultManifestKey(decoded);", replace: "const guarded = isDefaultManifestKey(rel);", layers: ['unit', 'e2e'], e2eOnly: 'default-manifest' },
  { id: "middleware-head-not-judged", file: "tools/assets-middleware.ts", find: "if (guarded) {\n      const v = inspectDefaultManifest(", replace: "if (guarded && req.method === 'GET') {\n      const v = inspectDefaultManifest(", layers: ['unit'] },
  { id: "middleware-variant-served-without-being-judged", file: "tools/assets-middleware.ts", find: "const v = inspectDefaultManifest(chosen.file, coding, st.size);", replace: "const v = inspectDefaultManifest(chosen.file, null, st.size);", layers: ['unit'] },
  { id: "middleware-size-limit-removed", file: "tools/assets-middleware.ts", find: "if (size > MAX_MANIFEST_BYTES) return fail(`${size} バイトは大きすぎて検査できません`);", replace: "", layers: ['unit'] },
  { id: "middleware-missing-manifest-gets-the-bare-404", file: "tools/assets-middleware.ts", find: "send(res, 404, guarded ? notDeployedBody() : `not found: ${url.pathname}`);", replace: "send(res, 404, `not found: ${url.pathname}`);", layers: ['unit', 'e2e'], e2eOnly: 'default-manifest' },
  { id: "middleware-no-assets-dir-gets-the-bare-404", file: "tools/assets-middleware.ts", find: "send(res, 404, guarded ? notDeployedBody(NO_DIR + '\\n') : NO_DIR);", replace: "send(res, 404, NO_DIR);", layers: ['unit'] },
  { id: "middleware-query-string-joins-the-path", file: "tools/assets-middleware.ts", find: "const rel = url.pathname.slice(prefix.length);", replace: "const rel = (url.pathname + url.search).slice(prefix.length);", layers: ['unit', 'e2e'], e2eOnly: 'default-manifest' },
  { id: "middleware-guard-only-without-a-range-header", file: "tools/assets-middleware.ts", find: "if (guarded) {\n      const v = inspectDefaultManifest(", replace: "if (guarded && !req.headers.range) {\n      const v = inspectDefaultManifest(", layers: ['unit'] },
  { id: "worker-range-request-not-judged", file: "worker/index.ts", find: "if (guarded) {\n    const j = await judgeDefaultManifest(", replace: "if (guarded && !request.headers.get('Range')) {\n    const j = await judgeDefaultManifest(", layers: ['worker'] },
  { id: "worker-head-not-judged", file: "worker/index.ts", find: "if (guarded) {\n    const j = await judgeDefaultManifest(", replace: "if (guarded && request.method === 'GET') {\n    const j = await judgeDefaultManifest(", layers: ['worker'] },
  { id: "worker-stored-gzip-not-decoded", file: "worker/index.ts", find: "if (enc !== '' && enc !== 'identity') {", replace: "if (false as boolean) {", layers: ['worker'] },
  { id: "worker-missing-manifest-gets-the-bare-404", file: "worker/index.ts", find: "if (!head) return text(404, guarded ? notDeployedBody() : `not found: ${pathname}`);", replace: "if (!head) return text(404, `not found: ${pathname}`);", layers: ['worker'] },
  { id: "worker-brotli-manifest-passes-uninspected", file: "worker/index.ts", find: "if (enc !== 'gzip' && enc !== 'deflate') return broken(", replace: "if (false as boolean) return broken(", layers: ['worker'] },
  { id: "loader-drops-the-servers-explanation", file: "src/bundle/loader.ts", find: "const why = explain ? await explanationOf(res) : '';", replace: "const why = '';", layers: ['unit', 'e2e'], e2eOnly: 'default-manifest' },
  { id: "loader-copies-html-error-pages", file: "src/bundle/loader.ts", find: "if (!/^text\\/plain/i.test(res.headers.get('content-type') ?? '')) return '';", replace: "", layers: ['unit'] },
  { id: "loader-explanation-not-cut", file: "src/bundle/loader.ts", find: ".trim().slice(0, 2000); } catch", replace: ".trim(); } catch", layers: ['unit'] },
  // ---- decision 0152, part B: the measurement beacon (shared/perf.ts, tools/perf-middleware.ts, worker/index.ts, src/perf/*, src/main.ts) ----
  { id: "perf-window-is-5-s", file: "src/perf/stats.ts", find: "export const RECENT_WINDOW_MS = 10_000;", replace: "export const RECENT_WINDOW_MS = 5_000;", layers: ['unit'] },
  { id: "perf-median-is-p75", file: "src/perf/stats.ts", find: "intervalMedianMs: percentile(gaps, 50),", replace: "intervalMedianMs: percentile(gaps, 75),", layers: ['unit'] },
  { id: "perf-p95-is-max", file: "src/perf/stats.ts", find: "intervalP95Ms: percentile(gaps, 95),", replace: "intervalP95Ms: percentile(gaps, 100),", layers: ['unit'] },
  { id: "perf-rate-divides-by-the-full-window", file: "src/perf/stats.ts", find: "perSecond: span === null || span <= 0 ? null : inWin.length / (span / 1000),", replace: "perSecond: span === null || span <= 0 ? null : inWin.length / (windowMs / 1000),", layers: ['unit'] },
  { id: "perf-window-includes-its-old-edge", file: "src/perf/stats.ts", find: "const inWin = times.filter((t) => t > now - windowMs && t <= now);", replace: "const inWin = times.filter((t) => t >= now - windowMs && t <= now);", layers: ['unit'] },
  { id: "perf-input-keeps-all-events", file: "src/perf/stats.ts", find: "const lastMs = values.slice(-last).map(round3);", replace: "const lastMs = values.map(round3);", layers: ['unit'] },
  { id: "perf-input-counts-dropped-events", file: "src/perf/stats.ts", find: "if (r.dropped !== null) continue;\n    const v = toAnyFrame(r);", replace: "const v = toAnyFrame(r);", layers: ['unit'] },
  // decision 0202: input -> update from the event's timeStamp to the next animation frame
  { id: "latency-start-is-the-listener-not-the-event-stamp", file: "src/ui/latency.ts", find: "f === null || r.tEvent === null ? null : (f.raf as number) - r.tEvent;", replace: "f === null || r.tEvent === null ? null : (f.raf as number) - r.tPage;", layers: ['unit'] },
  { id: "latency-end-is-the-draw-not-the-animation-frame", file: "src/ui/latency.ts", find: "f === null || r.tEvent === null ? null : (f.raf as number) - r.tEvent;", replace: "f === null || r.tEvent === null ? null : f.at - r.tEvent;", layers: ['unit'] },
  { id: "latency-animation-frame-after-hidden-tab-accepted", file: "src/ui/latency.ts", find: "f.raf = t >= f.at && t - f.at <= RAF_MAX_WAIT_MS ? t : -1;", replace: "f.raf = t;", layers: ['unit'] },
  { id: "latency-animation-frame-asked-while-idle", file: "src/ui/latency.ts", find: "if (credited || this.mayBeAnswered(at)) this.armRaf();", replace: "this.armRaf();", layers: ['unit'] },
  { id: "latency-animation-frame-not-asked-for-a-waiting-event", file: "src/ui/latency.ts", find: "if (credited || this.mayBeAnswered(at)) this.armRaf();", replace: "if (credited) this.armRaf();", layers: ['unit'] },
  { id: "latency-stamp-after-the-listener-accepted", file: "src/ui/latency.ts", find: "meta.tEvent <= meta.tListener && tPage", replace: "tPage", layers: ['unit'] },
  { id: "latency-stamp-of-any-age-accepted", file: "src/ui/latency.ts", find: " && tPage - meta.tEvent <= EVENT_STAMP_MAX_AGE_MS;", replace: ";", layers: ['unit'] },
  { id: "latency-later-animation-frame-wins", file: "src/ui/latency.ts", find: "f.raf < (best.raf as number)", replace: "f.raf > (best.raf as number)", layers: ['unit'] },
  { id: "latency-stage-s0-ends-at-the-page-record", file: "src/ui/latency.ts", find: "return { s0: r.tListener - r.tEvent,", replace: "return { s0: r.tPage - r.tEvent,", layers: ['unit'] },
  { id: "latency-event-stamp-is-the-listener-time", file: "src/input/pointer.ts", find: "tEvent: performance.timeOrigin + e.timeStamp,", replace: "tEvent: performance.timeOrigin + performance.now(),", layers: ['unit'] },
  { id: "latency-event-stamp-outlives-its-event", file: "src/input/pointer.ts", find: "finally { meta = undefined; }", replace: "finally { /* kept */ }", layers: ['unit'] },
  { id: "perf-summary-v3-verdict-from-the-old-number", file: "shared/perf-summary.mjs", find: "inputP95Ms: eventToRaf?.p95Ms ?? null,", replace: "inputP95Ms: listenerToPut?.p95Ms ?? null,", layers: ['unit'] },
  { id: "perf-summary-v2-old-number-judged-as-p2", file: "shared/perf-summary.mjs", find: "inputP95Ms: eventToRaf?.p95Ms ?? null,", replace: "inputP95Ms: eventToRaf?.p95Ms ?? listenerToPut?.p95Ms ?? null,", layers: ['unit'] },
  { id: "perf-beacon-version-3-not-accepted", file: "shared/perf.ts", find: "num('v', b.v, 2, 3, true);", replace: "num('v', b.v, 2, 2, true);", layers: ['unit'] },
  { id: "perf-beacon-input-block-not-checked", file: "shared/perf.ts", find: "if (b.v === 3) inputBlock(b.input, num, out);", replace: "", layers: ['unit'] },
  { id: "perf-beacon-cross-clock-stage-must-be-positive", file: "shared/perf.ts", find: "cross ? -100 : 0, 3.6e6", replace: "0, 3.6e6", layers: ['unit'] },
  // decision 0202: the screen meter's rAF in bursts
  { id: "display-burst-never-ends", file: "src/perf/display.ts", find: "    this.bursting = false;\n    this.frameLoop = false;\n    this.meter.endBurst(this.host.now());", replace: "", layers: ['unit'] },
  { id: "display-loop-goes-on-after-the-burst", file: "src/perf/display.ts", find: "      if (!this.bursting) return;   // the burst ended while this callback was pending: it neither counts nor asks for another\n", replace: "", layers: ['unit'] },
  { id: "display-no-periodic-burst", file: "src/perf/display.ts", find: "this.periodTimer = this.host.setTimer(() => { this.beginBurst(); this.schedulePeriod(); }, this.periodMs);", replace: "this.periodTimer = this.host.setTimer(() => { this.schedulePeriod(); }, this.periodMs);", layers: ['unit'] },
  { id: "display-no-burst-at-the-first-idle", file: "src/perf/display.ts", find: "if (this.mode === 'burst' && !this.idleBurstDone) {", replace: "if (false as boolean) {", layers: ['unit'] },
  { id: "display-no-burst-at-the-start", file: "src/perf/display.ts", find: "    } else {\n      this.beginBurst();\n      this.schedulePeriod();\n    }", replace: "    } else {\n      this.schedulePeriod();\n    }", layers: ['unit'] },
  { id: "display-default-is-the-continuous-loop", file: "src/perf/display.ts", find: "this.mode = opts.mode ?? 'burst';", replace: "this.mode = opts.mode ?? 'continuous';", layers: ['unit'] },
  { id: "display-rate-divides-by-the-page-seconds", file: "src/perf/display.ts", find: "const totalVis = this.observedUntil(now);", replace: "const totalVis = this.visibleUntil(now);", layers: ['unit'] },
  { id: "display-recent-is-the-last-10-s-in-burst-mode", file: "src/perf/display.ts", find: "const from = this.burstMode ? (this.lastBurstStart ?? now) - 1 : Math.max(started, now - RECENT_MS);", replace: "const from = Math.max(started, now - RECENT_MS);", layers: ['unit'] },
  { id: "display-burst-block-not-checked", file: "shared/perf.ts", find: "      if (d.mode === 'burst') {\n        if (!isObj(d.burst))", replace: "      if (false as boolean) {\n        if (!isObj(d.burst))", layers: ['unit'] },
  { id: "perf-target-rate-is-60", file: "src/perf/stats.ts", find: "updatesPerSecond: 120,", replace: "updatesPerSecond: 60,", layers: ['unit'] },
  { id: "perf-origin-not-checked", file: "shared/perf.ts", find: "if (f.origin !== undefined && f.secFetchSite === undefined) {", replace: "if (false as boolean) {", layers: ['unit', 'worker'] },
  { id: "perf-origin-checked-even-when-sec-fetch-site-says-same-origin", file: "shared/perf.ts", find: "if (f.origin !== undefined && f.secFetchSite === undefined) {", replace: "if (f.origin !== undefined) {", layers: ['unit'] },
  { id: "perf-sec-fetch-site-not-checked", file: "shared/perf.ts", find: "if (f.secFetchSite !== undefined && f.secFetchSite !== 'same-origin' && f.secFetchSite !== 'none') return", replace: "if (false as boolean) return", layers: ['unit', 'worker'] },
  { id: "perf-limit-is-32-kb", file: "shared/perf.ts", find: "export const PERF_MAX_BODY = 16 * 1024;", replace: "export const PERF_MAX_BODY = 32 * 1024;", layers: ['unit', 'worker'] },
  { id: "perf-content-type-not-checked", file: "shared/perf.ts", find: "if (type !== 'application/json') return { ok: false, status: 415,", replace: "if (false as boolean) return { ok: false, status: 415,", layers: ['unit', 'worker'] },
  { id: "perf-array-body-accepted", file: "shared/perf.ts", find: "if (typeof json !== 'object' || json === null || Array.isArray(json)) return", replace: "if (typeof json !== 'object' || json === null) return", layers: ['unit', 'worker'] },
  { id: "perf-tailnet-range-too-wide", file: "shared/perf.ts", find: "if (a0 === 100 && a1 >= 64 && a1 <= 127) return 'tailnet';", replace: "if (a0 === 100 && a1 >= 64 && a1 <= 128) return 'tailnet';", layers: ['unit'] },
  { id: "perf-log-stores-the-address", file: "tools/perf-middleware.ts", find: "classifyPeer(req.socket.remoteAddress, one(req.headers['x-forwarded-for']))))", replace: "(req.socket.remoteAddress ?? 'other') as 'other'))", layers: ['unit'] },
  { id: "perf-log-overwritten-not-appended", file: "tools/perf-middleware.ts", find: "fs.appendFileSync(logPath,", replace: "fs.writeFileSync(logPath,", layers: ['unit'] },
  { id: "perf-without-log-path-pretends-to-store", file: "tools/perf-middleware.ts", find: "reply(res, 503, 'the beacon is not stored: set FL_PERF_LOG (or FL_ASSETS_DIR) for this server\\n', false); return; }", replace: "reply(res, 204, '', true); return; }", layers: ['unit'] },
  { id: "perf-default-log-in-the-assets-dir-itself", file: "tools/perf-middleware.ts", find: "return path.join(path.dirname(path.resolve(env.FL_ASSETS_DIR)), 'perf-log.jsonl');", replace: "return path.join(path.resolve(env.FL_ASSETS_DIR), 'perf-log.jsonl');", layers: ['unit'] },
  { id: "perf-worker-claims-to-store", file: "worker/index.ts", find: "const noStore = { 'Cache-Control': 'no-store', [PERF_STORED_HEADER]: 'no' };", replace: "const noStore = { 'Cache-Control': 'no-store', [PERF_STORED_HEADER]: 'yes' };", layers: ['worker'] },
  { id: "perf-worker-body-limit-not-enforced", file: "worker/index.ts", find: "if (size > PERF_MAX_BODY) { await reader.cancel();", replace: "if (false as boolean) { await reader.cancel();", layers: ['worker'] },
  { id: "perf-beacon-every-60-s", file: "src/perf/beacon.ts", find: "export const BEACON_PERIOD_MS = 30_000;", replace: "export const BEACON_PERIOD_MS = 60_000;", layers: ['unit'] },
  { id: "perf-beacon-21-periodic", file: "src/perf/beacon.ts", find: "export const BEACON_MAX_PERIODIC = 20;", replace: "export const BEACON_MAX_PERIODIC = 21;", layers: ['unit'] },
  { id: "perf-beacon-end-sent-twice", file: "src/perf/beacon.ts", find: "if (this.terminalSent) return;\n    this.terminalSent = true;", replace: "this.terminalSent = true;", layers: ['unit'] },
  { id: "perf-beacon-periodic-continues-after-the-end", file: "src/perf/beacon.ts", find: "  end(reason: 'exit' | 'error'): void {\n    this.stop();", replace: "  end(reason: 'exit' | 'error'): void {", layers: ['unit'] },
  { id: "perf-beacon-first-idle-twice", file: "src/perf/beacon.ts", find: "if (this.firstSent || this.terminalSent) return;", replace: "if (this.terminalSent) return;", layers: ['unit'] },
  { id: "desk_gl-upload-uses-the-surface-stride", file: "src/engine/desk-gl.ts", find: "gl.pixelStorei(gl.UNPACK_ROW_LENGTH, w.stride >> 2);", replace: "gl.pixelStorei(gl.UNPACK_ROW_LENGTH, w.w);", layers: ['unit'] },
  { id: "desk_gl-upload-offset-is-the-first-pixel", file: "src/engine/desk-gl.ts", find: "w.bits + u.y0 * w.stride + u.x0 * 4", replace: "w.bits + u.y0 * w.w * 4 + u.x0 * 4", layers: ['unit'] },
  { id: "desk_gl-draws-bottom-first", file: "src/engine/desk-gl.ts", find: "    gl.activeTexture(gl.TEXTURE0);\n    for (const w of sc.windows) {", replace: "    gl.activeTexture(gl.TEXTURE0);\n    for (const w of [...sc.windows].reverse()) {", layers: ['unit'] },
  { id: "desk_gl-draws-when-nothing-changed", file: "src/engine/desk-gl.ts", find: "if (sc.updates.length === 0 && !changed && !this.needDraw) return 0;", replace: "if (false as boolean) return 0;", layers: ['unit'] },
  { id: "desk_gl-per-pixel-alpha-is-not-told-from-constant-alpha", file: "src/engine/desk-gl.ts", find: "opaque ? 0 : premul ? 2 : 1", replace: "opaque ? 0 : 1", layers: ['unit'] },
  { id: "desk_gl-glass-is-not-told-from-an-ordinary-window", file: "src/engine/desk-gl.ts", find: "glass ? 3 : opaque ? 0 : premul ? 2 : 1", replace: "opaque ? 0 : premul ? 2 : 1", layers: ['unit'] },
  { id: "desk_gl-ignores-the-region-box", file: "src/engine/desk-gl.ts", find: "gl.uniform4f(this.loc.u_region as WebGLUniformLocation, w.cl, w.ct, w.cr, w.cb);", replace: "gl.uniform4f(this.loc.u_region as WebGLUniformLocation, 0, 0, w.w, w.h);", layers: ['unit'] },
  { id: "desk_gl-accepts-a-software-renderer", file: "src/engine/desk-gl.ts", find: "make().getContext('webgl2', { failIfMajorPerformanceCaveat: true })", replace: "make().getContext('webgl2')", layers: ['unit'] },
  { id: "desk_gl-accepts-a-small-texture-limit", file: "src/engine/desk-gl.ts", find: "(gl.getParameter(gl.MAX_TEXTURE_SIZE) as number) >= GL_MIN_TEXTURE_SIZE", replace: "true", layers: ['unit'] },
  { id: "desk_gl-keeps-textures-of-gone-windows", file: "src/engine/desk-gl.ts", find: "for (const [lid, t] of this.texs) if (!live.has(lid)) { gl.deleteTexture(t.tex); this.texs.delete(lid); }", replace: "", layers: ['unit'] },
  { id: "perf-beacon-desk_since-is-not-a-difference", file: "src/perf/beacon.ts", find: "presents: c.presents - p.presents,", replace: "presents: c.presents,", layers: ['unit'] },
  { id: "perf-beacon-engine-run-busy-share-uses-the-wrong-window", file: "src/perf/beacon.ts", find: "busyFraction: Math.round((sum / wall) * 1000) / 1000", replace: "busyFraction: Math.round((sum / w.length) * 1000) / 1000", layers: ['unit'] },
  { id: "perf-beacon-desk-mode-is-always-2d", file: "src/perf/beacon.ts", find: "mode: typeof (now as Record<string, unknown>).draws === 'number' ? 'gl' : '2d'", replace: "mode: '2d'", layers: ['unit'] },
  { id: "perf-beacon-omits-the-access-cookie", file: "src/perf/beacon.ts", find: "credentials: 'same-origin',", replace: "credentials: 'omit',", layers: ['unit'] },
  { id: "perf-beacon-start-up-in-ms", file: "src/perf/beacon.ts", find: "firstIdleSeconds: typeof idleMs === 'number' ? idleMs / 1000 : null,", replace: "firstIdleSeconds: typeof idleMs === 'number' ? idleMs : null,", layers: ['unit'] },
  { id: "perf-browser-info-reads-no-renderer", file: "src/perf/browser-info.ts", find: "const r = ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : null;", replace: "const r = null as string | null;", layers: ['unit'] },
  { id: "perf-panel-p1-limit-is-exclusive", file: "src/perf/panel.ts", find: "status: v.firstIdleSeconds <= TARGETS.startupSeconds ? 'met' : 'miss' });", replace: "status: v.firstIdleSeconds < TARGETS.startupSeconds ? 'met' : 'miss' });", layers: ['unit'] },
  { id: "perf-panel-waiting-turns-red-at-the-limit", file: "src/perf/panel.ts", find: "status: v.waitingSeconds > TARGETS.startupSeconds ? 'miss' : 'na' });", replace: "status: v.waitingSeconds >= TARGETS.startupSeconds ? 'miss' : 'na' });", layers: ['unit'] },
  { id: "perf-panel-rate-met-below-target", file: "src/perf/panel.ts", find: "(u.perSecond >= TARGETS.updatesPerSecond ? 'met' : 'miss')", replace: "(u.perSecond >= TARGETS.updatesPerSecond / 2 ? 'met' : 'miss')", layers: ['unit'] },
  { id: "perf-panel-underruns-not-measured-is-met", file: "src/perf/panel.ts", find: "status: v.underruns === null ? 'na' : v.underruns === 0 ? 'met' : 'miss' });", replace: "status: v.underruns === null || v.underruns === 0 ? 'met' : 'miss' });", layers: ['unit'] },
  // ---- decision 0175: the audio output (unit layer: test/unit/audio-*.test.ts runs the worklet source itself) ----
  { id: 'audio-underrun-needs-two-frames-missing', file: 'src/audio/fle-out.worklet.js', find: 'if (have < n) {', replace: 'if (have < n - 1) {', layers: ['unit'] },
  { id: 'audio-output-timeline-stops-on-underrun', file: 'src/audio/fle-out.worklet.js', find: 'Atomics.store(h, W.RD, (rd + n) >>> 0 | 0);', replace: 'if (have === n) Atomics.store(h, W.RD, (rd + n) >>> 0 | 0);', layers: ['unit'] },
  { id: 'audio-episodes-not-counted', file: 'src/audio/fle-out.worklet.js', find: 'if (run === 1) Atomics.add(h, W.UR_RUNS, 1);', replace: '', layers: ['unit'] },
  { id: 'audio-warm-up-never-ends', file: 'src/audio/fle-out.worklet.js', find: 'if (!warm) Atomics.store(h, W.WARM, 1);', replace: '', layers: ['unit'] },
  { id: 'audio-silence-counted-as-sound', file: 'src/audio/fle-out.worklet.js', find: 'if (a !== 0 || b !== 0) anyNonZero = true;', replace: 'anyNonZero = true;', layers: ['unit'] },
  { id: 'audio-longest-run-not-kept', file: 'src/audio/fle-out.worklet.js', find: 'if (run > Atomics.load(h, W.UR_MAXRUN)) Atomics.store(h, W.UR_MAXRUN, run);', replace: '', layers: ['unit'] },
  { id: 'audio-margin-not-posted', file: 'src/audio/fle-out.worklet.js', find: 'if (fill < this.fmin) this.fmin = fill;', replace: '', layers: ['unit'] },
  { id: 'audio-hook-takes-any-message', file: 'src/audio/worker-hook.ts', find: "a.cmd !== 'audio-init' || ", replace: '', layers: ['unit'] },
  // decision 0211: the audio output is ON by default (audio=0 turns it off) and a suspended context must not stop the song
  { id: 'audio-default-rate-48000', file: 'src/audio/install.ts', find: 'sampleRate: 44100, capFrames: 32768', replace: 'sampleRate: 48000, capFrames: 32768', layers: ['unit'] },
  { id: 'audio-off-reason-ignored', file: 'src/audio/install.ts', find: 'if (audioOffReason(win) !== null) return null;', replace: '', layers: ['unit'] },
  { id: 'audio-sink-ignores-the-worklet', file: 'src/audio/soft-sink.ts', find: 'if (q !== this.lastQ) { this.lastQ = q; this.baseAt = now; this.carry = 0; this.engaged = false; return 0; }', replace: 'if (q !== this.lastQ) { this.lastQ = q; }', layers: ['unit'] },
  { id: 'audio-sink-no-grace', file: 'src/audio/soft-sink.ts', find: 'if (!this.engaged && now - this.baseAt < GRACE_MS) return 0;', replace: '', layers: ['unit'] },
  { id: 'audio-sink-moves-when-not-playing', file: 'src/audio/soft-sink.ts', find: 'if (Atomics.load(h, W.STATE) !== STATE_PLAYING) { this.baseAt = now; this.carry = 0; this.engaged = false; return 0; }', replace: '', layers: ['unit'] },
  { id: 'audio-sink-drops-the-fraction', file: 'src/audio/soft-sink.ts', find: 'this.carry = exact - n;', replace: 'this.carry = 0;', layers: ['unit'] },
  { id: 'audio-sink-goes-backwards', file: 'src/audio/soft-sink.ts', find: 'if (!(dt > 0)) return 0;', replace: '', layers: ['unit'] },
  { id: 'audio-sink-never-started', file: 'src/audio/page-audio.ts', find: 'this.sinkTimer = setInterval(() => { this.sink.tick(performance.now()); }, 10);', replace: '', layers: ['unit'] },
  { id: 'audio-resume-result-not-kept', file: 'src/audio/page-audio.ts', find: 'this.lastResume = rec;', replace: '', layers: ['unit'] },
  { id: 'audio-blocked-never-reported', file: 'src/audio/page-audio.ts', find: "now - r.at >= RESUME_WAIT_MS ? 'blocked' : 'resuming'", replace: "false ? 'blocked' : 'resuming'", layers: ['unit'] },
  { id: 'audio-start-resume-counted-as-gesture', file: 'src/audio/page-audio.ts', find: "if (trigger !== 'start') this.gestures++;", replace: 'this.gestures++;', layers: ['unit'] },
  { id: 'audio-rejected-resume-called-pending', file: 'src/audio/page-audio.ts', find: "result = 'rejected';", replace: "result = 'pending';", layers: ['unit'] },
  { id: 'audio-resume-while-running', file: 'src/audio/page-audio.ts', find: "if (!ctx || ctx.state === 'running') return null;", replace: 'if (!ctx) return null;', layers: ['unit'] },
  { id: 'audio-status-needs-gesture-wording', file: 'src/audio/status.ts', find: "text: '音: 停止中 (クリックまたはキー入力で有効になります)'", replace: "text: '音: 動作中'", layers: ['unit'] },
  { id: 'audio-status-not-refreshed-on-change', file: 'src/audio/status.ts', find: 'if (audio) { audio.onChange(render); setTimer(render, 1000); }', replace: 'if (audio) { setTimer(render, 1000); }', layers: ['unit'] },
  { id: 'perf-schema-running-without-running-context', file: 'shared/perf.ts', find: "if (d.status === 'running' && d.ctxState !== 'running')", replace: 'if (false)', layers: ['unit'] },
  { id: 'perf-schema-running-without-worklet-node', file: 'shared/perf.ts', find: "if (d.status === 'running' && d.nodeReady !== true)", replace: 'if (false)', layers: ['unit'] },
  { id: 'audio-timeline-capture-shifts-after-a-gap', file: 'src/audio/fle-out.worklet.js', find: 'if (this.cp !== null && this.cp.timeline && this.cp.started) for', replace: 'if (false) for', layers: ['unit'] },
  { id: 'audio-pulses-unbounded', file: 'src/audio/page-audio.ts', find: 'if (list.length > max) list.splice(0, list.length - max);', replace: '', layers: ['unit'] },
  { id: 'audio-engine-gap-not-reported', file: 'src/audio/page-audio.ts', find: 'engineGapFrames: last.gapFrames, engineSkippedFrames: last.skipped,', replace: 'engineGapFrames: 0, engineSkippedFrames: 0,', layers: ['unit'] },
  { id: 'audio-late-write-not-reported', file: 'src/audio/page-audio.ts', find: 'engineLateWriteLost: last.rewrite?.lost ?? null', replace: 'engineLateWriteLost: 0', layers: ['unit'] },
  { id: 'perf-schema-blocked-without-gesture', file: 'shared/perf.ts', find: "if (d.status === 'blocked' && (typeof d.gestures === 'number' && d.gestures === 0))", replace: 'if (false)', layers: ['unit'] },
  { id: 'audio-p3-counts-nothing', file: 'src/audio/underrun.ts', find: 'get p3(): number { return this.c.urSteady; }', replace: 'get p3(): number { return 0; }', layers: ['unit'] },
  // ---- decision 0192: the screen-side measurement (src/perf/display.ts, panel.ts, browser-info.ts, beacon.ts, shared/perf.ts, scripts/perf-log-summary.mjs) ----
  { id: 'display-hz-from-the-mean', file: 'src/perf/display.ts', find: 'hz: med !== null && med > 0 ? round(1000 / med, 1) : null,', replace: 'hz: q.mean !== null && q.mean > 0 ? round(1000 / q.mean, 1) : null,', layers: ['unit'] },
  { id: 'display-hz-from-the-p95', file: 'src/perf/display.ts', find: 'const med = q.q(50), p10 = q.q(10);', replace: 'const med = q.q(95), p10 = q.q(10);', layers: ['unit'] },
  { id: 'display-peak-from-the-median', file: 'src/perf/display.ts', find: 'peakHz: p10 !== null && p10 > 0 ? round(1000 / p10, 1) : null,', replace: 'peakHz: med !== null && med > 0 ? round(1000 / med, 1) : null,', layers: ['unit'] },
  { id: 'display-hidden-chain-not-cut', file: 'src/perf/display.ts', find: '    this.changes++;\n    this.lastTick = null;\n    this.drawsSince = 0;', replace: '    this.changes++;\n    this.drawsSince = 0;', layers: ['unit'] },
  { id: 'display-ticks-while-hidden-counted', file: 'src/perf/display.ts', find: '    if (!this.visible || !this.observing) return;\n    const dt = this.lastTick === null', replace: '    const dt = this.lastTick === null', layers: ['unit'] },
  { id: 'display-rate-over-the-whole-elapsed-time', file: 'src/perf/display.ts', find: 'const totalVis = this.observedUntil(now);', replace: 'const totalVis = now - started;', layers: ['unit'] },
  { id: 'display-fresh-counts-draws-not-frames', file: 'src/perf/display.ts', find: '    if (this.drawsSince > 0) a.fresh++;', replace: '    a.fresh += this.drawsSince;', layers: ['unit'] },
  { id: 'display-slow-is-3x-median', file: 'src/perf/display.ts', find: 'export const SLOW_FACTOR = 1.5;', replace: 'export const SLOW_FACTOR = 3;', layers: ['unit'] },
  { id: 'display-recent-window-is-30-s', file: 'src/perf/display.ts', find: 'export const RECENT_MS = 10_000;', replace: 'export const RECENT_MS = 30_000;', layers: ['unit'] },
  { id: 'display-draw-time-from-the-post', file: 'src/perf/display.ts', find: 'const dm = Math.max(0, drawnAt - recvAt);', replace: 'const dm = Math.max(0, drawnAt - (postedAt ?? recvAt));', layers: ['unit'] },
  { id: 'display-negative-transit-kept', file: 'src/perf/display.ts', find: 'const tm = postedAt === undefined ? null : Math.max(0, recvAt - postedAt);', replace: 'const tm = postedAt === undefined ? null : recvAt - postedAt;', layers: ['unit'] },
  { id: 'display-histogram-bin-0-5-ms', file: 'src/perf/display.ts', find: 'export const HIST_BIN_MS = 0.05;', replace: 'export const HIST_BIN_MS = 0.5;', layers: ['unit'] },
  { id: 'display-histogram-range-50-ms', file: 'src/perf/display.ts', find: 'export const RAF_RANGE_MS = 250;', replace: 'export const RAF_RANGE_MS = 50;', layers: ['unit'] },
  { id: 'display-ring-never-evicted', file: 'src/perf/display.ts', find: 'const horizon = now - RECENT_MS - 1000;', replace: 'const horizon = -Infinity;', layers: ['unit'] },
  { id: 'display-idle-window-includes-the-start-up', file: 'src/perf/display.ts', find: '    this.idle = new Acc();\n    this.idle.visibleBase = this.observedUntil(t);', replace: '    this.idle = this.total;\n    this.idle.visibleBase = 0;', layers: ['unit'] },
  { id: 'display-markidle-before-start-forgotten', file: 'src/perf/display.ts', find: '    if (this.idleMarked) this.markIdle();', replace: '', layers: ['unit'] },
  { id: 'display-same-timestamp-is-a-frame', file: 'src/perf/display.ts', find: 'if (dt !== null && !(dt > 0)) return;', replace: '', layers: ['unit'] },
  { id: 'display-longtask_since-idle-counts-all', file: 'src/perf/display.ts', find: 'if (this.idleAt !== null && start >= this.idleAt) {', replace: 'if (this.idleAt !== null) {', layers: ['unit'] },
  { id: 'display-event-threshold-100-ms', file: 'src/perf/display.ts', find: 'export const EVENT_THRESHOLD_MS = 16;', replace: 'export const EVENT_THRESHOLD_MS = 104;', layers: ['unit'] },
  { id: 'display-met-hz-is-half-the-target', file: 'src/perf/display.ts', find: 'export const DISPLAY_MET_HZ = 0.95 * DISPLAY_TARGET_HZ;', replace: 'export const DISPLAY_MET_HZ = 0.5 * DISPLAY_TARGET_HZ;', layers: ['unit'] },
  { id: 'hzfit-ratio-inverted', file: 'src/perf/display.ts', find: 'const ratio = round(f.engineHz / rafHz, 3);', replace: 'const ratio = round(rafHz / f.engineHz, 3);', layers: ['unit'] },
  { id: 'hzfit-page-behind-ignored', file: 'src/perf/display.ts', find: 'if (peakHz !== null && peakHz > 1.3 * rafHz) return', replace: 'if (false as boolean) return', layers: ['unit'] },
  { id: 'hzfit-tolerance-50-percent', file: 'src/perf/display.ts', find: 'if (ratio < 0.9) return', replace: 'if (ratio < 0.5) return', layers: ['unit'] },
  { id: 'hzfit-whole-run-before-idle-window', file: 'src/perf/display.ts', find: 'const s = raf.sinceIdle && raf.sinceIdle.n >= 30 ? raf.sinceIdle : null;', replace: 'const s = null as typeof raf.sinceIdle;', layers: ['unit'] },
  { id: 'hzclass-band-too-wide', file: 'src/perf/display.ts', find: 'return bd < 0.06 ? best : null;', replace: 'return bd < 0.3 ? best : null;', layers: ['unit'] },
  { id: 'panel-60-hz-screen-not-named', file: 'src/perf/panel.ts', find: 'return c < TARGETS.updatesPerSecond ?', replace: 'return false as boolean ?', layers: ['unit'] },
  { id: 'panel-page-behind-not-named', file: 'src/perf/panel.ts', find: 'if (peakHz !== null && peakHz > 1.3 * hz) return', replace: 'if (false as boolean) return', layers: ['unit'] },
  { id: 'panel-long-tasks-never-red', file: 'src/perf/panel.ts', find: "target: '0 回', status: !lt.supported || lt.sinceIdle === null ? 'na' : lt.sinceIdle === 0 ? 'met' : 'miss',", replace: "target: '0 回', status: !lt.supported || lt.sinceIdle === null ? 'na' : 'met',", layers: ['unit'] },
  { id: 'panel-idle-screen-judged-by-freshness', file: 'src/perf/panel.ts', find: 'const moving = enough && (c.drawsPerSecond ?? 0) >= FRESH_MIN_DRAWS_PER_SECOND;', replace: 'const moving = enough;', layers: ['unit'] },
  { id: 'panel-draw-budget-one-frame', file: 'src/perf/panel.ts', find: 'const DRAW_P95_MAX_MS = 4.2;', replace: 'const DRAW_P95_MAX_MS = 8.4;', layers: ['unit'] },
  { id: 'panel-dropped-frames-5-percent', file: 'src/perf/panel.ts', find: 'const DROPPED_MAX_RATIO = 0.01;', replace: 'const DROPPED_MAX_RATIO = 0.05;', layers: ['unit'] },
  { id: 'panel-notice-omits-the-ip-address', file: 'src/perf/panel.ts', find: '送らないもの: IP アドレス(', replace: '送らないもの: (', layers: ['unit'] },
  { id: 'panel-hz-mismatch-green', file: 'src/perf/panel.ts', find: "target: '一致', status: h.fit === 'match' ? 'met' : h.fit === 'unknown' ? 'na' : 'miss',", replace: "target: '一致', status: h.fit === 'unknown' ? 'na' : 'met',", layers: ['unit'] },
  { id: 'beacon-size-limit-4x', file: 'src/perf/beacon.ts', find: 'export function beaconText(p: BeaconPayload, max = PERF_MAX_BODY - 512): string {', replace: 'export function beaconText(p: BeaconPayload, max = PERF_MAX_BODY * 4): string {', layers: ['unit'] },
  { id: 'beacon-version-stays-1', file: 'src/perf/beacon.ts', find: 'export const BEACON_VERSION = 3;', replace: 'export const BEACON_VERSION = 2;', layers: ['unit'] },
  { id: 'beacon-display-block-dropped', file: 'src/perf/beacon.ts', find: '    display: i.display ?? null,', replace: '    display: null,', layers: ['unit'] },
  { id: 'peer-forwarded-for-trusted-from-any-socket', file: 'shared/perf.ts', find: "if (direct !== 'loopback' || !forwardedFor) return direct;", replace: 'if (!forwardedFor) return direct;', layers: ['unit'] },
  { id: 'peer-forwarded-for-ignored', file: 'shared/perf.ts', find: 'return wellFormed ? classifyRemote(a) : direct;', replace: 'return direct;', layers: ['unit'] },
  { id: 'peer-forwarded-for-garbage-is-other', file: 'shared/perf.ts', find: 'return wellFormed ? classifyRemote(a) : direct;', replace: 'return classifyRemote(a);', layers: ['unit'] },
  { id: 'schema-physical-cores-may-be-a-number', file: 'shared/perf.ts', find: "if (!isObj(pc) || pc.value !== null || typeof pc.reason", replace: "if (!isObj(pc) || typeof pc.reason", layers: ['unit'] },
  { id: 'schema-bins-sum-not-checked', file: 'shared/perf.ts', find: "!== v.n) out.push(`${path}.bins: the counts do not add up to n`);", replace: "!== v.n && false) out.push(`${path}.bins: the counts do not add up to n`);", layers: ['unit'] },
  { id: 'client-edge-reported-as-chrome', file: 'src/perf/browser-info.ts', find: "const b = pick(/Edg(?:e|A|iOS)?\\/([\\d.]+)/, 'Edge') ?? pick(/OPR\\/([\\d.]+)/, 'Opera') ?? pick(/Firefox\\/([\\d.]+)/, 'Firefox') ?? pick(/(?:HeadlessChrome|Chrome|CriOS)\\/([\\d.]+)/, 'Chrome')", replace: "const b = pick(/(?:HeadlessChrome|Chrome|CriOS)\\/([\\d.]+)/, 'Chrome') ?? pick(/Edg(?:e|A|iOS)?\\/([\\d.]+)/, 'Edge') ?? pick(/OPR\\/([\\d.]+)/, 'Opera') ?? pick(/Firefox\\/([\\d.]+)/, 'Firefox')", layers: ['unit'] },
  { id: 'client-version-with-four-parts', file: 'src/perf/browser-info.ts', find: "version: parts.length >= 2 ? `${parts[0]}.${parts[1]}` : (parts[0] ?? null),", replace: "version: m[1] ?? null,", layers: ['unit'] },
  { id: 'client-physical-cores-invented', file: 'src/perf/browser-info.ts', find: "physicalCores: { value: null, reason: PHYSICAL_CORES_REASON },", replace: "physicalCores: { value: null, reason: '' },", layers: ['unit'] },
  { id: 'client-host-kind-leaks-the-name', file: 'src/perf/browser-info.ts', find: "if (h.endsWith('.ts.net')) return 'ts.net';", replace: "if (h.endsWith('.ts.net')) return h as 'ts.net';", layers: ['unit'] },
  { id: 'summary-owner-and-this-machine-merged', file: 'shared/perf-summary.mjs', find: "if (remote === 'tailnet') return 'tailnet (the owner via the tailnet)';", replace: "if (remote === 'tailnet' || remote === 'loopback') return 'tailnet (the owner via the tailnet)';", layers: ['unit'] },
  { id: 'summary-session-token-ignored', file: 'shared/perf-summary.mjs', find: "if (r.sid) key = `${r.remote}|sid:${r.sid}`;", replace: "if (false) key = '';", layers: ['unit'] },
  { id: 'summary-software-gpu-not-headless', file: 'shared/perf-summary.mjs', find: "const headless = parsed.headless || (typeof gpu === 'string' && SOFTWARE_GPU.test(gpu));", replace: 'const headless = parsed.headless;', layers: ['unit'] },
  { id: 'summary-stand-in-in-the-groups', file: 'shared/perf-summary.mjs', find: "groups: groupRuns(runList.filter((r) => !r.standIn && r.real > 0)),", replace: "groups: groupRuns(runList.filter((r) => r.real > 0 || r.standIn)),", layers: ['unit'] },
  // the page (e2e): the wiring in main.ts that the unit tests cannot see
  // ---- decision 0193 (fpick1): the page's side of the engine's file dialogs ----
  { id: 'filedlg-device-name-allowed', file: 'src/engine/filedlg.ts', find: 'return !DEVICE.test(stem);', replace: 'return true;', layers: ['unit'] },
  { id: 'filedlg-trailing-dot-allowed', file: 'src/engine/filedlg.ts', find: "if (/[ .]$/.test(c)) return false;", replace: '', layers: ['unit'] },
  { id: 'filedlg-sanitizer-keeps-bad-characters', file: 'src/engine/filedlg.ts', find: ".replace(BAD_CHARS, '_').replace(/[ .]+$/, '')", replace: '', layers: ['unit'] },
  { id: 'filedlg-path-limit-off-by-one', file: 'src/engine/filedlg.ts', find: 'p.length > MAX_PATH_UNITS ||', replace: 'p.length > MAX_PATH_UNITS + 1 ||', layers: ['unit'] },
  { id: 'filedlg-file-limit-exclusive', file: 'src/engine/filedlg.ts', find: "if (file.size > MAX_FILE_BYTES) return { ok: false, reason: `${file.name}", replace: "if (file.size >= MAX_FILE_BYTES) return { ok: false, reason: `${file.name}", layers: ['unit'] },
  { id: 'filedlg-total-limit-off', file: 'src/engine/filedlg.ts', find: 'if (total > MAX_TOTAL_BYTES)', replace: 'if (false as boolean)', layers: ['unit'] },
  { id: 'filedlg-flp-not-preferred', file: 'src/engine/filedlg.ts', find: "let resultIndex = pick(['.flp']);", replace: 'let resultIndex = -1;', layers: ['unit'] },
  { id: 'filedlg-extra-files-all-results', file: 'src/engine/filedlg.ts', find: "role: i === resultIndex ? 'R' : 'E'", replace: "role: 'R'", layers: ['unit'] },
  { id: 'filedlg-save-default-extension-dropped', file: 'src/engine/filedlg.ts', find: "if (!name.includes('.') && req.defaultExt) {\n    const e", replace: "if (false as boolean) {\n    const e", layers: ['unit'] },
  { id: 'filedlg-save-ignores-the-engines-folder', file: 'src/engine/filedlg.ts', find: "const folder = req.folder !== null &&", replace: "const folder = false as boolean && req.folder !== null &&", layers: ['unit'] },
  { id: 'filedlg-parse-filter-shape-unchecked', file: 'src/engine/filedlg.ts', find: "if (!Array.isArray(f)) return null;", replace: '', layers: ['unit'] },
  { id: 'filedlg-bad-request-not-answered', file: 'src/engine/client.ts', find: "this.answerFileDlg(m.id, { ok: false, reason: 'unknown request' }); break; }", replace: "break; }", layers: ['unit'] },
  { id: 'filedlg-no-handler-not-answered', file: 'src/engine/client.ts', find: "else this.answerFileDlg(m.id, { ok: false, reason: 'no dialog in this page' });", replace: '', layers: ['unit'] },
  { id: 'filedlg-hooks-always-installed', file: 'src/engine/fruity.ts', find: '...(host.fileDlg ? {', replace: '...(true ? {', layers: ['unit'] },
  // ---- decision 0220 (cache1): the stored copy of the start-up assets, WebCrypto verification, the beacon's assets block ----
  { id: 'store-write-uses-a-view-not-a-copy', file: 'src/bundle/loader.ts', find: 'const part = buf.slice(flushed, upTo);', replace: 'const part = buf.subarray(flushed, upTo);', layers: ['unit'] },
  { id: 'stored-copy-hash-not-checked', file: 'src/bundle/loader.ts', find: 'if (h.hex !== ref.sha256) return null;', replace: '', layers: ['unit'] },
  { id: 'network-hash-mismatch-ignored-new-path', file: 'src/bundle/loader.ts', find: "if (hex !== ref.sha256) return fail('verify', `SHA-256 mismatch: manifest ${ref.sha256}, received ${hex}`);", replace: '', layers: ['unit'] },
  { id: 'stored-copy-size-not-checked-by-the-store', file: 'src/bundle/assetstore.ts', find: 'if (f.size !== ref.size) return null;\n    this.hold(ref.sha256);', replace: 'this.hold(ref.sha256);', layers: ['unit'] },
  { id: 'clean-ignores-other-pages-locks', file: 'src/bundle/assetstore.ts', find: "await locks.request(lockName(m[1] as string), { mode: 'exclusive', ifAvailable: true }, async (lock) => { if (lock) await rm(); else res.busy++; });", replace: 'await rm();', layers: ['unit'] },
  { id: 'clean-removes-the-current-version', file: 'src/bundle/assetstore.ts', find: 'if (want.has(n)) { res.kept++; continue; }', replace: '', layers: ['unit'] },
  { id: 'store-quota-not-checked-before-writing', file: 'src/bundle/assetstore.ts', find: 'if (est.quotaMiB !== null && est.usedMiB !== null && (est.quotaMiB - est.usedMiB) * 1024 * 1024 < ref.size * 1.25) throw', replace: 'if (false as boolean) throw', layers: ['unit'] },
  { id: 'beacon-assets-fail-text-allowed', file: 'shared/perf.ts', find: "if (it.fail !== null && !inList(PERF_ASSET_FAIL_CODES, it.fail)) out.push(`${p}.fail: ${JSON.stringify(it.fail)} is not a code of the list`);", replace: '', layers: ['unit'] },
  { id: 'report-origin-from-storage-says-network', file: 'src/bundle/assets-report.ts', find: "const net = t.origin === 'network' ? classifyNetwork(lookup(t.url)) : { origin: 'storage-cache' as const, wire: null, transfer: null };", replace: 'const net = classifyNetwork(lookup(t.url));', layers: ['unit'] },
  { id: 'wasm-buffer-allocation-failure-not-handled', file: 'src/bundle/loader.ts', find: 'if (!(e instanceof BufferUnavailable)) throw e;', replace: 'throw e;', layers: ["unit"] },
  // ---- decision 0212 (screen1): the guest's screen is the room; the header choice; ?dpi=; the display change ----
  { id: 'screen-size-rounded-not-floored', file: 'src/ui/screen-fit.ts', find: 'Math.max(lo, Math.min(hi, Math.floor(v)))', replace: 'Math.max(lo, Math.min(hi, Math.round(v)))', layers: ['unit'] },
  { id: 'screen-room-changes-not-debounced', file: 'src/ui/screen-fit.ts', find: 'if (this.timer !== null) this.timers.clear(this.timer);\n    this.timer = this.timers.set(', replace: 'this.timer = this.timers.set(', layers: ['unit'] },
  { id: 'screen-appview-reads-the-wrong-word', file: 'src/engine/appview.ts', find: 'screenW: dv.getUint32(48, true), screenH: dv.getUint32(52, true)', replace: 'screenW: dv.getUint32(44, true), screenH: dv.getUint32(52, true)', layers: ['unit'] },
  { id: 'screen-beacon-mode-not-checked', file: 'shared/perf.ts', find: "if (typeof g.mode !== 'string' || !['fit', 'fit-device', 'fixed', 'legacy'].includes(g.mode))", replace: 'if (false as boolean)', layers: ['unit'] },
  // ---- decision 0223: the operating-system state the Worker reports (os-state.ts), the successor of the page's main-window determination (main-window.ts) ----
  { id: 'os-title-takes-an-inactive-window', file: 'src/engine/os-state.ts', find: "const act = v.windows.find((w) => w.active && w.visible && w.title.trim() !== '');", replace: "const act = v.windows.find((w) => w.visible && w.title.trim() !== '');", layers: ['unit'] },
  { id: 'os-title-takes-a-hidden-window', file: 'src/engine/os-state.ts', find: "const act = v.windows.find((w) => w.active && w.visible && w.title.trim() !== '');", replace: "const act = v.windows.find((w) => w.active && w.title.trim() !== '');", layers: ['unit'] },
  { id: 'os-title-blank-active-window-blanks-the-title', file: 'src/engine/os-state.ts', find: "const act = v.windows.find((w) => w.active && w.visible && w.title.trim() !== '');", replace: "const act = v.windows.find((w) => w.active && w.visible);", layers: ['unit'] },
  { id: 'os-title-last-active-window-wins', file: 'src/engine/os-state.ts', find: "const act = v.windows.find((w) => w.active && w.visible && w.title.trim() !== '');", replace: "const act = [...v.windows].reverse().find((w) => w.active && w.visible && w.title.trim() !== '');", layers: ['unit'] },
  { id: 'os-title-not-trimmed', file: 'src/engine/os-state.ts', find: "title: act ? act.title.trim() : lastTitle", replace: "title: act ? act.title : lastTitle", layers: ['unit'] },
  { id: 'os-key-ignores-the-screen-height', file: 'src/engine/os-state.ts', find: "${s.screenW}x${s.screenH}/${s.title}`", replace: "${s.screenW}x/${s.title}`", layers: ['unit'] },
  { id: 'os-screen-size-defaults-to-1920', file: 'src/engine/os-state.ts', find: "screenW: v.screenW ?? 0, screenH: v.screenH ?? 0 };", replace: "screenW: v.screenW ?? 1920, screenH: v.screenH ?? 0 };", layers: ['unit'] },
  { id: 'os-cursor-shared-dropped', file: 'src/engine/os-state.ts', find: "cursorShared: v.cursorShared, title:", replace: "cursorShared: false, title:", layers: ['unit'] },
  { id: 'latency-answer-ignores-the-seq-list', file: 'src/ui/latency.ts', find: "for (const seq of seqs) {\n      const r = this.bySeq.get(seq);", replace: "for (const seq of [...this.bySeq.keys()]) {\n      const r = this.bySeq.get(seq);", layers: ['unit'] },
  { id: 'latency-answer-credits-an-unprocessed-input', file: 'src/ui/latency.ts', find: "if (!r || r.tInject === null || r.tIdle === null || r.outcome === 'window' || r.outcome === 'stream') continue;", replace: "if (!r || r.outcome === 'window' || r.outcome === 'stream') continue;", layers: ['unit'] },
  { id: 'latency-answer-counts-an-input-twice', file: 'src/ui/latency.ts', find: "if (!r || r.tInject === null || r.tIdle === null || r.outcome === 'window' || r.outcome === 'stream') continue;", replace: "if (!r || r.tInject === null || r.tIdle === null) continue;", layers: ['unit'] },
  { id: 'latency-answer-leaves-a-late-input-late', file: 'src/ui/latency.ts', find: "if (r.outcome === 'late') this.lateCount--;\n    else if (r.outcome === 'none') this.noAnswerCount--;", replace: "if (false) this.lateCount--;", layers: ['unit'] },
  // ---- decision 0223: display method D (desk.ts: the compositor's report, the presenter; ring.ts: the input ring; client.ts: the page's end of the ring). The runtime's compositor (lift/runtime/fle_desk.c) is mutated by tests/desk1_compose.rs's own list (tables/mutations.txt of the decision) ----
  { id: 'desk-put-offset-ignored', file: 'src/engine/desk.ts', find: "this.ctx.putImageData(this.view.image, 0, 0, r.x0, r.y0, w, h);", replace: "this.ctx.putImageData(this.view.image, 0, 0, 0, 0, w, h);", layers: ['unit'] },
  { id: 'desk_canvas-resize-does-not-ask_for-the-whole-picture', file: 'src/engine/desk.ts', find: "this.surface.height = st.height; this.api.resync(); return this.present(); }", replace: "this.surface.height = st.height; return this.present(); }", layers: ['unit'] },
  { id: 'desk_state-magic-not-checked', file: 'src/engine/desk.ts', find: "if (dv.getUint32(0, true) !== DESK_MAGIC || dv.getUint32(4, true) !== DESK_VERSION) return null;", replace: "if (false as boolean) return null;", layers: ['unit'] },
  { id: 'desk_rect-count-unbounded', file: 'src/engine/desk.ts', find: "const n = Math.min(dv.getUint32(28, true), DESK_MAX_RECTS);", replace: "const n = dv.getUint32(28, true);", layers: ['unit'] },
  { id: 'desk-whole-flag-lost', file: 'src/engine/desk.ts', find: "whole: (dv.getUint32(36, true) & DESK_FLAG_WHOLE) !== 0", replace: "whole: false", layers: ['unit'] },
  { id: 'desk-empty-rectangle-put', file: 'src/engine/desk.ts', find: "if (w <= 0 || h <= 0) continue;", replace: "if (false as boolean) continue;", layers: ['unit'] },
  { id: 'ring-full-overwrites', file: 'src/input/ring.ts', find: "if (head - tail >= RING_CAPACITY) { Atomics.add(this.i32, FULL, 1); return false; }", replace: "if (head - tail >= RING_CAPACITY) { Atomics.add(this.i32, FULL, 1); }", layers: ['unit'] },
  { id: 'ring-tail-not-advanced', file: 'src/input/ring.ts', find: "Atomics.store(this.i32, TAIL, tail + 1);\n", replace: "\n", layers: ['unit'] },
  { id: 'ring-page-clock-truncated', file: 'src/input/ring.ts', find: "this.f64[(r + 6) >> 1] = tPage;", replace: "this.f64[(r + 6) >> 1] = Math.fround(tPage);", layers: ['unit'] },
  { id: 'ring-key-scan-and-vk-swapped', file: 'src/input/ring.ts', find: "this.i32[r + 3] = ev.vk; this.i32[r + 4] = ev.scan;", replace: "this.i32[r + 3] = ev.scan; this.i32[r + 4] = ev.vk;", layers: ['unit'] },
  { id: 'ring-display-size-swapped', file: 'src/input/ring.ts', find: "this.i32[r + 3] = ev.width; this.i32[r + 4] = ev.height;", replace: "this.i32[r + 3] = ev.height; this.i32[r + 4] = ev.width;", layers: ['unit'] },
  { id: 'ring-parked-inverted', file: 'src/input/ring.ts', find: "get parked(): boolean { return Atomics.load(this.i32, PARKED) === 1; }", replace: "get parked(): boolean { return Atomics.load(this.i32, PARKED) === 0; }", layers: ['unit'] },
  { id: 'ring-mouse-button-data-lost', file: 'src/input/ring.ts', find: "this.i32[r + 4] = ev.y; this.i32[r + 5] = ev.data; }", replace: "this.i32[r + 4] = ev.y; this.i32[r + 5] = 0; }", layers: ['unit'] },
  { id: 'client-input-never-uses-the-ring', file: 'src/engine/client.ts', find: "if (this.ring.push(seq, tPage, ev)) {", replace: "if (false as boolean && this.ring.push(seq, tPage, ev)) {", layers: ['unit'] },
  { id: 'client-wake-never-sent', file: 'src/engine/client.ts', find: "if (this.ring.parked) { this.ringStats.wakes++;", replace: "if (false as boolean) { this.ringStats.wakes++;", layers: ['unit'] },
  { id: 'client-wake-always-sent', file: 'src/engine/client.ts', find: "if (this.ring.parked) { this.ringStats.wakes++;", replace: "if (true as boolean) { this.ringStats.wakes++;", layers: ['unit'] },
  { id: 'client-full-ring-event-lost', file: 'src/engine/client.ts', find: "      this.ringStats.viaMessage++;\n    }\n    this.worker?.postMessage({ cmd: 'input', seq, tPage, ev } satisfies ToWorker);", replace: "      this.ringStats.viaMessage++;\n      return;\n    }\n    this.worker?.postMessage({ cmd: 'input', seq, tPage, ev } satisfies ToWorker);", layers: ['unit'] },
  // ---- decision 0224: the Worker keeps each measurement beacon as one R2 object and GET /__perf/summary reads them back ----
  { id: 'perf-key-sid-not-checked', file: 'shared/perf.ts', find: "typeof value.sid === 'string' && /^[0-9a-f]{12}$/.test(value.sid) ? value.sid :", replace: "typeof value.sid === 'string' ? value.sid :", layers: ['unit', 'worker'] },
  { id: 'perf-key-date-not-utc', file: 'shared/perf.ts', find: "${now.toISOString().slice(0, 10)}/${sid}", replace: "${new Date(now.getTime() + 9 * 3600e3).toISOString().slice(0, 10)}/${sid}", layers: ['unit'] },
  { id: 'perf-key-seq-not-checked', file: 'shared/perf.ts', find: "const seq = typeof value.seq === 'number' && Number.isInteger(value.seq) && value.seq >= 0 && value.seq <= 999_999_999 ? value.seq : 0;", replace: "const seq = typeof value.seq === 'number' ? value.seq : 0;", layers: ['unit'] },
  { id: 'perf-store-keeps-a-peer-class', file: 'worker/perf-store.ts', find: "perfLine(value, now, 'workers')", replace: "perfLine(value, now, 'other')", layers: ['unit', 'worker'] },
  { id: 'perf-store-failure-is-fatal', file: 'worker/perf-store.ts', find: "    return false;\n  }\n}", replace: "    throw e;\n  }\n}", layers: ['unit'] },
  { id: 'perf-store-without-a-binding-crashes', file: 'worker/perf-store.ts', find: "if (!bucket) return false;", replace: "", layers: ['unit', 'worker'] },
  { id: 'perf-store-wrong-content-type', file: 'worker/perf-store.ts', find: "{ httpMetadata: { contentType: 'application/json' } }", replace: "{}", layers: ['unit', 'worker'] },
  { id: 'perf-worker-always-says-stored', file: 'worker/index.ts', find: "[PERF_STORED_HEADER]: stored ? 'yes' : 'no' }", replace: "[PERF_STORED_HEADER]: 'yes' }", layers: ['worker'] },
  { id: 'perf-worker-stores-before-checking-the-body', file: 'worker/index.ts', find: "  const body = parsePerfBody(all);\n  if (!body.ok) return text(body.status, body.body, noStore);", replace: "  const body = parsePerfBody(all);\n  if (!body.ok) { await storeBeacon(env.PERF_BUCKET, { raw: 'bad' }); return text(body.status, body.body, noStore); }", layers: ['worker'] },
  { id: 'perf-summary-no-cap', file: 'worker/perf-summary.ts', find: "listed.length > PERF_SUMMARY_MAX_OBJECTS ? [...listed]", replace: "false ? [...listed]", layers: ['unit'] },
  { id: 'perf-summary-oldest-first', file: 'worker/perf-summary.ts', find: "b.uploaded - a.uploaded ||", replace: "a.uploaded - b.uploaded ||", layers: ['unit'] },
  { id: 'perf-summary-unbounded-reads-at-once', file: 'worker/perf-summary.ts', find: "const READ_AT_ONCE = 6;", replace: "const READ_AT_ONCE = 100;", layers: ['unit'] },
  { id: 'perf-summary-days-not-checked', file: 'worker/perf-summary.ts', find: "if (days < 1 || days > PERF_SUMMARY_MAX_DAYS)", replace: "if (days < 0)", layers: ['unit', 'worker'] },
  { id: 'perf-summary-reads-one-day-too-few', file: 'worker/perf-summary.ts', find: "for (let i = 0; i < days; i++) out.push(", replace: "for (let i = 0; i < days - 1; i++) out.push(", layers: ['unit'] },
  { id: 'perf-summary-follows-no-cursor', file: 'worker/perf-summary.ts', find: "cursor = page.truncated ? page.cursor : undefined;", replace: "cursor = undefined;", layers: ['unit'] },
  { id: 'perf-summary-missing-object-fatal', file: 'worker/perf-summary.ts', find: "if (o === null) { missing++; continue; }", replace: "if (o === null) throw new Error('gone');", layers: ['unit'] },
  { id: 'perf-summary-not-sorted-by-time', file: 'shared/perf-summary.mjs', find: "keyed.sort((a, b) => (a.t < b.t ? -1 : a.t > b.t ? 1 : a.seq - b.seq || a.i - b.i));", replace: "", layers: ['unit'] },
  { id: 'perf-summary-rows-limit-ignored', file: 'shared/perf-summary.mjs', find: "const shown = typeof opts.rows === 'number' && opts.rows >= 0 && opts.rows < s.rows.length ?", replace: "const shown = false ?", layers: ['unit', 'worker'] },
  { id: 'perf-summary-workers-machine-unnamed', file: 'shared/perf-summary.mjs', find: "if (remote === 'workers') return", replace: "if (remote === 'workersx') return", layers: ['unit', 'worker'] },
  { id: 'perf-summary-route-answers-post', file: 'worker/index.ts', find: "async function handlePerfSummary(request: Request, env: Env, url: URL): Promise<Response> {\n  if (request.method !== 'GET' && request.method !== 'HEAD') return text(405, 'method not allowed', { Allow: 'GET, HEAD' });", replace: "async function handlePerfSummary(request: Request, env: Env, url: URL): Promise<Response> {", layers: ['worker'] },
  { id: 'first-pixel-after-idle-takes-the-page-back-to-running', file: 'src/engine/start-state.ts', find: "return current === 'loaded' || current === 'running' ? 'running' : current;", replace: "return 'running';", layers: ['unit'] },
  { id: 'desk_reads-the-memory-before-compose', file: 'src/engine/desk.ts', find: "    const ptr = this.api.compose();\n    let st: DeskState | null;\n    try { st = readDeskState(this.memory() as ArrayBufferLike, ptr);", replace: "    const bufBefore = this.memory() as ArrayBufferLike;\n    const ptr = this.api.compose();\n    let st: DeskState | null;\n    try { st = readDeskState(bufBefore, ptr);", layers: ['unit'] },
  { id: 'wasm-has-desk_needs-only-one-entry-point', file: 'scripts/lib/wasm-exports.mjs', find: "return DESK_EXPORTS.every((n) => names.has(n));", replace: "return DESK_EXPORTS.some((n) => names.has(n));", layers: ['unit'] },
  { id: 'desk_classify-ignores-the-class-in-the-text', file: 'test/e2e/desk-helpers.mjs', find: "if (w.cls !== undefined) return {", replace: "if (false && w.cls !== undefined) return {", layers: ['unit'] },
  { id: 'desk_classify-takes-a-narrow-popup-for-a-dialog', file: 'test/e2e/desk-helpers.mjs', find: "(w.w >= 300 && w.h >= 100)", replace: "(w.w >= 100 && w.h >= 50)", layers: ['unit'] },
  { id: 'features-let-the-engine-start-without-offscreencanvas', file: 'src/engine/features.ts', find: "  return report.missing;\n}", replace: "  return report.missing.filter((f) => f.name !== 'OffscreenCanvas');\n}", layers: ['unit'] },
  { id: 'desk_sleep-sleeps-at-exactly-afterMs', file: 'src/engine/desk-sleep.ts', find: "if (now - this.quietSince > this.afterMs)", replace: "if (now - this.quietSince >= this.afterMs)", layers: ['unit'] },
  { id: 'desk_sleep-change-does-not-wake', file: 'src/engine/desk-sleep.ts', find: "    this.asleep = false;\n    return was;", replace: "    return was;", layers: ['unit'] },
  // decision 0232: the header is one row whatever the manifest says (a long note used to make the legacy page 1302 px high, 2 px more than a 1300 px window); the picture the compositor reports as whole never reaches the canvas (a canvas older than the picture)
  // ---- decision 0248 (ci1): the file manager keeps its files in the tab's memory when the browser has no usable OPFS (src/files/memory.ts, detectRoot in src/files/opfs.ts) ----
  { id: "files-memory-probe-writes-nothing", file: "src/files/opfs.ts", find: "await w.write(new Uint8Array([1]));", replace: "", layers: ['unit'] },
  { id: "files-memory-probe-size-not-checked", file: "src/files/opfs.ts", find: "if ((await (await d.getFileHandle(probe)).getFile()).size !== 1) throw new FsError('probe', 'The browser stored a different number of bytes than were written.');", replace: "", layers: ['unit'] },
  { id: "files-memory-probe-file-left-in-the-opfs", file: "src/files/opfs.ts", find: "} finally { await d.removeEntry(probe).catch(() => { /* nothing to remove */ }); }", replace: "} finally { /* left behind */ }", layers: ['unit'] },
  { id: "files-memory-securityerror-not-named-blocked", file: "src/files/opfs.ts", find: "(e as { name?: string })?.name === 'SecurityError' || (e as { name?: string })?.name === 'NotAllowedError' ? 'blocked'", replace: "false ? 'blocked'", layers: ['unit'] },
  { id: "files-memory-root-decided-on-every-call", file: "src/files/opfs.ts", find: "(rootP ??= detectRoot());", replace: "detectRoot();", layers: ['unit'] },
  { id: "files-memory-missing-getdirectory-not-detected", file: "src/files/opfs.ts", find: "if (!nav.storage?.getDirectory) why = 'unsupported';", replace: "if (false as boolean) why = 'unsupported';", layers: ['unit'] },
  { id: "files-memory-abort-commits-the-write", file: "src/files/memory.ts", find: "abort: async () => { if (open) { open = false; node.writers--; } },", replace: "abort: async () => { if (open) { open = false; node.writers--; node.blob = new Blob(parts); } },", layers: ['unit'] },
  { id: "files-memory-second-writer-allowed", file: "src/files/memory.ts", find: "if (node.writers > 0) throw domError('NoModificationAllowedError', `${this.name} is open for writing.`);", replace: "", layers: ['unit'] },
  { id: "files-memory-remove-nonempty-folder-allowed", file: "src/files/memory.ts", find: "if (c.children.size > 0 && !options?.recursive) throw domError('InvalidModificationError', `${name} is not empty.`);", replace: "", layers: ['unit'] },
  { id: "files-memory-written-bytes-not-copied", file: "src/files/memory.ts", find: "else if (ArrayBuffer.isView(data)) parts.push(new Uint8Array(data.buffer, data.byteOffset, data.byteLength).slice());", replace: "else if (ArrayBuffer.isView(data)) parts.push(new Uint8Array(data.buffer, data.byteOffset, data.byteLength));", layers: ['unit'] },
  { id: "files-memory-file-where-a-folder-is-wanted-reported-missing", file: "src/files/memory.ts", find: "if (c) throw domError('TypeMismatchError', `${name} is a file.`);", replace: "", layers: ['unit'] },
  { id: "files-memory-removed-file-handle-stays-alive", file: "src/files/memory.ts", find: "      c.removed = true;\n    }\n    this.node.children.delete(name);", replace: "    }\n    this.node.children.delete(name);", layers: ['unit'] },
  { id: "files-memory-modification-time-not-updated", file: "src/files/memory.ts", find: "node.blob = new Blob(parts);\n        node.modified = Date.now();", replace: "node.blob = new Blob(parts);", layers: ['unit'] },
  { id: "files-memory-odd-names-accepted", file: "src/files/memory.ts", find: "n === '' || n === '.' || n === '..' ||", replace: "", layers: ['unit'] },
  { id: "files-memory-probe-name-shared-between-pages", file: "src/files/opfs.ts", find: "const probe = `.fl-probe-${Math.random().toString(36).slice(2, 10)}`;", replace: "const probe = '.fl-probe';", layers: ['unit'] },
  { id: "files-memory-quota-limits-the-memory-tree", file: "src/files/opfs.ts", find: "if (!fs.persistent || s.quota === null || s.usage === null) return null;", replace: "if (s.quota === null || s.usage === null) return null;", layers: ['unit'] },
  { id: "files-memory-room-ignores-the-usage", file: "src/files/opfs.ts", find: "return Math.max(0, s.quota - s.usage);", replace: "return Math.max(0, s.quota);", layers: ['unit'] },
];

if (argv.includes('--list')) { for (const m of M) console.log(m.id, m.layers.join(',')); process.exit(0); }

function copyTree() {
  fs.rmSync(copy, { recursive: true, force: true });
  fs.mkdirSync(copy, { recursive: true });
  for (const e of fs.readdirSync(APP)) {
    if (['node_modules', 'dist', '.wrangler'].includes(e)) continue;
    fs.cpSync(path.join(APP, e), path.join(copy, e), { recursive: true });
  }
  fs.symlinkSync(path.join(APP, 'node_modules'), path.join(copy, 'node_modules'));
}

// APP_LOGS: the e2e runs inside a mutation write their result files to <work>/sublogs, not into the caller's logs
function sh(cmd, args, extraEnv = {}) {
  const r = spawnSync(cmd, args, { cwd: copy, encoding: 'utf8', env: { ...process.env, APP_REPO_ROOT: repoRoot, APP_LOGS: path.join(work, 'sublogs'), ...extraEnv }, maxBuffer: 64 * 1024 * 1024 });
  return { status: r.status, out: (r.stdout || '') + (r.stderr || '') };
}
const failedNames = (out) => [...new Set([...out.matchAll(/^\s*(?:FAIL|×)\s+(.+)$/gm)].map((m) => m[1].trim().slice(0, 150)))];
const failedE2e = (out) => [...out.matchAll(/^FAIL\s+(.+)$/gm)].map((m) => m[1].slice(0, 150));

function runLayer(layer, mut) {
  if (layer === 'unit') {
    const r = sh(path.join(copy, 'node_modules/.bin/vitest'), ['run']);
    const m = /Tests\s+(?:(\d+) failed \| )?(\d+) passed/.exec(r.out);
    return { layer, detected: r.status !== 0, detail: `${failedNames(r.out).length} failing test(s)`, names: failedNames(r.out).slice(0, 6), summary: (/Tests\s+.*$/m.exec(r.out) || [''])[0].trim() };
  }
  if (layer === 'worker') {
    const r = sh('node', ['test/worker/run.mjs']);
    return { layer, detected: r.status !== 0, detail: `${failedNames(r.out).length} failing test(s)`, names: failedNames(r.out).slice(0, 6), summary: (/Tests\s+.*$/m.exec(r.out) || [''])[0].trim() };
  }
  const only = ['--only', mut.e2eOnly ?? 'default-manifest,perf-endpoint'];
  const r = sh('node', ['test/e2e/run.mjs', '--target', 'preview', ...only]);
  const fails = failedE2e(r.out);
  const summary = (/^e2e: .*$/m.exec(r.out) || [''])[0];
  // a run that died before it printed its summary line (a harness failure) is NOT a detection
  const ran = summary !== '';
  return { layer, detected: ran && fails.length > 0, detail: ran ? `${fails.length} failing e2e check(s)` : `HARNESS FAILURE (no summary line): ${r.out.slice(-300)}`, names: fails.slice(0, 6), summary };
}

function simdIdentity() {
  // assemble convert.wat with the shuffle replaced by the identity permutation; return its base64
  const wat = fs.readFileSync(path.join(copy, 'src/engine/convert.wat'), 'utf8').replace('i8x16.shuffle 2 1 0 3 6 5 4 7 10 9 8 11 14 13 12 15', 'i8x16.shuffle 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15');
  const wf = path.join(work, 'mut.wat');
  fs.writeFileSync(wf, wat);
  const r = spawnSync('wat2wasm', [wf, '-o', path.join(work, 'mut.wasm')], { encoding: 'utf8' });
  if (r.status !== 0) throw new Error('wat2wasm failed: ' + r.stderr);
  return fs.readFileSync(path.join(work, 'mut.wasm')).toString('base64');
}

const report = [];
copyTree();
const baseline = runLayer('unit', {});
console.log(`baseline (unmodified copy): unit ${baseline.detected ? 'FAILS (stop)' : 'green'}  ${baseline.summary}`);
if (baseline.detected) process.exit(2);
for (const mut of M) {
  if (only && !only.includes(mut.id)) continue;
  const wanted = mut.layers.filter((l) => !onlyLayers || onlyLayers.includes(l));
  if (wanted.length === 0) continue;   // none of this mutation's layers was asked for: the copy is not touched
  const f = path.join(copy, mut.file);
  const original = fs.readFileSync(f, 'utf8');
  let mutated;
  if (mut.special === 'simd-identity-shuffle') mutated = original.replace(/'AGFz[^']*'/, `'${simdIdentity()}'`);
  else { if (!original.includes(mut.find)) { console.log(`!! ${mut.id}: the text to change was not found in ${mut.file}`); report.push({ id: mut.id, error: 'pattern not found' }); continue; } mutated = original.replace(mut.find, () => mut.replace); } // a function: a string would treat `$'` and `$&` in the replacement as patterns
  if (mutated === original) { console.log(`!! ${mut.id}: the mutation did not change the file`); report.push({ id: mut.id, error: 'no change' }); continue; }
  fs.writeFileSync(f, mutated);
  const layers = wanted.map((l) => runLayer(l, mut));
  fs.writeFileSync(f, original); // back to the original
  const restored = fs.readFileSync(f, 'utf8') === original && fs.readFileSync(path.join(APP, mut.file), 'utf8') === original;
  const detected = layers.some((l) => l.detected);
  report.push({ id: mut.id, file: mut.file, detected, restored, layers });
  console.log(`${detected ? 'DETECTED ' : 'MISSED   '} ${mut.id}  [${mut.file}]  restored=${restored}`);
  for (const l of layers) console.log(`    ${l.layer}: ${l.detected ? 'fails' : 'PASSES (not detected)'} - ${l.detail}${l.names.length ? ': ' + l.names.slice(0, 2).join(' | ') : ''}`);
}
// after all: the copy must be identical to the real tree again, and green
const same = spawnSync('diff', ['-r', '-q', '--exclude=node_modules', '--exclude=dist', '--exclude=.wrangler', APP, copy], { encoding: 'utf8' });
const again = runLayer('unit', {});
console.log(`after all mutations: diff of copy vs tree: ${same.status === 0 ? 'identical' : same.stdout}; unit tests ${again.detected ? 'FAIL' : 'green'}`);
const nDet = report.filter((r) => r.detected).length;
console.log(`\nmutation check: ${nDet}/${report.length} detected`);
fs.writeFileSync(path.join(logsDir, onlyLayers ? `mutation-results-${onlyLayers.join('+')}.json` : 'mutation-results.json'), JSON.stringify({ report, baseline: baseline.summary, copyIdentical: same.status === 0 }, null, 2));
process.exit(nDet === report.length && same.status === 0 ? 0 : 1);
