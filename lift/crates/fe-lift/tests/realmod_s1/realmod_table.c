/* The link-time registration of the real module of the regression test (lift/runtime/fle_realmod.h). RM_ENTRIES may be overridden with
 * -D to test a table that the runtime must refuse. */
#include "fle_realmod.h"

#ifndef RM_ENTRIES
#define RM_ENTRIES { "mod2.dll", 0x2000000 }
#endif
#ifndef RM_COUNT
#define RM_COUNT 1
#endif

const fle_realmod_entry fle_realmod_table[] = { RM_ENTRIES };
const uint64_t fle_realmod_count = RM_COUNT;
