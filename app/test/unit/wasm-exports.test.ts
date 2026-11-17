import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
// @ts-expect-error a plain .mjs script helper
import { DESK_EXPORTS, wasmExportNames, wasmHasDesk } from '../../scripts/lib/wasm-exports.mjs';

const u32 = (n: number): number[] => { const o: number[] = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; };
const str = (s: string): number[] => [...u32(s.length), ...Buffer.from(s)];
/** a module with a type section, a function section, a code section and an export section that names `names` (all function 0), written to a temp file */
function wasmWith(names: string[], dir: string): string {
  const typeSec = [1, 4, 1, 0x60, 0, 0];
  const funcSec = [3, 2, 1, 0];
  const exp = [...u32(names.length), ...names.flatMap((n) => [...str(n), 0, 0])];
  const expSec = [7, ...u32(exp.length), ...exp];
  const body = [0, 0x0b];   // no locals, end
  const codeSec = [10, ...u32(body.length + 2), 1, body.length, ...body];
  const bytes = Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, ...typeSec, ...funcSec, ...expSec, ...codeSec]);
  const f = path.join(dir, `m${names.length}-${Math.random().toString(16).slice(2, 8)}.wasm`);
  fs.writeFileSync(f, bytes);
  new WebAssembly.Module(bytes);   // the test module is valid
  return f;
}

describe('wasmExportNames / wasmHasDesk', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wasm-exports-'));
  it('reads the export names without compiling', () => {
    expect(wasmExportNames(wasmWith(['a', 'fle_desk_compose'], dir))).toEqual(['a', 'fle_desk_compose']);
  });
  it('wasmHasDesk needs both entry points', () => {
    expect(DESK_EXPORTS).toEqual(['fle_desk_compose', 'fle_desk_resync']);
    expect(wasmHasDesk(wasmWith(['fle_desk_compose', 'fle_desk_resync', 'x'], dir))).toBe(true);
    expect(wasmHasDesk(wasmWith(['fle_desk_compose'], dir))).toBe(false);
    expect(wasmHasDesk(wasmWith([], dir))).toBe(false);
  });
  it('refuses a file that is not a wasm module', () => {
    const f = path.join(dir, 'bad.wasm'); fs.writeFileSync(f, 'not wasm at all');
    expect(() => wasmExportNames(f)).toThrow(/not a WebAssembly module/);
    fs.rmSync(dir, { recursive: true, force: true });
  });
});
