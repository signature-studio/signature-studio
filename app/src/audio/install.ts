// Wiring of the audio output into the page: always on (decision 0211; decision 0243: no URL parameter at all, the ring and the rate are constants).
// The engine Worker is made by EngineClient (client.ts, unchanged): this wraps the Worker constructor for the Worker named 'fl-engine' only, so that the ring reaches it before its first message.
import { PageAudio } from './page-audio.ts';
// the worklet is a plain JavaScript file that Vite copies as an asset (new URL(..., import.meta.url)); addModule() loads it from this origin
const workletUrl = new URL('./fle-out.worklet.js', import.meta.url).href;

/** the ring and the output (decision 0243: constants, no URL parameter): the AudioContext runs at the engine's device rate (S-ur-rate), the ring holds the engine's own 32,768 frames (S-audio-ring) */
export const AUDIO = { sampleRate: 44100, capFrames: 32768 } as const;

let installFailure: string | null = null;

/** why the page has no audio output, or null when it has one: what the browser lacks (the ring is a SharedArrayBuffer, the output an AudioWorklet), or the error of the construction */
export function audioOffReason(win: Window & typeof globalThis = window): string | null {
  if (typeof win.SharedArrayBuffer === 'undefined') return 'no SharedArrayBuffer (the page is not cross-origin isolated)';
  if (typeof win.AudioContext === 'undefined') return 'no AudioContext';
  if (typeof win.AudioWorkletNode === 'undefined') return 'no AudioWorklet';
  return installFailure;
}

/** the events whose handlers the browsers count as a user activation (Chrome, Firefox, Safari differ in the list; resume() on one that does not count is harmless) */
export const GESTURE_EVENTS = ['pointerdown', 'pointerup', 'mousedown', 'click', 'keydown', 'keyup', 'touchend'] as const;

export function installAudio(win: Window & typeof globalThis = window): PageAudio | null {
  if (audioOffReason(win) !== null) return null;
  let audio: PageAudio;
  try { audio = new PageAudio({ sampleRate: AUDIO.sampleRate, capFrames: AUDIO.capFrames, workletUrl }); } catch (e) { installFailure = String((e as Error)?.message ?? e).slice(0, 200); return null; }
  const Orig = win.Worker;
  class AudioWorkerCtor extends Orig {
    constructor(url: string | URL, o?: WorkerOptions) {
      super(url, o);
      if (o?.name === 'fl-engine') {
        this.postMessage(audio.initMessage());
        audio.watch(this);
        // the AudioContext is made for a page that has an engine only (a page whose manifest failed has none), and after the client has gone on with the engine's start-up: the first AudioContext of a page costs the main thread tens of ms (76 ms measured, decision 0211)
        win.setTimeout(() => { void audio.start(); }, 0);
      }
    }
  }
  win.Worker = AudioWorkerCtor as unknown as typeof Worker;
  const gesture = (): void => { void audio.resume(); };
  for (const t of GESTURE_EVENTS) win.addEventListener(t, gesture, { capture: true, passive: true });
  return audio;
}
