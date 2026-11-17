/* Decision 0136 (PROVISIONAL): comctl32!_TrackMouseEvent (the wall of decision 0128 at WM_MOUSEMOVE). The tracking state and the messages are in fle_input.c; this file reads and writes the TRACKMOUSEEVENT.
 * Learn (commctrl/nf-commctrl-_trackmouseevent): "Posts messages when the mouse pointer leaves a window or hovers over a window for a specified amount of time. This function calls TrackMouseEvent if it exists,
 * otherwise it emulates it"; "If the function succeeds, the return value is nonzero. If the function fails, return value is zero" (winuser TrackMouseEvent: "To get extended error information, call GetLastError").
 * TRACKMOUSEEVENT { DWORD cbSize; DWORD dwFlags; HWND hwndTrack; DWORD dwHoverTime } = 24 bytes on x64 (4 + 4 + 8 + 4 + 4 padding). */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_win.h"
#include "fle_input.h"

#define TME_SIZE 24u
#define TME_VALID (FLE_TME_HOVER | FLE_TME_LEAVE | FLE_TME_NONCLIENT | FLE_TME_QUERY | FLE_TME_CANCEL)

void fle_imp__TrackMouseEvent(void)
{
    static const char api[] = "_TrackMouseEvent";
    uint64_t p = R[1];
    uint32_t cb, flags, hover;
    uint64_t h;
    fle_input_track_t q;

    if (p == 0) fle_stop(api, "lpEventTrack is NULL: the page gives no behaviour");
    cb = (uint32_t)fle_ld(p, 4);
    flags = (uint32_t)fle_ld(p + 4, 4);
    h = fle_ld(p + 8, 8);
    hover = (uint32_t)fle_ld(p + 16, 4);
    /* S-tme-invalid: the pages name no failure value beyond "zero"; a cbSize that is not the structure size and a flag that is not one of the five documented fail with ERROR_INVALID_PARAMETER (87), an
     * hwndTrack that is not a live window with ERROR_INVALID_WINDOW_HANDLE (1400) (the codes are choices). */
    if (cb != TME_SIZE || (flags & ~TME_VALID) != 0) {
        fle_set_last_error(FLE_W32_ERR_INVALID_PARAMETER);
        R[0] = 0;
        return;
    }
    if (!fle_input_track(flags, h, hover, &q)) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE);
        R[0] = 0;
        return;
    }
    if (flags & FLE_TME_QUERY) {
        fle_st(p + 4, 4, q.flags);
        fle_st(p + 8, 8, q.hwnd);
        fle_st(p + 16, 4, q.hover_ms);
    }
    R[0] = 1;
}
