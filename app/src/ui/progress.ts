// Download progress rows: bytes, share, speed (MiB/s over the time since the asset's first report). One row per label.
import type { Progress } from '../bundle/loader.ts';
import { transferSpeed } from './phases.ts';

export const formatSize = (n: number): string => (n >= 1048576 ? `${(n / 1048576).toFixed(n >= 10 * 1048576 ? 0 : 1)} MiB` : `${(n / 1024).toFixed(1)} KiB`);

export class ProgressView {
  private readonly rows = new Map<string, { bar: HTMLProgressElement; text: HTMLSpanElement; first: { loaded: number; at: number } | undefined }>();
  private readonly list: HTMLElement;
  constructor(list: HTMLElement) { this.list = list; }

  update(p: Progress, note = ''): void {
    let r = this.rows.get(p.label);
    if (!r) {
      const li = document.createElement('li');
      li.dataset.label = p.label;
      const text = document.createElement('span');
      const bar = document.createElement('progress');
      bar.max = 1;
      bar.value = 0;
      li.append(text, bar);
      this.list.append(li);
      r = { bar, text, first: undefined };
      this.rows.set(p.label, r);
    }
    r.first ??= { loaded: p.loaded, at: p.at };
    const speed = transferSpeed(r.first, { loaded: p.loaded, at: p.at });
    r.bar.max = Math.max(1, p.total);
    r.bar.value = p.loaded;
    r.text.textContent = `${p.label}: ${formatSize(p.loaded)} / ${formatSize(p.total)}${speed !== null ? ` · ${speed.toFixed(0)} MiB/s` : ''}${p.done ? ' (verified)' : ''}${note}`;
  }
}
