#!/usr/bin/env node
// The import rows of one VST3 module (the node replacement of the mk_stubs.py that gen_vst3a.sh called).
// usage: node mk_stubs.mjs PE IMPORTS_TABLE_RS IMPORTS_VST RUNTIME_DIR ROWS_OUT STUBS_C ORD_ROWS_OUT [ORD_ANSWERS]
//   PE              the PE image (its import table gives the (DLL, API) names; an ordinal import has no name)
//   IMPORTS_TABLE_RS lift/crates/fe-lift/src/imports_table.rs: the rows the lift already has (TABLE), not repeated
//   IMPORTS_VST     lift/harness/vst/imports-vst.txt: the answers of the harness (vst3a C files), not repeated
//   RUNTIME_DIR     lift/runtime: an API whose fle_imp_<API> is defined there gets a row to it
//   ROWS_OUT        "<dll> <API> <runtime function>" lines (fe-lift --extra-imports), for every import not covered above
//   STUBS_C         the weak stopping stubs of the APIs that no answer defines, and the ordinal table (vst3a_ord_table, vst3a.h)
//   ORD_ROWS_OUT    "<0x pseudo-VA> <function>" dispatch rows of the ordinal imports with no answer (merged with the host rows)
//   ORD_ANSWERS     lift/harness/vst/ordinals-vst.txt (optional): "<dll> <ordinal> <API> <answer function>" rows. An ordinal with a row gets the extra-imports row
//                   "<dll> #<ordinal> <answer function>" (its thunk calls the answer) and no stub; its IAT slot gets the lift's response row of "#<ordinal>" (main_vst_ext.c)
// An API with no answer stops the run by name when it is called (vst3a_unanswered, vst3a_imp.c). Decision of #1367: no import is answered by a guess.
import fs from 'node:fs';
import path from 'node:path';

const [pePath, tablePath, importsVstPath, runtimeDir, rowsOut, stubsOut, ordRowsOut, ordAnswersPath] = process.argv.slice(2);
if (!ordRowsOut) {
  console.error('usage: node mk_stubs.mjs PE IMPORTS_TABLE_RS IMPORTS_VST RUNTIME_DIR ROWS_OUT STUBS_C ORD_ROWS_OUT [ORD_ANSWERS]');
  process.exit(64);
}
const ordAnswers = new Map();
if (ordAnswersPath) {
  for (const raw of fs.readFileSync(ordAnswersPath, 'utf8').split('\n')) {
    const t = raw.split('#')[0].trim();
    if (!t) continue;
    const [dll, ordinal, , fn] = t.split(/\s+/);
    ordAnswers.set(`${dll.toLowerCase()} ${Number(ordinal)}`, fn);
  }
}

/** (dll, API) names of the import table of a PE image; an ordinal import is `{ dll, ordinal }` */
export function peImportNames(buf) {
  if (buf.readUInt16LE(0) !== 0x5a4d) throw new Error('not a PE image');
  const coff = buf.readUInt32LE(0x3c) + 4;
  const sectionCount = buf.readUInt16LE(coff + 2);
  const optSize = buf.readUInt16LE(coff + 16);
  const opt = coff + 20;
  const magic = buf.readUInt16LE(opt);
  if (magic !== 0x20b) throw new Error('not a PE32+ image');
  const importVA = buf.readUInt32LE(opt + 112 + 8);
  const sections = [];
  for (let i = 0; i < sectionCount; i++) {
    const s = opt + optSize + 40 * i;
    sections.push({ va: buf.readUInt32LE(s + 12), size: Math.max(buf.readUInt32LE(s + 8), buf.readUInt32LE(s + 16)), ptr: buf.readUInt32LE(s + 20) });
  }
  const offsetOf = (rva) => {
    const s = sections.find((x) => rva >= x.va && rva < x.va + x.size);
    if (!s) throw new Error(`PE: RVA ${rva} is in no section`);
    return s.ptr + rva - s.va;
  };
  const cstr = (off) => buf.toString('latin1', off, buf.indexOf(0, off));
  const out = [];
  if (importVA === 0) return out;
  for (let d = offsetOf(importVA); ; d += 20) {
    const oft = buf.readUInt32LE(d);
    const nameRVA = buf.readUInt32LE(d + 12);
    const ft = buf.readUInt32LE(d + 16);
    if (nameRVA === 0 && oft === 0 && ft === 0) break;
    const dll = cstr(offsetOf(nameRVA)).toLowerCase();
    for (let k = 0; ; k++) {
      const th = buf.readBigUInt64LE(offsetOf(oft || ft) + 8 * k);
      if (th === 0n) break;
      if (th >> 63n) out.push({ dll, ordinal: Number(th & 0xffffn) });
      else out.push({ dll, api: cstr(offsetOf(Number(th & 0x7fffffffn)) + 2) });
    }
  }
  return out;
}

