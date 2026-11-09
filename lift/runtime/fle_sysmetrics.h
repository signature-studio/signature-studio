/* user32!GetSystemMetrics for the indexes the guest asks and for which this environment has a value
 * (docs/decisions/0057-virtual-system-metrics.md; the page read and the source of every number and value are listed there and
 * in docs/investigations/getsystemmetrics-caller.md). Same convention as fle_imports.h: the argument in R[1] (ecx); the Win32
 * return value in R[0]; the guest return address stays on the stack for the caller's ret. Stateless, so there is no thread
 * rule.
 *
 * Nothing is read from the host: no screen, no mouse, no DPI setting, no theme. The values below are values of this lift
 * environment, fixed, so that a run does not depend on the machine it runs on. The screen values are the ones of the virtual
 * display of decision 0053 (fle_gdc.h), by name, so the two cannot drift apart.
 *
 * Every other index stops the run through fle_stop (fle_env.h): "fle: GetSystemMetrics: <why>" and abort(). A value is never
 * made up for it. */
#ifndef FLE_SYSMETRICS_H
#define FLE_SYSMETRICS_H

#include <stdint.h>

/* SM_* index numbers: the Learn page of GetSystemMetrics lists each name with its number (OFFICIAL). */
#define FLE_SM_IDX_CXSCREEN         0
#define FLE_SM_IDX_CYSCREEN         1
#define FLE_SM_IDX_CXICON           11
#define FLE_SM_IDX_CYICON           12
#define FLE_SM_IDX_CXCURSOR         13
#define FLE_SM_IDX_CYCURSOR         14
#define FLE_SM_IDX_MOUSEPRESENT     19
#define FLE_SM_IDX_CXSMICON         49
#define FLE_SM_IDX_CYSMICON         50
#define FLE_SM_IDX_MOUSEWHEELPRESENT 75
#define FLE_SM_IDX_XVIRTUALSCREEN   76
#define FLE_SM_IDX_YVIRTUALSCREEN   77
#define FLE_SM_IDX_CXVIRTUALSCREEN  78
#define FLE_SM_IDX_CYVIRTUALSCREEN  79
#define FLE_SM_IDX_CMONITORS        80
#define FLE_SM_IDX_TABLETPC         86
#define FLE_SM_IDX_SERVERR2         89
#define FLE_SM_IDX_DIGITIZER        94
#define FLE_SM_IDX_MAXIMUMTOUCHES   95
#define FLE_SM_IDX_CONVERTIBLESLATEMODE 0x2003
#define FLE_SM_IDX_SYSTEMDOCKED     0x2004

/* Decided values (decision 0057). CXSCREEN, CYSCREEN, CXVIRTUALSCREEN, CYVIRTUALSCREEN are FLE_GDC_HORZRES and FLE_GDC_VERTRES
 * (fle_gdc.h): the page says SM_CXSCREEN / SM_CYSCREEN equal GetDeviceCaps(HORZRES / VERTRES). */
/* System icon sizes: the Learn page "About Icons" (System icon sizes) lists, at 96 DPI, SM_CXSMICON / SM_CYSMICON 16x16 and
 * SM_CXICON / SM_CYICON 32x32 (OFFICIAL). 96 DPI is the DPI of the virtual display (FLE_GDC_LOGPIXELSX, decision 0053). */
#define FLE_SM_CXICON               32
#define FLE_SM_CYICON               32
#define FLE_SM_CXSMICON             16
#define FLE_SM_CYSMICON             16
/* Decision 0106 (PROVISIONAL): SM_CXCURSOR / SM_CYCURSOR (13, 14), asked by ILTools' own initialisation (CreateCursorFromFont). The Learn page "About
 * Cursors", section "Cursor Sizes" (updated_at 2026-09-21): SM_CXCURSOR / SM_CYCURSOR is "the nominal cursor size", and "the standard sizes at default
 * cursor size setting" are 32x32 below 150% display scale (below 144 DPI). The virtual display is 96 DPI (decision 0053), and the page does not say that
 * a Windows 10 22H2 follows the same table (it names no OS), nor what the "default cursor size setting" of this environment is. */
#define FLE_SM_CXCURSOR             32
#define FLE_SM_CYCURSOR             32
#define FLE_SM_MOUSEPRESENT         1   /* provisional choice: the page says only "rarely zero" */
#define FLE_SM_MOUSEWHEELPRESENT    1   /* provisional choice: the page gives no default */
#define FLE_SM_VIRTUALSCREEN_ORIGIN 0   /* one monitor, which holds the origin (0,0) (Learn: The Virtual Screen) */
#define FLE_SM_CMONITORS            1   /* one monitor (decision 0053: a single virtual display) */
#define FLE_SM_SERVERR2             0   /* page: the build number on Windows Server 2003 R2, otherwise 0; this OS is Windows 10 (0014) */
/* Decision 0098 (PROVISIONAL): SM_DIGITIZER (94). The page: "The return value is a bitmask that specifies the type of digitizer input supported by the
 * device" and, for the cases it defines, "otherwise, 0". This environment has a mouse (SM_MOUSEPRESENT above) and no touch or pen digitizer, so
 * no NID_* bit is set. */
#define FLE_SM_DIGITIZER            0
/* Decision 0107 (PROVISIONAL): the other four indexes that the guest's f_80e360 asks right after SM_DIGITIZER (unit 276), all consistent with
 * "a desktop PC with a mouse and no touch or pen digitizer".
 *  - SM_TABLETPC (86) 0: the page gives nonzero only for "the Windows XP Tablet PC edition" or "Windows Vista or Windows 7 and the Tablet PC
 *    Input service is started", "otherwise, 0"; this environment is Windows 10 (decision 0014), which is neither.
 *  - SM_MAXIMUMTOUCHES (95) 0: "Nonzero if there are digitizers in the system; otherwise, 0"; there is none (SM_DIGITIZER above).
 *  - SM_CONVERTIBLESLATEMODE (0x2003) 1: "0 for Slate Mode and non-zero otherwise"; the page also says the metric "doesn't apply to desktop PCs",
 *    so it gives no value for one. Not slate mode, so nonzero; the Windows-hardware page "Recommended settings for better tablet experiences" says
 *    to set the equivalent OEM value ConvertibleSlateMode to 1 "for other form factors like Desktop, Laptop, or Notebook". The choice 1 against
 *    0 is not decided by a primary source; the guest's branch does not depend on it while SM_DIGITIZER is 0 (decision 0107, reading).
 *  - SM_SYSTEMDOCKED (0x2004) 0: "0 for Undocked Mode and non-zero otherwise"; no docking station is modelled. Not decided by a primary source
 *    for a desktop PC; the guest stores the answer and never reads it (decision 0107, reading). */
#define FLE_SM_TABLETPC             0
#define FLE_SM_MAXIMUMTOUCHES       0
#define FLE_SM_CONVERTIBLESLATEMODE 1
#define FLE_SM_SYSTEMDOCKED         0

/* user32!GetSystemMetrics: int f(int nIndex). rcx (the low DWORD is the index). */
void fle_imp_GetSystemMetrics(void);

#endif
