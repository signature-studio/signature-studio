// The start's download modal (Issue #1520): an HTML box in the middle of the window, on the black canvas, that shows the start-up's downloads as progress bars. It is in the page
// from the first paint and is removed from the DOM (not hidden) just before the engine starts; after close() nothing of it is left and an update does nothing.
import type { Progress } from '../bundle/loader.ts';
import type { ProgressView } from './progress.ts';

export class LoadingModal {
  private readonly root: HTMLElement;
  private readonly rows: ProgressView;
  private isOpen = true;
  constructor(root: HTMLElement, rows: ProgressView) {
    this.root = root;
    this.rows = rows;
  }

  get open(): boolean { return this.isOpen; }

  update(p: Progress, note = ''): void {
    if (this.isOpen) this.rows.update(p, note);
  }

  /** the bars' values (0 to 1) by label */
  values(): { label: string; value: number }[] { return this.rows.values(); }

  /** removes the modal from the DOM; once it is closed, it stays closed (an error or the start may both ask) */
  close(): void {
    if (!this.isOpen) return;
    this.isOpen = false;
    this.root.remove();
  }
}
