/* Signed divide (IDIV) and one-operand IMUL for fe-lift output. Defined in fle_int_ext.c.
 * See docs/decisions/0010-int-ext.md. */
#ifndef FLE_INT_EXT_H
#define FLE_INT_EXT_H

#include <stdint.h>

/* Divides the 2w-bit signed dividend by the w-bit signed src. Returns the quotient
 * (masked to w bits) and stores the remainder (masked) in *rem. #DE aborts. */
uint64_t fle_idiv(unsigned w, unsigned __int128 dvd, uint64_t src, uint64_t *rem);

/* One-operand IMUL: returns the high w bits of the signed product and stores the
 * low w bits in *lo. CF = OF = (the product is not the sign extension of its low half). */
uint64_t fle_imul1(unsigned w, uint64_t a, uint64_t b, uint64_t *lo);

#endif
