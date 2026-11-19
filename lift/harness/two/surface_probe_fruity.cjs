// Checks of the surface table API of the FRUITY-on wasm (decision 0128, PROVISIONAL; harness/fruity_surface.c; web/present/CONTRACT.md sections 2-3) from node, at the first non-background pixel and at the stop:
// the table that fle_surface_table() points to (read with the JS reader of the present1 harness, core.js SurfaceTable semantics, here re-written without the ES module) against the C functions fle_surface_count / fle_surface_info /
// fle_surface_take_dirty, and the dirty rectangle taken at the stop against the pixels (every pixel outside the rectangle is zero before the first take? no: a surface is all zero at the start, so the rectangle must cover every
// non-zero pixel of the surface; checked). Usage (the wasm must be built with FRUITY=1): node --experimental-wasm-jspi surface_probe_fruity.cjs <FLEngine dll> <real modules joined with ':'> (environment as run-node.sh).
// The probe runs inside the engine: Module.fleSurfaceEvent is called at the events (synchronously in the engine's stack), where the exports can be called.
const path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const create = require(path.join(dir, 'flengine6.js'));
const results = [];
let memory = null, exportsObj = null;
const W = (i) => new Int32Array(memory.buffer)[i];
function readTable() {
  const base = exportsObj.fle_surface_table() >> 2;
  const u = new Int32Array(memory.buffer);
  const head = { magic: u[base] >>> 0, version: u[base + 1], entrySize: u[base + 2], capacity: u[base + 3], live: u[base + 4] };
  const entries = [];
  for (let k = 0; k < head.capacity; k++) {
    const b = base + 8 + k * (head.entrySize >> 2);   // version 2 (decision 0145): the entry size comes from the header (112 B); version 1 had 96 B
    if (!(u[b + 3] & 1)) continue;
    entries.push({ slot: k, lseq: u[b] >>> 0, lid: u[b + 1] >>> 0, hwnd: u[b + 2] >>> 0, flags: u[b + 3] >>> 0, x: u[b + 4], y: u[b + 5], width: u[b + 6] >>> 0, height: u[b + 7] >>> 0, stride: u[b + 8] >>> 0, format: u[b + 9],
      pixels: u[b + 10] >>> 0, layoutGen: u[b + 11] >>> 0, gen: u[b + 17] >>> 0, dx0: u[b + 19] >>> 0, dy0: u[b + 20] >>> 0, dx1: u[b + 21] >>> 0, dy1: u[b + 22] >>> 0 });
  }
  return { head, entries };
}
const opts = { arguments: process.argv.slice(2) };
opts.fleSurfaceEvent = (kind, lid, pixels, w, h, stride) => {
  const t = readTable();
  const r = { kind, lid, table: t.head, entries: t.entries.map((e) => ({ lid: e.lid, hwnd: e.hwnd.toString(16), flags: e.flags, x: e.x, y: e.y, w: e.width, h: e.height, stride: e.stride, format: e.format, pixels: e.pixels, gen: e.gen, dirty: [e.dx0, e.dy0, e.dx1, e.dy1], layoutGen: e.layoutGen })) };
  r.count_c = exportsObj.fle_surface_count();
  // fle_surface_info(i, out) and fle_surface_take_dirty(lid, out) write to fle_surface_scratch(): an entry (entry_size / 4 words, 28 since version 2) and 5 words
  const scratch = exportsObj.fle_surface_scratch();
  r.info_c = [];
  for (let i = 0; i < r.count_c; i++) { const rc = exportsObj.fle_surface_info(i, scratch); const u = new Int32Array(memory.buffer, scratch, 28); r.info_c.push({ rc, lid: u[1] >>> 0, w: u[6] >>> 0, h: u[7] >>> 0, pixels: u[10] >>> 0, gen: u[17] >>> 0 }); }
  r.info_c_beyond = exportsObj.fle_surface_info(r.count_c, scratch);
  {
    const rc5 = exportsObj.fle_surface_take_dirty(lid, scratch);
    const v = new Uint32Array(memory.buffer, scratch, 5);
    r.take = { rc: rc5, gen: v[0], rect: [v[1], v[2], v[3], v[4]] };
    const rc6 = exportsObj.fle_surface_take_dirty(lid, scratch);
    const v2 = new Uint32Array(memory.buffer, scratch, 5);
    r.take2 = { rc: rc6, rect: [v2[1], v2[2], v2[3], v2[4]] };
    r.take_unknown_lid = exportsObj.fle_surface_take_dirty(4000, scratch);
    // every non-zero pixel must lie inside the first dirty rectangle (a surface is zero at creation and every write is reported): checked on the bytes
    const bytes = new Uint8Array(memory.buffer, pixels, stride * h);
    let bad = 0, nz = 0;
    const [x0, y0, x1, y1] = r.take.rect;
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const o = y * stride + 4 * x; if (bytes[o] | bytes[o + 1] | bytes[o + 2]) { nz++; if (x < x0 || x >= x1 || y < y0 || y >= y1) bad++; } }
    r.nonzero = nz; r.nonzeroOutsideDirty = bad;
  }
  results.push(r);
};
opts.instantiateWasm = (imports, done) => {
  const bytes = require('fs').readFileSync(path.join(dir, 'flengine6.wasm'));
  WebAssembly.instantiate(bytes, imports).then((r) => { memory = r.instance.exports.memory; exportsObj = r.instance.exports; opts.__exports = exportsObj; done(r.instance, r.module); });
  return {};
};
create(opts).catch(() => {}).finally(() => {});
process.on('exit', () => { require('fs').writeFileSync(process.env.WF1_PROBE_OUT || '/dev/stderr', JSON.stringify(results, null, 1) + '\n'); });
