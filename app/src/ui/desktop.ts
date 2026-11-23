// The page's desktop (decision 0141, reduced by decisions 0223 and opt1/0243): the engine is shown on ONE canvas (deskCanvas()): its windows are composed by the runtime (lift/runtime/fle_desk.c), the engine
// Worker shows the picture on that canvas, and the page knows no window. DesktopView sizes the box and the canvas and answers the geometry the input code needs.
import { SCREEN } from '../input/events.ts';
import type { ViewRect } from '../input/tracker.ts';

export class DesktopView {
  readonly root: HTMLElement;
  private scale = 1;
  /** decision 0212: the size of the guest's screen in device pixels (1920 x 1080 until the page or the engine says another); the canvas has these pixels and is shown at `scale` CSS px per pixel (C-1, #1347: 1 / devicePixelRatio, the CSS size is fractional so that it is whole device pixels) */
  private screen: { width: number; height: number } = { width: SCREEN.width, height: SCREEN.height };
  private desk: HTMLCanvasElement | null = null;
  private onLayout: (() => void) | null = null;
  constructor(root: HTMLElement) {
    this.root = root;
  }

  onLayoutChange(fn: () => void): void { this.onLayout = fn; }

  /** decision 0223: the ONE canvas of the real engine's desktop (the engine Worker shows the runtime's picture on it; the page draws nothing). It is made at the first call and fills the screen. */
  deskCanvas(): HTMLCanvasElement {
    if (this.desk) return this.desk;
    const canvas = document.createElement('canvas');
    canvas.width = this.screen.width;
    canvas.height = this.screen.height;
    canvas.dataset.desk = '1';
    canvas.style.position = 'absolute';
    canvas.style.left = '0';
    canvas.style.top = '0';
    this.root.append(canvas);
    this.desk = canvas;
    this.apply();
    return canvas;
  }

  setScale(scale: number): void { if (scale === this.scale) return; this.scale = scale; this.apply(); }
  get currentScale(): number { return this.scale; }
  setCursor(css: string): void { this.root.style.cursor = css; }

  setScreen(width: number, height: number): void { if (width === this.screen.width && height === this.screen.height) return; this.screen = { width, height }; this.apply(); }
  get screenSize(): { width: number; height: number } { return this.screen; }
  /** the CSS size of the screen: not rounded (#1347). At 1 / devicePixelRatio it is whole device pixels; rounding it to whole CSS pixels would make the canvas differ from the device pixels. */
  get screenCssSize(): { width: number; height: number } { return { width: this.screen.width * this.scale, height: this.screen.height * this.scale }; }
  rect(): ViewRect { const r = this.root.getBoundingClientRect(); return { left: r.left, top: r.top, width: r.width, height: r.height }; }

  private apply(): void {
    const size = this.screenCssSize;
    this.root.style.width = `${size.width}px`;
    this.root.style.height = `${size.height}px`;
    if (this.desk) {
      this.desk.style.width = `${size.width}px`;
      this.desk.style.height = `${size.height}px`;
    }
    this.onLayout?.();
  }
}
