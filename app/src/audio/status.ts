// The small audio status the page shows in its header (decision 0211): the owner opens the page without parameters, so a silent page has to say why. Text for the user; the same words go to the beacon's
// audio block as `status` (the kind), so a log line tells "suspended, waiting for a gesture" from "the browser refuses" from "running".
import type { AudioDetail, AudioStatusKind, PageAudio } from './page-audio.ts';

export type AudioLineKind = AudioStatusKind | 'unavailable';
export interface AudioLine { kind: AudioLineKind; text: string; title: string }

/** `off` is the reason the page has no audio output (install.ts audioOffReason), or null */
export function audioLine(off: string | null, d: AudioDetail | null): AudioLine {
  if (off !== null) {
    return { kind: 'unavailable', text: `音: 使えません (${off})`, title: `This browser cannot run the audio output: ${off}` };
  }
  if (d === null) return { kind: 'starting', text: '音: 準備中', title: 'The audio output is starting.' };
  const khz = d.sampleRate === null ? '' : ` ${Math.round(d.sampleRate / 100) / 10} kHz`;
  switch (d.status) {
    case 'running': {
      const lat = d.outputLatencyMs !== null && d.outputLatencyMs > 0 ? ` · 出力遅延 ${Math.round(d.outputLatencyMs)} ms` : '';
      const ur = d.underrunQuantaLast60s ?? 0;
      const cut = ur > 0 ? ` · 直近 60 秒の途切れ ${ur}` : '';
      return { kind: 'running', text: `音: 動作中${khz}${lat}${cut}`, title: `The audio output runs${khz}. The engine's frames are played as they arrive; a gap shows up as silence and is counted (${d.underrunQuanta} quanta so far).` };
    }
    case 'needs-gesture':
      return { kind: 'needs-gesture', text: '音: 停止中 (クリックまたはキー入力で有効になります)', title: 'The browser keeps the audio output stopped until you click or press a key on this page.' };
    case 'resuming':
      return { kind: 'resuming', text: '音: 開始中…', title: 'Asking the browser to start the audio output.' };
    case 'blocked':
      return { kind: 'blocked', text: '音: ブラウザが開始を許可していません (サイトの自動再生の設定を確認してください)', title: `resume() ${d.lastResume?.result ?? '?'} after ${d.gestures} gesture(s); context state ${d.ctxState ?? '?'}${d.autoplayPolicy ? `; autoplay policy ${d.autoplayPolicy}` : ''}. The song still advances, silently.` };
    case 'error':
      return { kind: 'error', text: '音: エラー (詳細は Page log)', title: (d.error ?? 'the audio output failed').slice(0, 300) };
    case 'closed':
      return { kind: 'closed', text: '音: 停止しました', title: 'The audio context was closed.' };
    case 'idle':
      return { kind: 'idle', text: '音: エンジンの起動待ち', title: 'The audio output starts together with the engine.' };
    case 'starting':
      return { kind: 'starting', text: '音: 準備中', title: 'The audio output is starting.' };
  }
}

/** keeps an element up to date: when the audio object tells of a change, and once a second (the "blocked" verdict and the underrun count are read, not announced) */
export function bindAudioStatus(el: HTMLElement | null, off: string | null, audio: PageAudio | null, setTimer: (fn: () => void, ms: number) => unknown = (fn, ms) => setInterval(fn, ms)): () => AudioLine {
  const compute = (): AudioLine => audioLine(off, audio ? audio.detail() : null);
  const render = (): void => {
    if (!el) return;
    const l = compute();
    if (el.textContent !== l.text) el.textContent = l.text;
    if (el.dataset.state !== l.kind) el.dataset.state = l.kind;
    if (el.title !== l.title) el.title = l.title;
  };
  render();
  if (audio) { audio.onChange(render); setTimer(render, 1000); }
  return compute;
}
