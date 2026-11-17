/* The plugin as a real module of the runtime (docs/decisions/0082-two-module-load-semantics.md; fle_realmod.h): the link supplies the table. A real module has a name that the
 * guest's LoadLibraryW matches, a base that is its ImageBase, DllMain run by that LoadLibraryW, and an hInstance that CreateWindowExW / RegisterClassW accept
 * (fle_class_hinst_known). build_vst_native.sh defines VST_RM_NAME (lower-case, with the extension; no path: fle_load.c stops on a path) and VST_RM_BASE (the ImageBase of the
 * file, from its PE header). */
#include "fle_realmod.h"

#ifndef VST_RM_NAME
#error "VST_RM_NAME is not defined"
#endif
const fle_realmod_entry fle_realmod_table[] = {
    { VST_RM_NAME, (uint64_t)VST_RM_BASE },
};
const uint64_t fle_realmod_count = 1;
