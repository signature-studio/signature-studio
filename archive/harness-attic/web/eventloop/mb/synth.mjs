// Synthetic stand-in for lifted code (NOT FL Studio code): F functions `void f_N(void)` in the shape fe-lift emits (guest registers in globals,
// guest memory through ld/st, `call` = push return address + C call + pop, indirect call = dispatch through a sorted VA table), only to measure
// what Asyncify does to the size and the build time of that shape. Deterministic (seed).
//   node synth.mjs <out.c> <functions> [seed=1] [meanOps=60] [callRate=0.08] [dispatchShare=0.15] [async=1]
// the call-site statistics used as defaults are described in docs/investigations/browser-event-loop.md (section M-B, "Asyncify code expansion").
import fs from 'node:fs';
const [out, F0, seed0, mean0, cr0, ds0, asyncMode, budgetArg] = process.argv.slice(2);
const syncWait = asyncMode === '2';   // run-time variant with a wait that does not suspend (C counter), the baseline of cps
const cps = asyncMode === '3';          // stackless transformation done by the generator (M-C'): unwind by returning, rewind by a resume switch
const budget = budgetArg === '1' || syncWait || cps;   // run-time variant: every function entry spends one unit of a global budget, so entry(n) runs a bounded number of calls
const F = +F0;
let s = +(seed0 || 1) >>> 0;
const rnd = () => { s = (s + 0x6D2B79F5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const ri = (n) => Math.floor(rnd() * n);
const meanOps = +(mean0 || 60);
const callRate = +(cr0 || 0.08);
const dispShare = +(ds0 || 0.15);
const fd = fs.openSync(out, 'w');
const w = (x) => fs.writeSync(fd, x);
w(`#include <stdint.h>
#include <string.h>
#include <emscripten.h>
typedef struct { uint8_t cf, zf, sf, of; } fle_eflags;
uint64_t R[16];
unsigned long fle_budget;
int fle_depth;
fle_eflags FL;
uint8_t GM[1 << 22];
static inline uint64_t fle_ld(uint64_t a, int n) { uint64_t v = 0; memcpy(&v, GM + (a & ((1 << 22) - 8)), n); return v; }
static inline void fle_st(uint64_t a, int n, uint64_t v) { memcpy(GM + (a & ((1 << 22) - 8)), &v, n); }
static inline uint64_t fle_op_add(int b, uint64_t x, uint64_t y, uint64_t c) { uint64_t r = x + y + c; FL.cf = r < x; FL.zf = r == 0; FL.sf = (uint8_t)(r >> 63); (void)b; return r; }
static inline uint64_t fle_op_sub(int b, uint64_t x, uint64_t y, uint64_t c) { uint64_t r = x - y - c; FL.cf = x < y; FL.zf = r == 0; FL.sf = (uint8_t)(r >> 63); (void)b; return r; }
static inline uint64_t fle_op_logic(int b, uint64_t r) { FL.cf = 0; FL.zf = r == 0; FL.sf = (uint8_t)(r >> 63); (void)b; return r; }
static inline int fle_cc(int k) { switch (k & 3) { case 0: return FL.cf; case 1: return FL.zf; case 2: return FL.sf; default: return !FL.zf; } }
`);
if (syncWait || cps) {
  w('int fle_mode, fle_sp;\nuint64_t fle_stk[1 << 16];\nunsigned long fle_counter, fle_waits;\n');
  if (syncWait) w('int fle_wait(void) { if ((++fle_counter & 63) == 0) fle_waits++; return 1; }\n');
  else w('int fle_wait(void) { if (fle_mode == 2) { fle_mode = 0; return 1; } if ((++fle_counter & 63) != 0) return 1; fle_mode = 1; fle_waits++; return 0; }\n');
} else if (asyncMode === '0') w('EM_JS(int, fle_imp_GetMessageW, (void), { return 0; });\n');
else w('EM_ASYNC_JS(int, fle_imp_GetMessageW, (void), { return await Module.asyncGet(0); });\n');
for (let j = 0; j < 8; j++) w(`int fle_imp_Nop${j}(void);\n`);
w('typedef struct { uint64_t va; void (*fn)(void); } fle_fn_entry;\nextern const fle_fn_entry fle_dispatch_table[];\n');
for (let i = 0; i < F; i++) w(`void f_${(0x1000 + i * 0x40).toString(16)}(void);\n`);
w(`__attribute__((noinline)) void fle_dispatch(uint64_t va) {
  uint64_t lo = 0, hi = ${F};
  while (lo < hi) { uint64_t mid = (lo + hi) / 2; if (fle_dispatch_table[mid].va < va) lo = mid + 1; else hi = mid; }
  if (lo < ${F} && fle_dispatch_table[lo].va == va) fle_dispatch_table[lo].fn(); else ${budget ? 'fle_dispatch_table[va % ' + F + '].fn()' : '__builtin_trap()'};
}
`);
const name = (i) => 'f_' + (0x1000 + i * 0x40).toString(16);
const getters = new Set([Math.floor(F / 2), Math.floor(F / 3), Math.floor(F / 7)]);
let totalOps = 0, totalCalls = 0, totalDisp = 0;
for (let i = 0; i < F; i++) {
  const n = Math.max(6, Math.round(-Math.log(1 - rnd()) * meanOps + 8));
  const lines = [];
  const sites = [];   // cps: resumable call sites of this function: {kind, label}
  for (let k = 0; k < n; k++) {
    const a = ri(16), b = ri(16), c = ri(16), imm = ri(256) * 8;
    const r = rnd();
    let body;
    if (r < callRate) {
      totalCalls++;
      const ret = 0x1000 + i * 0x40 + k * 4 + 4;
      const g1 = budget ? 'if (fle_depth < 60) { fle_depth++; ' : '';
      const g2 = budget ? ' fle_depth--; }' : '';
      if (rnd() < dispShare) {
        totalDisp++;
        if (cps) { const si = sites.length; sites.push({ kind: 'disp', label: `K_${i}_${k}` }); body = `{ rt = R[${a}]; R[4] -= 8; fle_st(R[4], 8, UINT64_C(${ret})); if (fle_depth < 60) { fle_depth++; K_${i}_${k}: fle_dispatch(rt); fle_depth--; if (fle_mode == 1) { fle_stk[fle_sp++] = rt; fle_stk[fle_sp++] = ${si}; return; } } R[4] += 8; }`; }
        else body = `{ uint64_t t = R[${a}]; R[4] -= 8; fle_st(R[4], 8, UINT64_C(${ret})); ${g1}fle_dispatch(t);${g2} R[4] += 8; }`;
      } else {
        const callee = name(ri(F));
        if (cps) { const si = sites.length; sites.push({ kind: 'direct', label: `K_${i}_${k}` }); body = `{ R[4] -= 8; fle_st(R[4], 8, UINT64_C(${ret})); if (fle_depth < 60) { fle_depth++; K_${i}_${k}: ${callee}(); fle_depth--; if (fle_mode == 1) { fle_stk[fle_sp++] = ${si}; return; } } R[4] += 8; }`; }
        else body = `{ R[4] -= 8; fle_st(R[4], 8, UINT64_C(${ret})); ${g1}${callee}();${g2} R[4] += 8; }`;
      }
    } else if (r < callRate + 0.01) body = `{ R[0] = (uint32_t)fle_imp_Nop${ri(8)}(); }`;
    else if (r < 0.40) body = `{ uint64_t t = fle_op_add(64, R[${a}], R[${b}], 0); R[${c}] = t; }`;
    else if (r < 0.50) body = `{ uint64_t t = fle_op_sub(64, R[${a}], UINT64_C(${imm}), 0); R[${c}] = t; }`;
    else if (r < 0.58) body = `{ (void)(fle_op_logic(32, ((uint32_t)R[${a}]) & ((uint32_t)R[${b}]))); }`;
    else if (r < 0.74) body = `R[${a}] = fle_ld((R[${b}] + UINT64_C(${imm})), 8);`;
    else if (r < 0.86) body = `fle_st((R[${b}] + UINT64_C(${imm})), 8, R[${a}]);`;
    else if (r < 0.94 && k + 2 < n) body = `if (fle_cc(${ri(4)})) { goto L_${i}_${k + 1 + ri(n - k - 1)}; }`;
    else body = `R[${a}] = R[${b}];`;
    lines.push(`L_${i}_${k}: { ${body} }\n`);
  }
  if (getters.has(i)) {
    if (cps) { const si = sites.length; sites.push({ kind: 'wait', label: `W_${i}` }); lines.push(`{ W_${i}: v = fle_wait(); if (fle_mode == 1) { fle_stk[fle_sp++] = ${si}; return; } R[0] = (uint32_t)v; }\n`); }
    else if (syncWait) lines.push('{ R[0] = (uint32_t)fle_wait(); }\n');
    else lines.push(budget ? 'if (R[15] == UINT64_C(0xdeadbeef)) { R[0] = (uint32_t)fle_imp_GetMessageW(); }\n' : '{ R[0] = (uint32_t)fle_imp_GetMessageW(); }\n');
  }
  w(`void ${name(i)}(void)\n{\n`);
  if (cps && sites.length) {
    if (sites.some((x) => x.kind === 'disp')) w('uint64_t rt = 0;\n');
    if (sites.some((x) => x.kind === 'wait')) w('int v = 0;\n');
    w('if (fle_mode == 2) { switch ((int)fle_stk[--fle_sp]) {\n');
    sites.forEach((x, si) => w(`case ${si}: ${x.kind === 'disp' ? 'rt = fle_stk[--fle_sp]; ' : ''}${x.kind === 'wait' ? '' : 'fle_depth++; '}goto ${x.label};\n`));
    w('default: __builtin_trap(); } }\n');
  }
  if (budget) w('if (fle_budget == 0) return; fle_budget--;\n');
  for (const l of lines) w(l);
  w(`L_${i}_${n}: return;\n}\n`);
  totalOps += n;
}
for (let j = 0; j < 8; j++) w(`__attribute__((noinline)) int fle_imp_Nop${j}(void) { return R[${j}] != 0; }\n`);
// the dispatch table is defined last and sorted by VA (it already is: VA grows with i)
w('const fle_fn_entry fle_dispatch_table[] = {\n');
for (let i = 0; i < F; i++) w(`{ UINT64_C(${0x1000 + i * 0x40}), ${name(i)} },\n`);
if (syncWait || cps) w('};\n__attribute__((used)) int entry(unsigned n) { if (fle_mode != 2) fle_budget = n; f_1000(); return (int)R[0]; }\n__attribute__((used)) int cps_mode(void) { return fle_mode; }\n__attribute__((used)) void cps_set(int m) { fle_mode = m; }\n__attribute__((used)) unsigned long checksum(void) { unsigned long s = fle_budget * 31 + fle_waits * 17 + fle_counter * 7; for (int i = 0; i < 16; i++) s = s * 1000003 + R[i]; return s; }\n__attribute__((used)) unsigned long waits(void) { return fle_waits; }\nint main(void) { return 0; }\n');
else w(budget ? '};\n__attribute__((used)) int entry(unsigned n) { fle_budget = n; f_1000(); return (int)R[0]; }\nint main(void) { return 0; }\n' : '};\n__attribute__((used)) int entry(void) { f_1000(); return (int)R[0]; }\nint main(void) { return 0; }\n');
fs.closeSync(fd);
console.log(JSON.stringify({ out, functions: F, ops: totalOps, calls: totalCalls, dispatchCalls: totalDisp, bytes: fs.statSync(out).size }));
