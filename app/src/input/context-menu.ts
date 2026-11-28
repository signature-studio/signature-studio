// The browser's own context menu never shows on the page (Issue #1482). FL draws its menus itself: a right click is the engine's input (pointer.ts), and the browser's menu
// (with its "Paste" alone, which appears in an editable place) is not wanted anywhere. The cancel is in the capture phase of the document, so it runs before any listener
// of an element, whether the element is enabled or not. A frame (the Browser face, an iframe) has its own window: its events do not reach the page's document, so the
// frame's window gets the same cancel (see ui/webview2-surface.ts).

/** cancels the browser's context menu for every element under `target` (the document of the page, or the window of a frame) */
export function suppressContextMenu(target: Pick<EventTarget, 'addEventListener'>): void {
  target.addEventListener('contextmenu', (e) => { e.preventDefault(); }, true);
}
