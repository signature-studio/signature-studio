// JS side of the engine's clipboard (#1389). An emscripten JS library, like harness/web/filedlg/fle_filedlg_lib.js: link with -sJSPI --js-library <this file>.
// fle_clip_host_write(units, bytes) is a plain import: the text of a SetClipboardData(CF_UNICODETEXT) goes to Module.fleClipWrite(text), and the Worker posts it to the page.
// fle_clip_host_read() is an ASYNC import (WebAssembly.Suspending): the engine's GetClipboardData / IsClipboardFormatAvailable suspend on it; the Worker calls
// Module.fleClipRequest(id), the page reads the operating system's clipboard and answers through Module.fleClipAnswer(id, text | null). The answer is copied into a malloc'd
// NUL-terminated UTF-16LE block for the C side (fle_clip.c os_text_read frees it). No answer in CLIP_TIMEOUT_MS, no host, or no text: the answer is NULL and the engine goes on.
// The library holds no policy: whether the page may read or write is the page's (app/src/engine/clipboard.ts).
addToLibrary({
  $FleClip: { count: 0, pending: new Map(), timeoutMs: 3000, maxUnits: 65536 },
  fle_clip_host_write__sig: 'vii',
  fle_clip_host_write: (units, bytes) => {
    const text = new TextDecoder('utf-16le').decode(HEAPU8.slice(units, units + bytes));
    if (typeof Module['fleClipWrite'] === 'function') Module['fleClipWrite'](text);
  },
  fle_clip_host_read__deps: ['$FleClip', 'malloc'],
  fle_clip_host_read__sig: 'i',
  fle_clip_host_read__async: true,
  fle_clip_host_read: () => {
    const S = FleClip;
    const id = ++S.count;
    return new Promise((resolve) => {
      let timer = null;
      const done = (text) => {
        if (!S.pending.has(id)) return;
        S.pending.delete(id);
        clearTimeout(timer);
        if (typeof text !== 'string' || text.length === 0) { resolve(0); return; }
        const t = text.length > S.maxUnits ? text.slice(0, S.maxUnits) : text;
        const p = _malloc(2 * (t.length + 1));   // malloc first: it may grow the memory, the view is read after it
        for (let i = 0; i < t.length; i++) { const c = t.charCodeAt(i); HEAPU8[p + 2 * i] = c & 0xff; HEAPU8[p + 2 * i + 1] = c >> 8; }
        HEAPU8[p + 2 * t.length] = 0;
        HEAPU8[p + 2 * t.length + 1] = 0;
        resolve(p);
      };
      S.pending.set(id, done);
      timer = setTimeout(() => done(null), S.timeoutMs);
      Module['fleClipAnswer'] = (answerId, text) => { const f = S.pending.get(answerId); if (f) f(text); };
      if (typeof Module['fleClipRequest'] === 'function') Module['fleClipRequest'](id);   // the Worker posts {t: 'clipboard-read'} to the page
      else done(null);
    });
  },
});