/** the APIs whose runtime function is defined in lift/runtime (a definition, not a declaration) */
function runtimeDefined(dir) {
  const set = new Set();
  for (const f of fs.readdirSync(dir)) {
    if (!/^fle_.*\.c$/.test(f)) continue;
    const text = fs.readFileSync(path.join(dir, f), 'latin1');
    for (const m of text.matchAll(/^void fle_imp_([A-Za-z0-9_]+)\(void\)\s*(\{|$)/gm)) set.add(m[1]);
  }
  return set;
}

/** the C name of the runtime function of an API: fle_imp_<API>. A C++ (mangled) API name has characters a C identifier cannot have; each of them becomes _<hex>_ */
function imp_fn(api) {
  return `fle_imp_${/^[A-Za-z0-9_]+$/.test(api) ? api : api.replace(/[^A-Za-z0-9_]/g, (c) => `_${c.charCodeAt(0).toString(16).padStart(2, '0')}_`)}`;
}

/** a C string literal of an API name (a ? is escaped, so that no trigraph can form) */
const cstr = (s) => JSON.stringify(s).replace(/\?/g, '\\?');

/** the TABLE rows of fe-lift's imports_table.rs: (dll, API) pairs, lower case dll */
function tableRows(text) {
  const set = new Set();
  for (const m of text.matchAll(/\("([^"]+)", "([^"]+)", "[^"]+", "[^"]+"\)/g)) set.add(`${m[1].toLowerCase()} ${m[2]}`);
  return set;
}

const pe = fs.readFileSync(pePath);
const imports = peImportNames(pe);
const table = tableRows(fs.readFileSync(tablePath, 'utf8'));
const vstRows = fs.readFileSync(importsVstPath, 'utf8').split('\n').map((l) => l.split('#')[0].trim()).filter(Boolean);
const vstKeys = new Set(vstRows.map((l) => {
  const [dll, api] = l.split(/\s+/);
  return `${dll.toLowerCase()} ${api}`;
}));
const vstFns = new Set(vstRows.map((l) => l.split(/\s+/)[2]));
const runtime = runtimeDefined(runtimeDir);

const rows = [];
const stubs = new Map();
const seen = new Set();
const fnOwner = new Map();
const ords = [];
for (const im of imports) {
  if (im.ordinal !== undefined) {
    if (!ords.some((o) => o.dll === im.dll && o.ordinal === im.ordinal)) {
      const answer = ordAnswers.get(`${im.dll} ${im.ordinal}`) ?? null;
      ords.push({ dll: im.dll, ordinal: im.ordinal, answer });
      if (answer) rows.push(`${im.dll} #${im.ordinal} ${answer}`);
    }
    continue;
  }
  const key = `${im.dll} ${im.api}`;
  if (seen.has(key) || table.has(key) || vstKeys.has(key)) continue;
  seen.add(key);
  const fn = imp_fn(im.api);
  if (fnOwner.has(fn) && fnOwner.get(fn) !== im.api) throw new Error(`mk_stubs: two APIs share the name ${fn}`);
  fnOwner.set(fn, im.api);
  rows.push(`${im.dll} ${im.api} ${fn}`);
  if (!runtime.has(im.api) && !vstFns.has(fn)) stubs.set(fn, { dll: im.dll, api: im.api });
}

const ordRows = [];
const ordEntries = [];
ords.forEach((o, k) => {
  if (o.answer) return;
  const va = 0x7FFC50000000 + 0x10 * (k + 1);
  ordRows.push(`0x${va.toString(16)} vst3a_ord_${k}`);
  ordEntries.push(`    { ${JSON.stringify(o.dll)}, ${o.ordinal}, UINT64_C(0x${va.toString(16)}), vst3a_ord_${k} },`);
});

const c = [];
c.push('/* generated by lift/harness/vst/mk_stubs.mjs for one module: the weak stopping stubs of the imports that no answer defines (they stop by name, vst3a_unanswered),');
c.push(' * and the ordinal table (vst3a_ord_table) that iat_fixup reads. Do not edit. */');
c.push('#include <stdint.h>');
c.push('#include "vst3a.h"');
c.push('');
for (const [fn, { dll, api }] of stubs) c.push(`__attribute__((weak)) void ${fn}(void) { vst3a_unanswered(${JSON.stringify(dll)}, ${cstr(api)}); }`);
ords.forEach((o, k) => { if (!o.answer) c.push(`static void vst3a_ord_${k}_stop(void) { vst3a_unanswered(${JSON.stringify(o.dll)}, ${JSON.stringify('#' + o.ordinal)}); }`); });
ords.forEach((o, k) => { if (!o.answer) c.push(`void vst3a_ord_${k}(void) { vst3a_ord_${k}_stop(); }`); });
c.push('');
c.push('const vst3a_ord_entry vst3a_ord_table[] = {');
c.push(...ordEntries);
c.push('    { NULL, 0, 0, NULL },');
c.push('};');
c.push(`const unsigned vst3a_ord_count = ${ordEntries.length};`);
c.push('');

fs.writeFileSync(rowsOut, rows.join('\n') + (rows.length ? '\n' : ''));
fs.writeFileSync(stubsOut, c.join('\n'));
fs.writeFileSync(ordRowsOut, ordRows.join('\n') + (ordRows.length ? '\n' : ''));
console.error(`mk_stubs: ${imports.length} imports; rows ${rows.length} (stubs ${stubs.size}); ordinal imports ${ords.length}`);
