/* X1 probe (docs/investigations/blend2d-jit-and-drawing-path.md): after DllMain has returned, call the real module Blend2D's export
 * AggCreateInstance, attach a small 32-bit bitmap to the returned Agg2D object and draw into it with the guest's own code, while
 * recording every VirtualAlloc / VirtualProtect / CreateFileMappingW / MapViewOfFile the guest makes (JIT = executable memory).
 *
 * Compiled into the harness only when lift/harness/two/build_two_native.sh runs with B2D_PROBE=1 (a4-multi.sh does it for
 * FLE_BLEND2D_PROBE=1); main_two.c calls these functions under #ifdef FLE_B2D_PROBE. A build without B2D_PROBE=1 contains none of this.
 * Native only. It never answers an API for the guest: a stop of the guest (abort, fault, limit) ends the process with a "b2d:" report.
 *
 * Run time (a build with B2D_PROBE=1): FLE_BLEND2D_PROBE=1 turns the phase on; unset, empty or 0 = the run is the one of a build without it.
 *   FLE_B2D_MODE=agg|nojit     agg (default): the context that Agg2D::attach makes (createInfo all zero, so JIT enabled). nojit: after attach,
 *                              the context is replaced by one that the probe makes with BL_CONTEXT_CREATE_FLAG_DISABLE_JIT (an EXPERIMENT:
 *                              the guest's Agg2D never asks for this; it shows whether Blend2D has a path without generated code)
 *   FLE_B2D_TLS=1              EXPERIMENT: before the calls, do what the Windows loader does for a module with a TLS directory (the runtime does not):
 *                              copy the 32-byte TLS template into a fresh block, put the block in the guest's TLS slot table (slot 1000) and write that index
 *                              to the module's _tls_index. Default 0 (the runtime's own state: _tls_index stays 0)
 *   FLE_B2D_OPS=<list>         comma-separated drawing steps, in this order, from: clear rect blend line path ellipse (default: all; `none` = attach only)
 *   FLE_B2D_MAX_CALLS=<n>      lifted-function entries allowed after the probe began (default 600000000)
 *   FLE_B2D_SECONDS=<s>        wall time allowed (default 600, 1..3600)
 *   FLE_B2D_TRAP=1             after the stop report raise SIGTRAP instead of _exit (run under gdb to read the C stack)
 * Any FLE_B2D_* variable without FLE_BLEND2D_PROBE=1 is refused (exit 2 before the guest runs), as is any other value. */
#ifndef FLE_HARNESS_B2D_PROBE_H
#define FLE_HARNESS_B2D_PROBE_H

#include <stdint.h>

/* Before the guest runs: reads and checks the variables. 0 = go on (also when the phase is off), -1 = refused (message on stderr). */
int b2d_probe_precheck(void);
/* 1 when FLE_BLEND2D_PROBE=1 (valid only after b2d_probe_precheck returned 0). */
int b2d_probe_requested(void);
/* After DllMain returned and its report was printed. entry_sp = R[4] before the entry call. Returns the exit code of the harness
 * (0 = every step returned); a stop ends the process itself. */
int b2d_probe_run(uint64_t entry_sp);

#endif
