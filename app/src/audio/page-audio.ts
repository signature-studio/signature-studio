// The page's side of the audio output (decision 0175; always on since decision 0211 and decision 0243). It owns the AudioContext, the 'fle-out' AudioWorkletNode and the SharedArrayBuffer ring, hands the ring to the engine Worker
// (worker-hook.ts) and keeps the numbers the worklet and the engine report. It never changes what the engine writes into the ring: the worklet plays the frames as they are.
import { DATA_OFFSET, makeRingBuffer, readCounters, W, type RingCounters } from './ring-layout.ts';
import { UnderrunStats } from './underrun.ts';
import { SoftSink } from './soft-sink.ts';

export interface PageAudioOptions {
  /** the AudioContext's sample rate; the engine's device rate is 44,100 (SetFormat of FLEngine) and the worklet consumes one frame per output frame, so any other rate would change the speed (S-ur-rate) */
  sampleRate?: number;
  /** ring capacity in frames (power of two); the engine's own ring is 32,768 (S-audio-ring) */
  capFrames?: number;
  /** how often the Worker reports its own counters, ms (0: never) */
  workerReportMs?: number;
  workletUrl: string;
}

export interface WorkletSample { frame: number; time: number; rd: number; wr: number; state: number; peak: number; sumSq: number; n: number; nonSilent: number; fmin: number | null; page: number }
export interface Capture { frames: Float32Array; valid: boolean; firstFrame: number; validFrames: number; page: number }
export interface EnginePulse { real: number; epoch: number; virt: number; consumed: number; written: number; threads: number[][]; mixer: number; unlockCompute: { n: number; sumMs: number; maxMs: number; over78: number }; page: number }
export interface WorkerReport { at: number; unlocks: { first: number; n: number; t: number }[]; waits: number; publishes: number; skipped: number; gapFrames: number; maxWaitDrift: number; pace: Record<string, number> | null; rewrite?: { events: number; applied: number; lost: number } | null; page: number }

/** decision 0211: what the page can say about the state of the audio output (the status line and the beacon carry it); 'idle' = no AudioContext yet (no engine Worker yet) */
export type AudioStatusKind = 'idle' | 'starting' | 'needs-gesture' | 'resuming' | 'running' | 'blocked' | 'error' | 'closed';
/** one resume() call: when (page ms), what asked for it, how it ended within RESUME_WAIT_MS ('running' = the context runs, 'pending' = the promise has not settled, 'rejected' = it threw, 'suspended' = it settled but the context is not running), how long that took */
export interface ResumeRecord { at: number; trigger: string; result: 'running' | 'pending' | 'rejected' | 'suspended' | 'no-context'; ms: number; stateAfter: string; error?: string }
export const RESUME_WAIT_MS = 1500;

export interface AudioDetail {
  /** decision 0211 */
  status: AudioStatusKind;
  /** the AudioWorkletNode exists (addModule finished): false with a running context means the worklet is still loading or failed to load */
  nodeReady: boolean;
  ctxState: string | null; sampleRate: number | null; baseLatencyMs: number | null; outputLatencyMs: number | null;
  autoplayPolicy: string | null;
  gestures: number; resumeAttempts: number; lastResume: ResumeRecord | null;
  softSinkFrames: number; capFrames: number; error: string | null;
  /** decision 0211: what the ENGINE says it lost (its latest 1 Hz report, cumulative; null before the first report): frames of silence it wrote for a hole between two publishes (it was too late to publish them), frames it dropped because the output had passed them. The worklet counts a hole as an underrun only when it reaches it before the engine has filled it with silence, so with an output delay (audiodelay) a hole can be audible and not counted in underrunQuanta */
  engineGapFrames: number | null; engineSkippedFrames: number | null; engineGapFramesLast60s: number | null;
  /** decision 0241: Unlocks of the guest that wrote into frames the engine had already handed to the output (events), the frames of them that were still in time and were handed over again (applied), and the ones too near to the read position to be changed (lost); null before the first report */
  engineLateWrites: number | null; engineLateWriteApplied: number | null; engineLateWriteLost: number | null;
  deviceState: 'closed' | 'open' | 'playing';
  quantaWhileOpen: number; quantaWhilePlaying: number; playingSeconds: number;
  underrunQuanta: number; underrunQuantaSinceFirstIdle: number | null; underrunQuantaSinceFirstSound: number | null; underrunQuantaLast60s: number | null;
  episodes: number; longestRunQuanta: number; secondsSinceLastUnderrun: number | null;
  nonSilentQuanta: number; peak: number; rms: number; fillFrames: number; fillMinFrames: number;
}

