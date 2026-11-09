// The page's state when the Worker reports the first pixel. The first pixel and the first idle both come from the Worker, and the page does not control their order: with display method D the first picture is
// drawn by the first present of the desktop, which can run after the engine's first idle message (measured with Firefox 158 under load: idle at 9.5 s, first pixel at 9.9 s). The first pixel must not take the page
// back from 'engine-idle' to 'running': nothing would ever set 'engine-idle' again, and the page would show 'running' for an engine that waits for input.
export function stateAfterFirstPixel(current: string): string {
  return current === 'loaded' || current === 'running' ? 'running' : current;
}
