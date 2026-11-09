/* Walls that File -> Export -> Wave file reached (decision 0188, PROVISIONAL). See fle_wall_exp1.h. */
#include <stdint.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_wall_exp1.h"

#define ES_SYSTEM_REQUIRED 0x00000001u
#define ES_DISPLAY_REQUIRED 0x00000002u
#define ES_USER_PRESENT 0x00000004u
#define ES_AWAYMODE_REQUIRED 0x00000040u
#define ES_CONTINUOUS 0x80000000u

/* SetThreadExecutionState (Learn, winbase.h): 'If the function succeeds, the return value is the previous thread execution state. If the function fails, the return value is NULL.' ES_USER_PRESENT 'is not
 * supported. If ES_USER_PRESENT is combined with other esFlags values, the call will fail and none of the specified states will be set.' ES_CONTINUOUS: 'the state being set should remain in effect until the
 * next call that uses ES_CONTINUOUS and one of the other state flags is cleared.' 'Calling SetThreadExecutionState without ES_CONTINUOUS simply resets the idle timer' (no state change). AWAYMODE 'must be
 * specified with ES_CONTINUOUS'. The model keeps one state for the process (the page says 'thread'; the engine's calls come from one thread, S-exp1-ste-thread) with the initial value ES_CONTINUOUS (the page
 * does not give the state of a thread that never called, and NULL means failure: S-exp1-ste-initial). A flag outside the five the page lists stops by name. Nothing here keeps a machine awake. */
static uint32_t ste_state = ES_CONTINUOUS;

void fle_imp_SetThreadExecutionState(void)
{
    static const char api[] = "SetThreadExecutionState";
    uint32_t fl = (uint32_t)R[1], prev = ste_state;

    if (fl & ~(ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED | ES_USER_PRESENT | ES_AWAYMODE_REQUIRED | ES_CONTINUOUS)) fle_stop(api, "a flag outside ES_SYSTEM_REQUIRED / ES_DISPLAY_REQUIRED / ES_USER_PRESENT / ES_AWAYMODE_REQUIRED / ES_CONTINUOUS: the page names no result");
    if ((fl & ES_USER_PRESENT) && (fl & ~ES_USER_PRESENT)) { R[0] = 0; return; }
    if ((fl & ES_AWAYMODE_REQUIRED) && !(fl & ES_CONTINUOUS)) fle_stop(api, "ES_AWAYMODE_REQUIRED without ES_CONTINUOUS: the page says it must be specified with it and names no result");
    if (fl & ES_CONTINUOUS) ste_state = fl;
    R[0] = prev;
}
