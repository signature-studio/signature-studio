// The Worker's side of the engine's idle (decisions 0128 and 0136, wall-clock mode; decision 0141). While the engine waits for input it is suspended in the async import fle_idle_wait (lift/harness/web/fruity/
// fle_idle_lib.js) and calls Module.fleIdle(n, nowTicks, nextTicks); the Worker's event loop then runs, and this class decides what the engine does next:
//   - input events waiting (queued while the engine was busy) are given to the engine's input queue (fle_g3_mouse / fle_g3_key), the virtual clock first moved by the wall time that passed since the idle began
//     (never past the engine's next deadline), and the engine is resumed with code 0;
//   - otherwise the engine's next deadline is paced against the wall clock (S-idle-clock-wall: virtual time runs at `rate` virtual ms per wall ms, and the clock is moved to the deadline exactly):
//       (decision 0170) one mapping virtual <-> wall (the anchor, made at the first idle). A deadline that the wall clock has already reached is delivered at once, in the same turn of the event
//         loop (the engine's next idle comes back through onIdle within microtasks), for at most `turnBudgetMs` of wall time; then the driver gives the event loop a turn (host.yieldTurn: a MessageChannel, not a timer, whose
//         nesting clamp of about 4 ms made one virtual ms cost 4 wall ms) and goes on. A deadline in the future is waited for with one timer. Virtual time is never skipped: only WHEN a deadline is delivered depends on the wall clock.
//         The lag (wall target minus the virtual clock) is capped at `maxLagMs`: a larger lag (a long freeze, a frozen tab) is cut by moving the anchor, so the virtual clock simply lags. A lead (the engine's own compute moved
//         its clock ahead of the wall clock, decision 0147's slices) is kept up to `maxLeadMs`, so the long-run speed is 1.000; a larger one is cut the same way, so a wait never exceeds the virtual distance to the deadline by more than that.
//     the engine is then resumed with that deadline in ticks (> 0 = "move the clock there");
//   - an input that arrives while the engine is idle is given at once and cancels the timer.
// Input that arrives before the first idle is DROPPED and counted (S-input-before-idle: what the user pointed at was the start-up picture, not the screen the click would reach a minute later).
// Input that arrives while the engine is busy (the Worker's event loop turns at thread switches too) is queued, consecutive moves merged (S-move-merge, like the engine's own S-mousemove-coalesce), at most `maxQueue`.
// Nothing here touches the DOM or the wasm module: the engine's exports and the clock are passed in, so the state machine is tested without a browser (test/unit/idle-driver.test.ts).
import { IN, type RawInput } from '../input/events.ts';

export const TICKS_PER_MS = 10000;

export interface EngineInput {
  mouse(kind: number, x: number, y: number, data: number): number;
  key(kind: number, vk: number, scan: number, ext: number): number;
  /** decision 0212: the display changed to width x height (fle_g3_display); absent in a build that has no such export */
  display?(width: number, height: number): number;
  /** decision 0272: the taskbar button of window `lid` was pressed (fle_g3_taskbar_syscommand); absent in a build that has no such export */
  syscommand?(lid: number, cmd: number): number;
  clockTo(ticks: number): void;
}
export interface DriverHost {
  /** resumes the engine; false when it was not waiting */
  deliver(code: number): boolean;
  /** milliseconds on one clock (the Worker passes performance.timeOrigin + performance.now()) */
  now(): number;
  setTimer(fn: () => void, ms: number): unknown;
  clearTimer(handle: unknown): void;
  /** runs `fn` in a LATER task of the event loop without the nested-timer clamp (the Worker: a MessageChannel). Absent: setTimer(fn, 0) */
  yieldTurn?(fn: () => void): void;
}
export interface Injected { seq: number; tPage: number; tInject: number; ev: RawInput }
export interface DriverHooks {
  /** the events handed to the engine at one idle (before it is resumed) */
  injected?(list: Injected[]): void;
  /** the engine is idle again after having been given `list`: it processed them (tIdle) */
  processed?(list: Injected[], tIdle: number): void;
  dropped?(seq: number, reason: string): void;
  idle?(n: number, nowTicks: number, nextTicks: number, busyMs: number | null, at: number): void;
  resumed?(code: number, at: number, why: 'input' | 'deadline' | 'end'): void;
  /** the lag reached maxLagMs and `ticks` of it were cut (the virtual clock stays that far behind the wall clock for good) */
  lagCut?(ticks: number): void;
}
export interface DriverStats {
  injected: number; rejected: number; dropped: number; droppedBeforeIdle: number; merged: number; overflow: number; idleCount: number;
  /** deadlines delivered (any way) */
  deadlineWakes: number; clockMovedTicks: number;
  /** deadlines delivered in the same turn as the idle, after a bounce of the event loop, and after a timer */
  chained: number; bounces: number; timerWakes: number;
  /** times the lag was cut at maxLagMs (and the virtual ms cut in ticks), times a lead was cut, the largest lag seen at an idle (ticks) */
  lagCuts: number; lagCutTicks: number; leadCuts: number; maxLagTicks: number; maxLeadTicks: number;
}
export const newDriverStats = (): DriverStats => ({ injected: 0, rejected: 0, dropped: 0, droppedBeforeIdle: 0, merged: 0, overflow: 0, idleCount: 0, deadlineWakes: 0, clockMovedTicks: 0, chained: 0, bounces: 0, timerWakes: 0, lagCuts: 0, lagCutTicks: 0, leadCuts: 0, maxLagTicks: 0, maxLeadTicks: 0 });
export interface DriverOptions {
  /** virtual ms per wall ms while the engine is idle (default 1) */
  rate?: number;
  maxQueue?: number;
  /** S-clk-maxlag: the largest lag (wall target minus virtual clock) kept, ms (default 1000) */
  maxLagMs?: number;
  /** S-clk-maxlead: the largest lead (virtual clock minus wall target) kept, ms (default 100); a wait for a deadline is stretched by the lead */
  maxLeadMs?: number;
  /** S-clk-budget: the wall time one turn of the event loop may spend delivering deadlines in a row before it gives the loop a turn, ms (default 4); 0 = one deadline per turn */
  turnBudgetMs?: number;
}

