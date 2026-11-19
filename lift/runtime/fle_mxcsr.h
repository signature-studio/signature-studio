/* MXCSR for fe-lift output: one variable for the runtime. STMXCSR and LDMXCSR
 * go through fle_mxcsr_store and fle_mxcsr_load (runtime/fle_mxcsr.c).
 * The reset value is 0x1F80 (Intel SDM Vol. 1, MXCSR; LDMXCSR "default MXCSR
 * value at reset is 1F80H"). */
#ifndef FLE_MXCSR_H
#define FLE_MXCSR_H

#include <stdint.h>

extern uint32_t fle_mxcsr;

/* STMXCSR m32: writes fle_mxcsr to the 32-bit location at address a. */
void fle_mxcsr_store(uint64_t a);

/* LDMXCSR m32: reads the 32-bit location at address a into fle_mxcsr.
 * Aborts if a reserved bit (31:16) is set. */
void fle_mxcsr_load(uint64_t a);

#endif
