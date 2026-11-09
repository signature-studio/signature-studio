// JS side of the browser file dialog bridge (decision 0193, fpick1, PROVISIONAL; REPLACES the design sketch of decision 0178). An emscripten JS library, like harness/web/fruity/fle_idle_lib.js:
// link with -sJSPI --js-library <this file>. fle_filedlg_host_pick(requestPtr) is an ASYNC import (WebAssembly.Suspending): the engine's
// IFileOpenDialog::Show suspends on it; the Worker posts the request to the page (Module.fleFileDlg), the PERSON at the page chooses (the page's overlay; a file picker needs a click of the person), and the Worker calls
// Module.fleFileDlgAnswer(id, answer) with the chosen File objects. This file reads each File straight into a malloc'd block of the engine's memory (no second copy of the bytes in JS), then resolves the Promise with
// a malloc'd text the C side parses (fle_filedlg.c browser_picker): `OK` then one line per item `<role> TAB <ptr> TAB <size> TAB <guest path>`, or `CANCEL`.
// answer = { ok: false } | { ok: true, typeIndex?: <1-based file type the person picked>, items: [{ role: 'R' | 'E' | 'S' | 'D', path: <guest path, C:\...>, file?: File }] }.  The library holds no policy: the paths and the limit come from the page's plan (app/src/engine/filedlg.ts).
// Module.fleFileDlgProgress(id, doneBytes, totalBytes), when set, is called while a large file is being read.
addToLibrary({
  $FleFileDlg: { count: 0, pending: null, maxBytes: 256 * 1024 * 1024 },
  fle_filedlg_host_pick__deps: ['$FleFileDlg', '$UTF8ToString', '$stringToUTF8', '$lengthBytesUTF8', 'malloc', 'free'],
  fle_filedlg_host_pick__sig: 'ip',
  fle_filedlg_host_pick__async: true,
  fle_filedlg_host_pick: (requestJsonPtr) => {
    const S = FleFileDlg;
    const request = JSON.parse(UTF8ToString(requestJsonPtr));
    const id = ++S.count;
    const toC = (text) => { const n = lengthBytesUTF8(text) + 1; const p = _malloc(n); stringToUTF8(text, p, n); return p; };
    return new Promise((resolve) => {
      const done = (text) => { if (S.pending && S.pending.id === id) S.pending = null; resolve(toC(text)); };
      S.pending = { id, done };
      Module['fleFileDlgAnswer'] = (answerId, answer) => {
        if (!S.pending || S.pending.id !== answerId) return false;
        (async () => {
          if (!answer || !answer.ok || !Array.isArray(answer.items) || answer.items.length === 0) { done('CANCEL'); return; }
          const lines = ['OK'];
          if (Number.isInteger(answer.typeIndex) && answer.typeIndex >= 1) lines.push(`TYPE\t${answer.typeIndex}`);   // decision 0216: the file type the person picked (1-based), for GetFileTypeIndex
          const blocks = [];
          const fail = () => { for (const b of blocks) _free(b); done('CANCEL'); };   // the blocks of the files already read belong to nobody
          let total = 0, read = 0;
          for (const it of answer.items) total += it.file ? it.file.size : 0;
          for (const it of answer.items) {
            if (!it || (it.role !== 'R' && it.role !== 'E' && it.role !== 'S' && it.role !== 'D') || typeof it.path !== 'string' || /[\t\r\n\u0000]/.test(it.path)) { fail(); return; }
            if (it.role === 'S' || it.role === 'D') { lines.push(`${it.role}\t0\t0\t${it.path}`); continue; }   // D (decision 0216): the folder a folder dialog returns
            const file = it.file;
            if (!file || file.size > S.maxBytes) { fail(); return; }
            const size = file.size;
            const ptr = size > 0 ? _malloc(size) : 0;
            if (size > 0 && ptr === 0) { fail(); return; }
            if (ptr) blocks.push(ptr);
            const reader = file.stream().getReader();
            let off = 0;
            for (;;) {
              const { done: eof, value } = await reader.read();
              if (eof) break;
              if (off + value.length > size) { fail(); return; }
              HEAPU8.set(value, ptr + off);   // HEAPU8 is looked up again at every chunk: the engine's memory may have grown meanwhile
              off += value.length;
              read += value.length;
              if (typeof Module['fleFileDlgProgress'] === 'function') Module['fleFileDlgProgress'](id, read, total);
            }
            if (off !== size) { fail(); return; }
            lines.push(`${it.role}\t${ptr}\t${size}\t${it.path}`);
          }
          done(lines.join('\n'));
        })().catch(() => done('CANCEL'));   // (a block read before the throw is lost: a failure of the stream itself, not expected)
        return true;
      };
      if (typeof Module['fleFileDlg'] === 'function') Module['fleFileDlg'](id, request);   // the Worker posts {t: 'filedlg-request'} to the page
      else done('CANCEL');                                                                // no host: nobody to ask
    });
  },
});
