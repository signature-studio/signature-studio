/* x87 FPU control state for fe-lift output: one struct for the runtime.
 * FNSTCW, FLDCW, FNSTSW, FNCLEX and FNINIT go through the functions below
 * (runtime/fle_x87ctl.c). x87 arithmetic is not implemented, so the status word
 * never gets an exception flag and the tag word only changes at FNINIT.
 * Reset state: control 037FH, status 0, tag FFFFH (Intel SDM Vol. 2, FINIT/FNINIT). */
#ifndef FLE_X87CTL_H
#define FLE_X87CTL_H

#include <stdint.h>

typedef struct {
    uint16_t cw; /* control word */
    uint16_t sw; /* status word */
    uint16_t tw; /* tag word */
} fle_x87_state;

extern fle_x87_state fle_x87;

/* FNSTCW m16: writes the control word to the 16-bit location at address a. */
void fle_x87_store_cw(uint64_t a);

/* FLDCW m16: reads the 16-bit location at address a into the control word.
 * Stored as (v & 0x1F3F) | 0x0040, which is what the host CPU reads back
 * (measured, see docs/decisions/0019-x87-control.md). Never aborts. */
void fle_x87_load_cw(uint64_t a);

/* FNSTSW m16: writes the status word to the 16-bit location at address a. */
void fle_x87_store_sw(uint64_t a);

/* FNSTSW AX: the status word, for the low 16 bits of RAX. */
uint16_t fle_x87_sw(void);

/* FNCLEX: clears status word bits 0-7 (PE UE OE ZE DE IE SF ES) and bit 15 (B). */
void fle_x87_clex(void);

/* FNINIT: control 037FH, status 0, tag FFFFH. */
void fle_x87_init(void);

#endif
