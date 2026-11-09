/* The link-time registration of the real module of the two-module build (lift/runtime/fle_realmod.h, decision 0082).
 * The name is the spelling that normalize() of fle_load.c produces: the guest calls SetDllDirectoryW("Shared\\") and then
 * LoadLibraryW("dsp_ippv2_x64.dll"). The base is the one the file was rebased to (decision 0080 D1); build_two.sh passes the
 * same value to `fe-pe rebase` and here. */
#include "fle_realmod.h"

#ifndef REALMOD_NAME
#define REALMOD_NAME "dsp_ippv2_x64.dll"
#endif
#ifndef REALMOD_BASE
#define REALMOD_BASE UINT64_C(0x2000000)
#endif

const fle_realmod_entry fle_realmod_table[] = { { REALMOD_NAME, REALMOD_BASE } };
const uint64_t fle_realmod_count = 1;
