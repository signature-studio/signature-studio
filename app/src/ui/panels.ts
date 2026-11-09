// The collapsible panels: the guest's windows and threads (read from the engine's log) and the engine log itself.
import type { EngineThread, EngineWindow } from '../engine/fruity-log.ts';

export function renderWindows(tbody: HTMLElement, countEl: HTMLElement, windows: readonly EngineWindow[]): void {
  countEl.textContent = String(windows.length);
  const frag = document.createDocumentFragment();
  for (const w of windows) {
    const tr = document.createElement('tr');
    tr.dataset.lid = String(w.lid);
    tr.dataset.visible = w.visible ? '1' : '0';
    for (const text of [String(w.lid), w.className, w.title, `${w.x}, ${w.y}`, `${w.width} x ${w.height}`, w.visible ? 'yes' : 'no', w.enabled ? 'yes' : 'no', String(w.z)]) {
      const td = document.createElement('td');
      td.textContent = text;
      tr.append(td);
    }
    frag.append(tr);
  }
  tbody.replaceChildren(frag);
}

export function renderThreads(pre: HTMLElement, countEl: HTMLElement, summary: string, threads: readonly EngineThread[]): void {
  countEl.textContent = String(threads.length);
  pre.textContent = [summary, ...threads.map((t) => `thread ${t.index}  id ${t.id}  ${t.state}`)].filter((l) => l !== '').join('\n');
}

/** the engine's printed lines: all of them are kept for the summary, the panel shows the last `shown` */
export class EngineLogView {
  readonly lines: string[] = [];
  private readonly pre: HTMLElement;
  private readonly count: HTMLElement;
  private readonly shown: number;
  private readonly keep: number;
  constructor(pre: HTMLElement, count: HTMLElement, shown = 1500, keep = 50000) { this.pre = pre; this.count = count; this.shown = shown; this.keep = keep; }

  add(stream: 'out' | 'err', lines: readonly string[]): void {
    for (const l of lines) this.lines.push(stream === 'err' ? `[err] ${l}` : l);
    if (this.lines.length > this.keep) this.lines.splice(0, this.lines.length - this.keep);
    this.count.textContent = String(this.lines.length);
    this.pre.textContent = this.lines.slice(-this.shown).join('\n');
  }
}
