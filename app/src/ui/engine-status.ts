// What the page says about the engine's state (decision 0141): waiting for input, computing (for how long), starting, stopped. Pure: the state is a function of the instants the page has seen, so the same code
// serves the screen and the tests. The engine does not tell the page "I am busy": it is busy from the moment it was resumed until it calls fle_idle_wait again (the Worker cannot post while the engine computes),
// so "computing for N s" is `now - the time of the last resume`.
export interface EngineClockState {
  /** the page asked the Worker to start the engine (ms, same clock as `now`) */
  startedAt: number | null;
  /** the first idle: the engine can take input from now on */
  firstIdleAt: number | null;
  lastIdleAt: number | null;
  lastResumeAt: number | null;
  /** how the engine ended (the page was told) */
  ended: { reason: 'finished' | 'stopped' | 'trap'; code: number } | null;
  error: boolean;
  /** durations (ms) between consecutive idle entries, newest last */
  cycles: readonly number[];
}

export type EngineState = 'not-started' | 'starting' | 'idle' | 'computing' | 'stopped' | 'error';
export interface EngineStateText { state: EngineState; text: string; inputAccepted: boolean }

const secs = (ms: number): string => `${(ms / 1000).toFixed(ms < 10000 ? 1 : 0)} s`;

export function engineState(c: EngineClockState, now: number): EngineStateText {
  if (c.error) return { state: 'error', text: 'Engine: failed (see the error above)', inputAccepted: false };
  if (c.ended) return { state: 'stopped', text: `Engine: stopped (${c.ended.reason}${c.ended.code >= 0 ? `, exit code ${c.ended.code}` : ''}) - it no longer takes input`, inputAccepted: false };
  if (c.startedAt === null) return { state: 'not-started', text: 'Engine: not started', inputAccepted: false };
  if (c.firstIdleAt === null) return { state: 'starting', text: `Engine: starting (${secs(now - c.startedAt)}) - input is ignored until its first idle`, inputAccepted: false };
  const idleNow = c.lastResumeAt === null || (c.lastIdleAt !== null && c.lastIdleAt >= c.lastResumeAt);
  if (idleNow) return { state: 'idle', text: 'Engine: waiting for input (idle)', inputAccepted: true };
  return { state: 'computing', text: `Engine: computing (${secs(now - (c.lastResumeAt as number))}) - input waits for its next idle`, inputAccepted: true };
}

/** mean length of the engine's idle-to-idle cycle (its frame time while it draws every tick), ms, or null */
export function meanCycle(cycles: readonly number[], last = 20): number | null {
  const v = cycles.slice(-last);
  return v.length === 0 ? null : v.reduce((a, b) => a + b, 0) / v.length;
}