export class IdleDriver {
  readonly stats: DriverStats = newDriverStats();
  private idle: { n: number; now: number; next: number; wall: number } | null = null;
  private timer: unknown = null;
  private started = false;
  private lastResumeAt: number | null = null;
  private readonly queue: { ev: RawInput; seq: number; tPage: number }[] = [];
  private awaiting: Injected[] = [];
  private readonly engine: EngineInput;
  private readonly host: DriverHost;
  private readonly hooks: DriverHooks;
  private readonly rate: number;
  private readonly maxQueue: number;
  private readonly maxLagTicks: number;
  private readonly maxLeadTicks: number;
  private readonly turnBudgetMs: number;
  /** virtual ticks `ticks` correspond to wall instant `wall` (pace 'catchup'); made at the first idle, moved when a lag or lead is cut */
  private anchor: { wall: number; ticks: number } | null = null;
  /** the wall instant at which the current turn of the event loop began (a timer, a bounce, an input, end()) */
  private turnStart = 0;

  constructor(engine: EngineInput, host: DriverHost, hooks: DriverHooks = {}, opts: DriverOptions = {}) {
    this.engine = engine;
    this.host = host;
    this.hooks = hooks;
    this.rate = opts.rate ?? 1;
    this.maxQueue = opts.maxQueue ?? 256;
    this.maxLagTicks = (opts.maxLagMs ?? 1000) * TICKS_PER_MS;
    this.maxLeadTicks = (opts.maxLeadMs ?? 100) * TICKS_PER_MS;
    this.turnBudgetMs = opts.turnBudgetMs ?? 4;
  }

  get ready(): boolean { return this.started; }
  get isIdle(): boolean { return this.idle !== null; }
  get queued(): number { return this.queue.length; }
  /** input events the engine has been given and has not yet finished with (decision 0182: the idle that ends them is the one at which the window list must be read) */
  get awaitingCount(): number { return this.awaiting.length; }

  /** a raw event from the page */
  submit(ev: RawInput, seq: number, tPage: number): 'dropped' | 'applied' | 'queued' {
    if (!this.started) {
      this.stats.dropped++;
      this.stats.droppedBeforeIdle++;
      this.hooks.dropped?.(seq, 'the engine has not reached its first idle: input is dropped');
      return 'dropped';
    }
    if (this.idle) { this.turnStart = this.host.now(); this.apply([{ ev, seq, tPage }]); return 'applied'; }
    const last = this.queue[this.queue.length - 1];
    if (last && ev.k === 'mouse' && ev.kind === IN.MOVE && last.ev.k === 'mouse' && last.ev.kind === IN.MOVE) {
      this.hooks.dropped?.(last.seq, 'merged into a later move');
      this.stats.merged++;
      this.queue[this.queue.length - 1] = { ev, seq, tPage };
      return 'queued';
    }
    if (this.queue.length >= this.maxQueue) {
      this.stats.dropped++;
      this.stats.overflow++;
      this.hooks.dropped?.(seq, `the input queue is full (${this.maxQueue})`);
      return 'dropped';
    }
    this.queue.push({ ev, seq, tPage });
    return 'queued';
  }

  /**
   * Decision 0280 (PROVISIONAL): inputs that the RUNTIME took from the ring itself, at one of its scheduler points while the engine was running (lift/harness/fruity_idle.c fle_input_ring_poll), not at an idle.
   * They are in the engine's input queue already (fle_input_host_*); this only keeps the books the idle path keeps in apply(): the count, and the list of inputs that await their answer (the present that shows
   * what the guest drew for them). The virtual clock is not moved (it is the engine's own: the engine is running, it is not waiting); an event is stamped with the virtual time of the moment it is queued.
   */
  injectedInline(done: Injected[]): void {
    this.stats.injected += done.length;
    this.awaiting.push(...done);
    this.hooks.injected?.(done);
  }

  /** the inputs injected inline since the last answer (the caller answers them with a present; cleared) */
  takeAwaiting(): Injected[] { const l = this.awaiting; this.awaiting = []; return l; }

