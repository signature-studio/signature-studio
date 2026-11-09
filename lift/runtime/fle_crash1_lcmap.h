/* Decision 0209 (crash1): the case table of LCMapStringW for the units outside the range D of decision 0087. */
#ifndef FLE_CRASH1_LCMAP_H
#define FLE_CRASH1_LCMAP_H

#include <stdint.h>

/* The Unicode Character Database 16.0.0 simple case mapping of code point cp (cp is a code point, 0..0x10FFFF; a surrogate unit is not a code point and is returned unchanged): the upper case when `upper`
 * is nonzero, else the lower case; cp itself when it has none. UNOFFICIAL for Windows (see fle_crash1_case.inc). The range D (fle_locale.c) is answered by its own table before this one. */
uint32_t fle_crash1_ucd_case(uint32_t cp, int upper);

/* The case of the code point (or UTF-16 unit) cp under the two rules of the Learn pages ("Use linguistic rules for casing, instead of file system rules (default)"): linguistic = the UCD simple mapping; otherwise (file system rules) a
 * letter is mapped only where the second letter maps back to it (the table is a list of pairs, decision 0087's reading; Wine 10 agrees on all code points, decision 0209). A surrogate unit is returned as it is. */
uint32_t fle_crash1_case(uint32_t cp, int upper, int linguistic);

/* Number of runs of the table (for the test). */
unsigned fle_crash1_ucd_runs(void);

#endif
