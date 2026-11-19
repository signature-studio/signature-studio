/* Unsigned DIV and BT for fe-lift output. Defined in fle_int_more.c.
 * See docs/decisions/0013-more-insns.md. */
#ifndef FLE_INT_MORE_H
#define FLE_INT_MORE_H

#include <stdint.h>

/* Divides the 2w-bit unsigned dividend by the w-bit src. Returns the quotient
 * (w bits) and stores the remainder in *rem. Zero divisor and a quotient above
 * the w-bit range abort with the #DE message. Flags become undefined. */
uint64_t fle_div(unsigned w, unsigned __int128 dvd, uint64_t src, uint64_t *rem);

/* BT: sets CF to cf. ZF is unchanged. OF, SF, AF and PF become undefined. */
void fle_bt_cf(uint8_t cf);

#endif
