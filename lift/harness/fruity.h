/* Stage 1 of CreateFruityInstance (docs/investigations/create-fruity-instance-harness.md): after DllMain has returned, call the engine's
 * export CreateFruityInstance(0xC0, 0xEDE95A, <fake host object>, &instance) as FL64.exe does, with limits and a stop report.
 *
 * Compiled into the harness only when lift/harness/two/build_two_native.sh runs with FRUITY=1 (a4-multi.sh does it for FLE_CALL_FRUITY=1);
 * main_two.c calls these functions under #ifdef FLE_FRUITY. A build without FRUITY=1 contains none of this, so its objects are the ones
 * of the harness before this file existed. Native only (wasm is not supported: stage 2 estimates it).
 *
 * Run time (a build with FRUITY=1): FLE_CALL_FRUITY=1 turns the phase on; unset, empty or 0 = the run is the one of a build without it.
 *   FLE_FRUITY_MAX_CALLS=<n>   lifted-function entries allowed after the call began (default 20000000, the harness limit)
 *   FLE_FRUITY_SECONDS=<s>     wall time allowed after the call began (default 60, 1..3600)
 *   FLE_FRUITY_STOP_AT=<list>  comma-separated API names ("CreateWindowExW" or "user32.dll!CreateWindowExW"): stop BEFORE the answer
 *                              of the first lifted `jmp [IAT]` thunk of one of them runs (a named stop; the same report)
 *   FLE_FRUITY_TRAP=1          after the stop report raise SIGTRAP instead of _exit (run under gdb to read the C stack)
 * Any FLE_FRUITY_* variable without FLE_CALL_FRUITY=1 is refused (exit 2 before the guest runs), as is any other value. */
#ifndef FLE_HARNESS_FRUITY_H
#define FLE_HARNESS_FRUITY_H

#include <stdint.h>

/* A mapped image (FLEngine first, then every real module): the stop report may read thunk bytes there. Always inert. */
void fruity_add_image(uint64_t base, uint64_t size);
/* Before the guest runs: reads the variables, checks that the dispatch table holds the three stub rows. 0 = go on (also when the phase is
 * off), -1 = refused (message on stderr; the caller exits 2). */
int fruity_precheck(void);
/* 1 when FLE_CALL_FRUITY=1 (valid only after fruity_precheck returned 0). */
int fruity_requested(void);
/* After DllMain returned and its report was printed. entry_sp = R[4] before the entry call. Returns the exit code of the harness
 * (0 = CreateFruityInstance returned); a stop ends the process itself. */
int fruity_run(uint64_t entry_sp);

#endif
