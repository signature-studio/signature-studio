// The names a WebAssembly module exports, read from its export section without compiling it (a 170 MB engine wasm is read for a few KB: the sections before the export section are skipped by their size).
// Used by scripts/pack-assets.mjs to write `engine.desk: true` into a manifest only when the wasm really has the desktop compositor (fle_desk_compose, fle_desk_resync).
import fs from 'node:fs';

function leb(buf, pos) {   // unsigned LEB128 -> [value, next position]; values up to 2^53
  let result = 0, mul = 1, p = pos;
  for (;;) {
    if (p >= buf.length) throw new Error('truncated LEB128');
    const b = buf[p++];
    result += (b & 0x7f) * mul;
    if ((b & 0x80) === 0) return [result, p];
    mul *= 128;
    if (mul > 2 ** 49) throw new Error('LEB128 too long');
  }
}

/** the export names of the wasm file at `file` (an array of strings) */
export function wasmExportNames(file) {
  const fd = fs.openSync(file, 'r');
  try {
    const head = Buffer.alloc(8);
    if (fs.readSync(fd, head, 0, 8, 0) !== 8 || head.readUInt32LE(0) !== 0x6d736100 || head.readUInt32LE(4) !== 1) throw new Error(`${file} is not a WebAssembly module (version 1)`);
    const size = fs.fstatSync(fd).size;
    let pos = 8;
    while (pos < size) {
      const sh = Buffer.alloc(11);
      const n = fs.readSync(fd, sh, 0, 11, pos);
      const id = sh[0];
      const [len, after] = leb(sh.subarray(0, n), 1);
      const bodyAt = pos + after;
      if (id === 7) {
        const body = Buffer.alloc(len);
        fs.readSync(fd, body, 0, len, bodyAt);
        let [count, p] = leb(body, 0);
        const names = [];
        for (let i = 0; i < count; i++) {
          let l; [l, p] = leb(body, p);
          names.push(body.subarray(p, p + l).toString('utf8'));
          p += l + 1;                       // the kind byte
          [, p] = leb(body, p);             // the index
        }
        return names;
      }
      pos = bodyAt + len;
    }
    return [];
  } finally { fs.closeSync(fd); }
}

export const DESK_EXPORTS = ['fle_desk_compose', 'fle_desk_resync'];
/** true when the wasm file exports the compositor's entry points */
export function wasmHasDesk(file) { const names = new Set(wasmExportNames(file)); return DESK_EXPORTS.every((n) => names.has(n)); }