  /** Module.fleIdle(n, nowTicks, nextTicks): the engine is waiting */
  onIdle(n: number, nowTicks: number, nextTicks: number): void {
    const at = this.host.now();
    this.started = true;
    this.stats.idleCount++;
    this.idle = { n, now: nowTicks, next: nextTicks, wall: at };
    if (!this.anchor) this.anchor = { wall: at, ticks: nowTicks };
    if (this.awaiting.length) { this.hooks.processed?.(this.awaiting, at); this.awaiting = []; }
    this.hooks.idle?.(n, nowTicks, nextTicks, this.lastResumeAt === null ? null : at - this.lastResumeAt, at);
    if (this.queue.length) { this.apply(this.queue.splice(0)); return; }
    this.arm();
  }

  /** ends the session (the engine's fle_idle_wait returns < 0) */
  end(): boolean { this.turnStart = this.host.now(); return this.resume(-1, 'end'); }

  /** where the wall clock says the virtual clock should be at wall instant `w`, kept within [now - maxLead, now + maxLag] (the anchor is moved when it is cut) */
  private target(w: number, now: number): number {
    const a = this.anchor as { wall: number; ticks: number };
    const raw = a.ticks + (w - a.wall) * TICKS_PER_MS * this.rate;
    let t = raw;
    if (raw > now + this.maxLagTicks) { t = now + this.maxLagTicks; this.stats.lagCuts++; this.stats.lagCutTicks += raw - t; this.hooks.lagCut?.(raw - t); }
    else if (raw < now - this.maxLeadTicks) { t = now - this.maxLeadTicks; this.stats.leadCuts++; }
    if (t !== raw) this.anchor = { wall: w, ticks: t };
    if (t - now > this.stats.maxLagTicks) this.stats.maxLagTicks = t - now;
    if (now - t > this.stats.maxLeadTicks) this.stats.maxLeadTicks = now - t;
    return t;
  }

  private arm(): void {
    const i = this.idle;
    if (!i || i.next <= 0) return;
    const n = i.n;
    const w = this.host.now();
    const t = this.target(w, i.now);
    if (i.next <= t) {
      // the wall clock has reached this deadline: deliver it now, in this turn while the budget lasts, else after a turn of the event loop
      if (w - this.turnStart < this.turnBudgetMs) { this.stats.chained++; this.stats.deadlineWakes++; this.resume(i.next, 'deadline'); return; }
      this.stats.bounces++;
      const go = (): void => {
        if (!this.idle || this.idle.n !== n) return;
        this.turnStart = this.host.now();
        this.stats.deadlineWakes++;
        this.resume(i.next, 'deadline');
      };
      if (this.host.yieldTurn) this.host.yieldTurn(go); else this.host.setTimer(go, 0);
      return;
    }
    const waitMs = (i.next - t) / TICKS_PER_MS / this.rate;
    this.timer = this.host.setTimer(() => {
      this.timer = null;
      if (!this.idle || this.idle.n !== n) return;
      this.turnStart = this.host.now();
      this.stats.deadlineWakes++;
      this.stats.timerWakes++;
      this.resume(i.next, 'deadline');
    }, waitMs);
  }

  private resume(code: number, why: 'input' | 'deadline' | 'end'): boolean {
    if (this.timer !== null) { this.host.clearTimer(this.timer); this.timer = null; }
    this.idle = null;
    const at = this.host.now();
    this.lastResumeAt = at;
    this.hooks.resumed?.(code, at, why);
    return this.host.deliver(code);
  }

  /** S-idle-clock-wall: an input moves the clock by the wall time since the idle began, never past the deadline */
  private moveClock(): void {
    const i = this.idle;
    if (!i) return;
    let t = this.anchor ? this.target(this.host.now(), i.now) : i.now + (this.host.now() - i.wall) * TICKS_PER_MS * this.rate;
    if (i.next > 0 && t > i.next) t = i.next;
    if (t > i.now) { this.engine.clockTo(t); this.stats.clockMovedTicks += t - i.now; }
  }

  private apply(list: { ev: RawInput; seq: number; tPage: number }[]): void {
    this.moveClock();
    const done: Injected[] = [];
    for (const it of list) {
      const e = it.ev;
      const r = e.k === 'mouse' ? this.engine.mouse(e.kind, e.x, e.y, e.data) : e.k === 'key' ? this.engine.key(e.kind, e.vk, e.scan, e.ext) : e.k === 'syscmd' ? (this.engine.syscommand ? this.engine.syscommand(e.lid, e.cmd) : 0) : this.engine.display ? this.engine.display(e.width, e.height) : 0;
      if (r === 1) { this.stats.injected++; done.push({ seq: it.seq, tPage: it.tPage, tInject: this.host.now(), ev: e }); }
      else { this.stats.rejected++; this.hooks.dropped?.(it.seq, 'the engine refused the event (its input queue is full)'); }
    }
    if (done.length === 0) { if (this.timer === null) this.arm(); return; }
    this.awaiting.push(...done);
    this.hooks.injected?.(done);
    this.resume(0, 'input');
  }
}