/** decision 0211: the audio output is on in every session now, so the logs the page keeps for diagnosis are bounded (the newest entries stay): 10 minutes of the engine's 2 Hz pulses and 1 Hz reports, 4 minutes of unlock times */
export const KEEP_PULSES = 1200;
export const KEEP_REPORTS = 1200;
export const KEEP_UNLOCKS = 2400;
function keepLast<T>(list: T[], x: T, max: number): void {
  list.push(x);
  if (list.length > max) list.splice(0, list.length - max);
}

/** navigator.getAutoplayPolicy (Firefox 112+, Safari 16.4+; Chrome has none): 'allowed' | 'allowed-muted' | 'disallowed', else null */
function autoplayPolicyOf(ctx: AudioContext | null): string | null {
  try {
    const nav = globalThis.navigator as (Navigator & { getAutoplayPolicy?: (t: unknown) => string }) | undefined;
    return ctx && typeof nav?.getAutoplayPolicy === 'function' ? nav.getAutoplayPolicy(ctx) : null;
  } catch { return null; }
}

export class PageAudio {
  readonly opts: Required<Omit<PageAudioOptions, 'workletUrl'>> & { workletUrl: string };
  readonly sab: SharedArrayBuffer;
  readonly header: Int32Array;
  ctx: AudioContext | null = null;
  node: AudioWorkletNode | null = null;
  state: 'idle' | 'starting' | 'running' | 'suspended' | 'error' = 'idle';
  error: string | null = null;
  readonly samples: WorkletSample[] = [];
  readonly pulses: EnginePulse[] = [];
  readonly reports: WorkerReport[] = [];
  readonly workerUnlocks: { first: number; n: number; t: number }[] = [];
  /** stretches the worklet copied on request (armCapture): the engine's own output from its first non-zero sample on */
  readonly captures: Capture[] = [];
  /** performance.now() of the page when the context first ran, and the context's baseLatency / outputLatency then */
  startedAt: number | null = null;
  latency: { base: number; output: number } | null = null;
  /** decision 0211: gestures seen (pointerdown, keydown, ...), the resume() calls and the last one's outcome, listeners told when the status can have changed */
  gestures = 0;
  resumeAttempts = 0;
  lastResume: ResumeRecord | null = null;
  readonly sink: SoftSink;
  private sinkTimer: ReturnType<typeof setInterval> | null = null;
  private readonly listeners = new Set<() => void>();
  /** counter snapshots at named events of the session ('first-idle': the engine's first idle) */
  readonly marks: Record<string, { page: number; c: RingCounters }> = {};
  /** one entry per 50 ms worklet post: when (page ms), the playing quanta and the underrun quanta so far */
  readonly history: { page: number; playing: number; ur: number }[] = [];
  private peak = 0;
  private sumSq = 0;
  private samplesN = 0;
  maxSamplesKept = 12000;

  constructor(o: PageAudioOptions) {
    this.opts = { sampleRate: o.sampleRate ?? 44100, capFrames: o.capFrames ?? 32768, workerReportMs: o.workerReportMs ?? 1000, workletUrl: o.workletUrl };
    this.sab = makeRingBuffer(this.opts.capFrames, this.opts.sampleRate);
    this.header = new Int32Array(this.sab, 0, 32);
    this.sink = new SoftSink(this.header, () => this.opts.sampleRate);
  }

