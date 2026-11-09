/* Direction flag for string instructions in fe-lift output.
 *
 * fle_df: 0 increments RSI and RDI, 1 decrements them. It is not in fle_eflags
 * because fle_flags.h has no DF. It is defined in fle_string.c, which must be
 * linked with the generated translation unit. The generated bodies declare it
 * with `extern uint8_t fle_df;` and do not include this header.
 */
#ifndef FLE_STRING_H
#define FLE_STRING_H

#include <stdint.h>

extern uint8_t fle_df;

#endif
