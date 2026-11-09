/* Decision 0161 (provisional): FLE_AI marks the small static inline helpers of the runtime headers (fle_ld, fle_st, fle_op_*, fle_cc, the SSE and the no-flags helpers) for
 * always_inline, but only in a translation unit that is compiled with -DFLE_ALWAYS_INLINE. Without the macro FLE_AI is empty and every header is what it was.
 * Why per unit: decision 0151 measured that wasm -Oz leaves these helpers as calls (57% of the node samples of a start-up) and that always_inline on the units that
 * hold the start-up (the top N of docs/investigations/hle1/tables/hot-units.tsv) takes most of the gain for a part of the wasm growth (all 92 units: +58.6%). */
#ifndef FLE_AI_H
#define FLE_AI_H

#ifdef FLE_ALWAYS_INLINE
#define FLE_AI __attribute__((always_inline))
#else
#define FLE_AI
#endif

#endif