  /** called when the status can have changed (context state, a resume() result, the node ready, an error) */
  onChange(fn: () => void): () => void { this.listeners.add(fn); return () => { this.listeners.delete(fn); }; }
  private changed(): void { for (const fn of this.listeners) { try { fn(); } catch { /* a listener must not stop the audio */ } } }

  /** decision 0211: what to tell the user */
  statusKind(now: number = performance.now()): AudioStatusKind {
    if (this.state === 'error') return 'error';
    if (this.state === 'idle') return 'idle';   // not started: the AudioContext is made when the engine Worker is (install.ts); a page without an engine never makes one
    const ctx = this.ctx;
    if (!ctx) return 'starting';
    if (ctx.state === 'closed') return 'closed';
    if (ctx.state === 'running') return this.node ? 'running' : 'starting';
    if (this.gestures === 0) return 'needs-gesture';
    const r = this.lastResume;
    return r && r.result !== 'running' && now - r.at >= RESUME_WAIT_MS ? 'blocked' : 'resuming';
  }

  /** the message that gives the ring to the engine Worker: post it before the engine's init (messages of one Worker are in order) */
  initMessage(): { cmd: 'audio-init'; sab: SharedArrayBuffer; cap: number; dataOffset: number; reportMs: number } {
    return { cmd: 'audio-init', sab: this.sab, cap: this.opts.capFrames, dataOffset: DATA_OFFSET, reportMs: this.opts.workerReportMs };
  }

  /** wires the Worker's messages (the engine's audio-pulse and the hook's aud2-worker) */
  watch(worker: Worker): void {
    worker.addEventListener('message', (ev: MessageEvent) => {
      const m = ev.data as { type?: string; t?: string } | null;
      if (!m || typeof m !== 'object') return;
      if (m.t === 'idle' && !this.marks['first-idle']) this.mark('first-idle');
      if (m.type === 'audio-pulse') keepLast(this.pulses, { ...(m as unknown as EnginePulse), page: performance.now() }, KEEP_PULSES);
      else if (m.type === 'aud2-worker') {
        const r = m as unknown as WorkerReport;
        keepLast(this.reports, { ...r, page: performance.now() }, KEEP_REPORTS);
        for (const u of r.unlocks) keepLast(this.workerUnlocks, u, KEEP_UNLOCKS);
      }
    });
  }

