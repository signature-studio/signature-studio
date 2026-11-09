/* Definition of the string-instruction direction flag. Starts at 0 (CLD), as the
 * Windows x64 ABI requires at function entry. See fle_string.h. */
#include "fle_string.h"

uint8_t fle_df = 0;
