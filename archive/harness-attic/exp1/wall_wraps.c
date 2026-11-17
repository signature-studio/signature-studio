/* EXPLORATION ONLY (decision 0188, PROVISIONAL): a thunk of FLEngine for an import that has no row (its generated body is `fle_unsupported(va)`) is replaced at LINK time by a call of the runtime function that a
 * row would reach, so that a wall can be passed without regenerating the 215 MB of generated C. Link with -Wl,--wrap=f_<va> for each function listed here. This is NOT how a build is shipped: the shipped
 * form is the row in harness/seeds/exp1-imports.txt (the thunk then calls fle_imp_X itself). A call to the thunk from inside the translation unit that defines it is not redirected by --wrap (the stop
 * `fle: unsupported instruction at <va>` then still appears: that is how such a case shows). */
#include "fle_runtime.h"
#include "fle_wall_exp1.h"

void __wrap_f_65c560(void) { fle_imp_SetThreadExecutionState(); }   /* kernel32!SetThreadExecutionState */
