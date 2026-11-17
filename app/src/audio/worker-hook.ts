// The engine Worker's side of the audio output (decision 0175). The page sends { cmd: 'audio-init', sab, cap, dataOffset } BEFORE the engine starts; the engine's own JavaScript (EM_JS in
// lift/runtime/fle_audio_ring.c, decision 0139) reads it as globalThis.fleAudioSab at its first audio call. Without the message nothing changes: the engine has no external consumer and uses its virtual clock.
// The same listener posts, every `reportMs`, the engine's own audio counters (fleAudioWorkerState / fleAudioPace of that EM_JS) with the Unlock times since the last post. It does not touch the engine's idle
// protocol: it only reads, from timer turns of the Worker's event loop. Imported by worker.ts for its side effect (one line).
interface WorkerScope {
  fleAudioSab?: { sab: SharedArrayBuffer; cap: number; dataOffset: number };
  fleAudioWorkerState?: () => { unlocks: { first: number; n: number; t: number }[]; waits: number; publishes: number; skipped: number; gapFrames: number; maxWaitDrift: number };
  fleAudioPace?: () => Record<string, number>;
  fleAudioRewrite?: () => Record<string, number>;
  postMessage(m: unknown): void;
  addEventListener?(type: 'message', fn: (ev: MessageEvent) => void): void;
}
const scope = globalThis as unknown as WorkerScope;

export interface AudioInitMessage { cmd: 'audio-init'; sab: SharedArrayBuffer; cap: number; dataOffset: number; reportMs?: number }

let timer: ReturnType<typeof setInterval> | null = null;
let sentUnlocks = 0;

export function onAudioMessage(m: unknown, s: WorkerScope = scope): boolean {
  const a = m as Partial<AudioInitMessage> | null;
  if (!a || a.cmd !== 'audio-init' || !(a.sab instanceof SharedArrayBuffer)) return false;
  s.fleAudioSab = { sab: a.sab, cap: a.cap as number, dataOffset: a.dataOffset as number };
  if (timer === null && (a.reportMs ?? 0) > 0) timer = setInterval(() => report(s), a.reportMs);
  return true;
}

/** decision 0222 (optional): the engine returned (FL's main form was closed): its exports must not be called any more (the runtime aborts a call after exit), so the report ends with it */
export function stopAudioReport(): void {
  if (timer !== null) { clearInterval(timer); timer = null; }
}

function report(s: WorkerScope): void {
  const st = s.fleAudioWorkerState?.();
  if (!st) return;
  const fresh = st.unlocks.slice(sentUnlocks);
  sentUnlocks = st.unlocks.length;
  s.postMessage({ type: 'aud2-worker', at: performance.timeOrigin + performance.now(), unlocks: fresh, waits: st.waits, publishes: st.publishes, skipped: st.skipped, gapFrames: st.gapFrames, maxWaitDrift: st.maxWaitDrift, rewrite: s.fleAudioRewrite?.() ?? null, pace: s.fleAudioPace?.() ?? null });
}

scope.addEventListener?.('message', (ev: MessageEvent) => { onAudioMessage(ev.data); });
