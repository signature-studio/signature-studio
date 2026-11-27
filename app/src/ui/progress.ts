// Download progress rows: bytes, share, speed (MiB/s over the time since the asset's first report). One row per label. The rows of the start-up's assets are made up front
// (Issue #1520), so the modal shows all of them at 0 % before the first byte; each bar is a progressbar whose value is the share of the asset (0 to 1).
import type { Progress } from '../bundle/loader.ts';
import { transferSpeed } from './phases.ts';

export const formatSize = (n: number): string => (n >= 1048576 ? `${(n / 1048576).toFixed(n >= 10 * 1048576 ? 0 : 1)} MiB` : `${(n / 1024).toFixed(1)} KiB`);

/** the share of an asset that has arrived, 0 to 1 (a finished asset is 1; an asset of no known size is 0 until it is finished) */
export function fractionOf(p: Pick<Progress, 'loaded' | 'total' | 'done'>): number {
  if (p.done) return 1;
  if (!(p.total > 0)) return 0;
  return Math.min(1, Math.max(0, p.loaded / p.total));
}

export class ProgressView {
  private readonly rows = new Map<string, { bar: HTMLProgressElement; text: HTMLSpanElement; first: { loaded: number; at: number } | undefined }>();
  private readonly list: HTMLElement;
  constructor(list: HTMLElement, labels: readonly string[] = []) {
    this.list = list;
    for (const label of labels) this.row(label);
  }

  // a row the page already has (the start-up's four are in index.html, so they are there before the script runs) is used; else a row is made
  private row(label: string): { bar: HTMLProgressElement; text: HTMLSpanElement; first: { loaded: number; at: number } | undefined } {
    let r = this.rows.get(label);
    if (r) return r;
    let li = Array.from(this.list.children).find((c) => (c as HTMLElement).dataset.label === label) as HTMLElement | undefined;
    if (!li) {
      li = document.createElement('li');
      li.dataset.label = label;
      li.append(document.createElement('span'), document.createElement('progress'));
      this.list.append(li);
    }
    const text = li.children[0] as HTMLSpanElement;
    const bar = li.children[1] as HTMLProgressElement;
    bar.setAttribute('role', 'progressbar');
    bar.setAttribute('aria-label', label);
    bar.max = 1;
    bar.value = 0;
    r = { bar, text, first: undefined };
    this.rows.set(label, r);
    return r;
  }

  update(p: Progress, note = ''): void {
    const r = this.row(p.label);
    r.first ??= { loaded: p.loaded, at: p.at };
    const speed = transferSpeed(r.first, { loaded: p.loaded, at: p.at });
    r.bar.max = 1;
    r.bar.value = fractionOf(p);
    r.text.textContent = `${p.label}: ${formatSize(p.loaded)} / ${formatSize(p.total)}${speed !== null ? ` · ${speed.toFixed(0)} MiB/s` : ''}${p.done ? ' (verified)' : ''}${note}`;
  }

  /** the bars' values (0 to 1) by label, in the order the rows were made */
  values(): { label: string; value: number }[] {
    return [...this.rows].map(([label, r]) => ({ label, value: r.bar.value }));
  }
}
