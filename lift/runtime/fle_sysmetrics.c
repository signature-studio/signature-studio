/* user32!GetSystemMetrics (docs/decisions/0057-virtual-system-metrics.md; the page read is listed there and in
 * docs/investigations/getsystemmetrics-caller.md). Stateless. Twenty-one indexes are answered (fourteen of decision 0057, SM_DIGITIZER of
 * decision 0098, SM_CXCURSOR and SM_CYCURSOR of decision 0106, four of decision 0107); the others the guest uses are named and stop, and
 * every other index stops. Nothing here reads the host: no display, no mouse, no DPI setting, no theme, no window, no clock, no file. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_sysmetrics.h"

/* Indexes that the guest's call sites pass (a literal in `mov ecx, imm`, or a value of the two scroll bar tables the guest
 * reads) and for which no value is decided: the page does not give a value for any of them, and the Windows 10 values are not
 * in a primary source. The names are the page's. */
static const struct {
    uint32_t idx;
    const char *name;
} undecided[] = {
    { 2, "SM_CXVSCROLL" }, { 3, "SM_CYHSCROLL" }, { 6, "SM_CYBORDER" }, { 7, "SM_CXFIXEDFRAME" }, { 8, "SM_CYFIXEDFRAME" },
    { 10, "SM_CXHTHUMB" },
    { 15, "SM_CYMENU" }, { 20, "SM_CYVSCROLL" }, { 21, "SM_CXHSCROLL" }, { 32, "SM_CXSIZEFRAME" }, { 45, "SM_CXEDGE" },
    { 46, "SM_CYEDGE" }, { 61, "SM_CXMAXIMIZED" }, { 68, "SM_CXDRAG" },
    { 69, "SM_CYDRAG" },
};

/* user32!GetSystemMetrics: int f(int nIndex ecx). Learn (GetSystemMetrics): the return value is the requested system metric or
 * configuration setting; on failure 0, and GetLastError gives no extended error information (so the last error is left alone).
 * Answered:
 *  - SM_CXSCREEN, SM_CYSCREEN: "the same value obtained by calling GetDeviceCaps as follows: GetDeviceCaps(hdcPrimaryMonitor,
 *    HORZRES / VERTRES)": the virtual display of decision 0053 (fle_gdc.h);
 *  - SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN: the bounding rectangle of all monitors; one
 *    monitor that holds the origin (0,0) (Learn, The Virtual Screen): 0, 0, the screen width, the screen height;
 *  - SM_CMONITORS: 1 (single virtual display);
 *  - SM_CXICON, SM_CYICON, SM_CXSMICON, SM_CYSMICON: 32, 32, 16, 16 (Learn "About Icons", system icon sizes at 96 DPI);
 *  - SM_CXCURSOR, SM_CYCURSOR: 32, 32 (decision 0106, PROVISIONAL; Learn "About Cursors", Cursor Sizes: 32x32 below 144 DPI at the default cursor size);
 *  - SM_MOUSEPRESENT, SM_MOUSEWHEELPRESENT: 1 each (provisional choices, decision 0057);
 *  - SM_SERVERR2: 0 (this environment is Windows 10, not Windows Server 2003 R2).
 *  - SM_DIGITIZER: 0 (decision 0098, PROVISIONAL: no touch or pen digitizer; the page defines the value as a bitmask of the digitizer types and
 *    as 0 "otherwise").
 *  - SM_TABLETPC 0, SM_MAXIMUMTOUCHES 0, SM_CONVERTIBLESLATEMODE 1, SM_SYSTEMDOCKED 0 (decision 0107, PROVISIONAL; the reasons are with the
 *    values in fle_sysmetrics.h).
 * Only the low DWORD of rcx is the index (an int). */
void fle_imp_GetSystemMetrics(void)
{
    static const char api[] = "GetSystemMetrics";
    uint32_t idx = (uint32_t)R[1];
    char why[200];
    unsigned i;

    switch (idx) {
    case FLE_SM_IDX_CXSCREEN: R[0] = (uint32_t)FLE_GDC_HORZRES; return;
    case FLE_SM_IDX_CYSCREEN: R[0] = (uint32_t)FLE_GDC_VERTRES; return;
    case FLE_SM_IDX_CXICON: R[0] = FLE_SM_CXICON; return;
    case FLE_SM_IDX_CYICON: R[0] = FLE_SM_CYICON; return;
    case FLE_SM_IDX_CXCURSOR: R[0] = FLE_SM_CXCURSOR; return;
    case FLE_SM_IDX_CYCURSOR: R[0] = FLE_SM_CYCURSOR; return;
    case FLE_SM_IDX_CXSMICON: R[0] = FLE_SM_CXSMICON; return;
    case FLE_SM_IDX_CYSMICON: R[0] = FLE_SM_CYSMICON; return;
    case FLE_SM_IDX_MOUSEPRESENT: R[0] = FLE_SM_MOUSEPRESENT; return;
    case FLE_SM_IDX_MOUSEWHEELPRESENT: R[0] = FLE_SM_MOUSEWHEELPRESENT; return;
    case FLE_SM_IDX_XVIRTUALSCREEN: R[0] = FLE_SM_VIRTUALSCREEN_ORIGIN; return;
    case FLE_SM_IDX_YVIRTUALSCREEN: R[0] = FLE_SM_VIRTUALSCREEN_ORIGIN; return;
    case FLE_SM_IDX_CXVIRTUALSCREEN: R[0] = (uint32_t)FLE_GDC_HORZRES; return;
    case FLE_SM_IDX_CYVIRTUALSCREEN: R[0] = (uint32_t)FLE_GDC_VERTRES; return;
    case FLE_SM_IDX_CMONITORS: R[0] = FLE_SM_CMONITORS; return;
    case FLE_SM_IDX_SERVERR2: R[0] = FLE_SM_SERVERR2; return;
    case FLE_SM_IDX_DIGITIZER: R[0] = FLE_SM_DIGITIZER; return;
    case FLE_SM_IDX_TABLETPC: R[0] = FLE_SM_TABLETPC; return;
    case FLE_SM_IDX_MAXIMUMTOUCHES: R[0] = FLE_SM_MAXIMUMTOUCHES; return;
    case FLE_SM_IDX_CONVERTIBLESLATEMODE: R[0] = FLE_SM_CONVERTIBLESLATEMODE; return;
    case FLE_SM_IDX_SYSTEMDOCKED: R[0] = FLE_SM_SYSTEMDOCKED; return;
    default: break;
    }
    for (i = 0; i < sizeof undecided / sizeof undecided[0]; i++) {
        if (undecided[i].idx != idx) continue;
        snprintf(why, sizeof why, "index %u (%s) is used by the guest but no value is decided (decision 0057)", (unsigned)idx, undecided[i].name);
        fle_stop(api, why);
    }
    snprintf(why, sizeof why, "index %u is not answered (the guest's call sites do not use it)", (unsigned)idx);
    fle_stop(api, why);
}
