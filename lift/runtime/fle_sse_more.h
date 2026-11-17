/* Packed SSE helpers for fe-lift output: PACKUSWB, PADDW, ADDPS, SUBPS, MULPS,
 * MINPS, MAXPS. Defined in fle_sse_more.c. The generated code declares each one
 * at block scope. See docs/decisions/0013-more-insns.md. */
#ifndef FLE_SSE_MORE_H
#define FLE_SSE_MORE_H

#include "fle_sse.h"

fle_xmm fle_packuswb(fle_xmm a, fle_xmm b);
fle_xmm fle_paddw(fle_xmm a, fle_xmm b);
fle_xmm fle_addps(fle_xmm a, fle_xmm b);
fle_xmm fle_subps(fle_xmm a, fle_xmm b);
fle_xmm fle_mulps(fle_xmm a, fle_xmm b);
fle_xmm fle_minps(fle_xmm a, fle_xmm b);
fle_xmm fle_maxps(fle_xmm a, fle_xmm b);

#endif
