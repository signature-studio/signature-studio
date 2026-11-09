/* A4 survey (docs/investigations/a4-survey-tool.md, decision 0060): at the stop report, read InitContext
 * (VA 0x15EAC90: +0 PackageUnitHeader*, +8 u32 InitCount) and the unit table from the guest image and print how far
 * DllMain -> InitUnits (f_4120c0) got. Harness only: nothing is added to the generated C. */
#ifndef FLE_HARNESS_A4_H
#define FLE_HARNESS_A4_H

#include <stdint.h>

/* The mapped image (call once after image_load). Before this, a4_report prints a one-line note and reads nothing. */
void a4_set_image(uint64_t base, uint64_t size);
void a4_report(void);
/* Decision 0113: called by the trace hook (native) at every lifted function entry; see a4.c. */
void a4_sample(void);

/* Provided by trace.c (the -finstrument-functions entry hook): was `fn` entered, is `fn` on the shadow call stack now. */
int trace_fn_seen(void *fn);
int trace_fn_on_stack(void *fn);
uint64_t trace_distinct_count(void);
uint64_t trace_call_count(void);

#endif