  /** creates the context and the node; a context made before any gesture is suspended by the browser's autoplay policy, resume() (from the first gesture, see resume) starts it. The software sink keeps the transport going meanwhile. */
  async start(): Promise<void> {
    if (this.state !== 'idle') return;
    this.state = 'starting';
    this.sinkTimer = setInterval(() => { this.sink.tick(performance.now()); }, 10);
    try {
      const ctx = new AudioContext({ sampleRate: this.opts.sampleRate, latencyHint: 'interactive' });
      this.ctx = ctx;
      ctx.onstatechange = () => {
        if (ctx.state === 'running') { this.startedAt ??= performance.now(); this.state = this.node ? 'running' : this.state; }
        else if (this.state === 'running') this.state = 'suspended';
        this.latency = { base: ctx.baseLatency, output: ctx.outputLatency };
        this.changed();
      };
      this.changed();
      void this.resume('start');
      await ctx.audioWorklet.addModule(this.opts.workletUrl);
      const node = new AudioWorkletNode(ctx, 'fle-out', { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2], processorOptions: { sab: this.sab, cap: this.opts.capFrames, dataOffset: DATA_OFFSET } });
      node.port.onmessage = (ev: MessageEvent<Omit<WorkletSample, 'page'> & { capture?: Float32Array; valid?: boolean; firstFrame?: number; validFrames?: number }>) => {
        const m = ev.data;
        if (m.capture) this.captures.push({ frames: m.capture, valid: m.valid === true, firstFrame: m.firstFrame ?? 0, validFrames: m.validFrames ?? 0, page: performance.now() });
        else this.onSample(m);
      };
      node.connect(ctx.destination);
      this.node = node;
      if (ctx.state === 'running') this.startedAt ??= performance.now();
      this.latency = { base: ctx.baseLatency, output: ctx.outputLatency };
      this.state = ctx.state === 'running' ? 'running' : 'suspended';
      this.changed();
    } catch (e) {
      this.state = 'error';
      this.error = String((e as Error)?.stack ?? e);
      this.changed();
    }
  }

  /** asks the worklet to copy `frames` frames starting at the first non-zero output sample (once); the result arrives in `captures`. `timeline` (decision 0211): the frames the engine had not delivered are copied as silence, so the copy keeps the output's time axis through under-runs (valid is still false then) */
  armCapture(frames: number, timeline = false): void { this.node?.port.postMessage({ cmd: 'capture', frames, ...(timeline ? { timeline: true } : {}) }); }

  /** a user gesture (pointerdown, keydown, ...): resume() when the browser started the context suspended. Returns when resume() settled or RESUME_WAIT_MS passed; the outcome is in lastResume (and in the beacon). */
  async resume(trigger = 'gesture'): Promise<ResumeRecord | null> {
    if (trigger !== 'start') this.gestures++;
    const ctx = this.ctx;
    if (!ctx || ctx.state === 'running') return null;
    const at = performance.now();
    this.resumeAttempts++;
    let result: ResumeRecord['result'] = 'pending';
    let error: string | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const outcome = await Promise.race([ctx.resume().then(() => 'settled' as const), new Promise<'wait'>((res) => { timer = setTimeout(() => { res('wait'); }, RESUME_WAIT_MS); })]);
      result = outcome === 'wait' ? 'pending' : (ctx.state as string) === 'running' ? 'running' : 'suspended';
    } catch (e) {
      result = 'rejected';
      error = String((e as Error)?.name ?? e).slice(0, 80);
    } finally { if (timer !== undefined) clearTimeout(timer); }
    const rec: ResumeRecord = { at, trigger, result, ms: Math.round((performance.now() - at) * 10) / 10, stateAfter: ctx.state, ...(error ? { error } : {}) };
    this.lastResume = rec;
    this.changed();
    return rec;
  }

  private onSample(s: Omit<WorkletSample, 'page'>): void {
    if (s.peak > this.peak) this.peak = s.peak;
    if (s.nonSilent > 0 && !this.marks['first-sound']) this.mark('first-sound');
    this.sumSq += s.sumSq;
    this.samplesN += s.n;
    if (this.samples.length < this.maxSamplesKept) this.samples.push({ ...s, page: performance.now() });
    const c = this.counters();
    this.history.push({ page: performance.now(), playing: c.playing, ur: c.urQuanta });
    if (this.history.length > 20000) this.history.splice(0, this.history.length - 20000);
  }

  counters(): RingCounters { return readCounters(this.header); }

  mark(name: string): void { this.marks[name] = { page: performance.now(), c: this.counters() }; }

  /** the numbers the page shows and the beacon sends */
  underruns(): UnderrunStats {
    const c = this.counters();
    return new UnderrunStats(c, { peak: this.peak, rms: this.samplesN ? Math.sqrt(this.sumSq / (this.samplesN * 2)) : 0 });
  }

  /** starts a new measurement window of the margin: FILL_MIN / FILL_MAX of the ring are reset (the worklet's posts carry the margin per 50 ms anyway) */
  resetFill(): void { Atomics.store(this.header, W.FILL_MIN, 0x7fffffff); Atomics.store(this.header, W.FILL_MAX, 0); }

  /** the numbers behind the P3 row (decision 0175, S-ur-window): counted per state of the device, since the engine's first idle, and over the last minute; the guest's transport state is not visible to the page, so "playing" here is the DirectSound device, which the engine starts at its own start-up */
  detail(now: number = performance.now()): AudioDetail {
    const u = this.underruns();
    const c = u.c;
    const first = this.marks['first-idle']?.c;
    const sound = this.marks['first-sound']?.c;
    const winMs = 60000;
    let base = this.history.find((h) => h.page >= now - winMs) ?? null;
    if (!base) base = this.history[0] ?? null;
    let lastUr = 0;
    for (let i = this.history.length - 1; i > 0; i--) { const cur = this.history[i], prev = this.history[i - 1]; if (cur && prev && cur.ur > prev.ur) { lastUr = cur.page; break; } }
    const ctx = this.ctx;
    const ms = (x: number | undefined): number | null => (typeof x === 'number' && Number.isFinite(x) ? Math.round(x * 10000) / 10 : null);
    return {
      status: this.statusKind(now), nodeReady: this.node !== null,
      ctxState: ctx ? ctx.state : null, sampleRate: ctx ? ctx.sampleRate : null, baseLatencyMs: ctx ? ms(ctx.baseLatency) : null, outputLatencyMs: ctx ? ms(ctx.outputLatency) : null,
      autoplayPolicy: autoplayPolicyOf(ctx),
      gestures: this.gestures, resumeAttempts: this.resumeAttempts, lastResume: this.lastResume ? { ...this.lastResume, at: Math.round(this.lastResume.at) } : null,
      softSinkFrames: this.sink.frames, capFrames: this.opts.capFrames, error: this.error === null ? null : this.error.slice(0, 200),
      ...this.engineLoss(now),
      deviceState: c.state === 2 ? 'playing' : c.state === 1 ? 'open' : 'closed',
      quantaWhileOpen: c.quanta - c.playing, quantaWhilePlaying: c.playing, playingSeconds: u.playingSeconds,
      underrunQuanta: c.urQuanta, underrunQuantaSinceFirstIdle: first ? c.urQuanta - first.urQuanta : null, underrunQuantaSinceFirstSound: sound ? c.urQuanta - sound.urQuanta : null, underrunQuantaLast60s: base ? c.urQuanta - base.ur : null,
      episodes: c.urRuns, longestRunQuanta: c.urMaxRun, secondsSinceLastUnderrun: lastUr > 0 ? (now - lastUr) / 1000 : null,
      nonSilentQuanta: c.nonSilent, peak: u.level.peak, rms: u.level.rms, fillFrames: this.fill(), fillMinFrames: c.fillMin,
    };
  }

  private engineLoss(now: number): { engineGapFrames: number | null; engineSkippedFrames: number | null; engineGapFramesLast60s: number | null; engineLateWrites: number | null; engineLateWriteApplied: number | null; engineLateWriteLost: number | null } {
    const last = this.reports[this.reports.length - 1];
    if (!last) return { engineGapFrames: null, engineSkippedFrames: null, engineGapFramesLast60s: null, engineLateWrites: null, engineLateWriteApplied: null, engineLateWriteLost: null };
    let base = this.reports[0] as WorkerReport;
    for (let i = this.reports.length - 1; i >= 0; i--) { const r = this.reports[i] as WorkerReport; if (r.page >= now - 60000) base = r; else break; }
    return { engineGapFrames: last.gapFrames, engineSkippedFrames: last.skipped, engineGapFramesLast60s: last.gapFrames - base.gapFrames, engineLateWrites: last.rewrite?.events ?? null, engineLateWriteApplied: last.rewrite?.applied ?? null, engineLateWriteLost: last.rewrite?.lost ?? null };
  }

  /** frames the ring holds that the engine published and the worklet has not read (negative: the engine is behind the output timeline) */
  fill(): number { return (Atomics.load(this.header, W.WR) - Atomics.load(this.header, W.RD)) | 0; }

  close(): void { if (this.sinkTimer !== null) { clearInterval(this.sinkTimer); this.sinkTimer = null; } void this.ctx?.close(); }
}
