/* Census (docs/decisions/0097-census-diagnostic.md): a diagnostic run of the whole unit table of FLEngine_x64.dll. Environment variable
 * FLE_CENSUS=<file> turns it on; without it every function below does nothing and the harness behaves as before (a4 numbers unchanged).
 *
 * A unit that stops by name (abort), faults (SIGSEGV, SIGBUS, SIGILL, SIGFPE) or runs away (wall time or function-entry limit of
 * the unit) is ABANDONED: the guest registers and the x87/MMX/MXCSR state are put back to their values at the call of the unit, rax
 * is 0, and the guest loop f_4120c0 goes on with the next unit. State that the unit changed in memory and in the runtime tables
 * is not undone. THE RESULT IS NOT A4 PROGRESS (decision 0046, 0060): an abandoned unit is not completed. */
#ifndef FLE_HARNESS_CENSUS_H
#define FLE_HARNESS_CENSUS_H

#include <stddef.h>
#include <stdint.h>

/* Called once after the image is mapped and before the entry is called. Returns 1 when FLE_CENSUS is set (census mode), else 0. */
int census_begin(uint64_t img_base, uint64_t img_size);
/* Another mapped image (a real module): its memory may be read for the report. Census mode only. */
void census_add_image(uint64_t base, uint64_t size);
/* Called when the entry has returned (and by the signal chain for a stop outside every unit). Writes the rest of the file. */
void census_end(const char *how);

/* Provided by harness/wasm/trace_native.c (the shadow call stack and the runtime message). */
const char *trace_full_message(void);
void trace_message_reset(void);
size_t trace_depth(void);
void trace_set_depth(size_t d);
void trace_set_call_limit(uint64_t n);
int trace_frame(size_t k, uint64_t *va, uint64_t *ret);
extern void (*trace_limit_hook)(void);

#endif
