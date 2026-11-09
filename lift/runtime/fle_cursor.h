/* The virtual standard cursor: user32!LoadCursorW(NULL, IDC_ARROW) (docs/decisions/0059-virtual-cursors.md; the pages read and
 * the source of every number are listed there and in docs/investigations/loadcursor-caller.md). Same convention as
 * fle_imports.h: arguments in R[1] (rcx), R[2] (rdx); the Win32 return value in R[0]; the guest return address stays on the
 * stack for the caller's ret. Stateless: a handle is a pure function of the IDC_ identifier, so there is no thread rule.
 *
 * Nothing is read from the host: no cursor, no theme, no mouse settings, no display, no module resource. The handle value is a
 * value of this lift environment (no page gives one), fixed, so that a run does not depend on the machine it runs on.
 *
 * Only the form the guest uses is answered: hInstance NULL and lpCursorName IDC_ARROW. Every other argument (a non-NULL
 * hInstance, a string name, another identifier) stops the run through fle_stop (fle_env.h): "fle: LoadCursorW: <why>" and
 * abort(). A value is never made up for it. */
#ifndef FLE_CURSOR_H
#define FLE_CURSOR_H

#include <stdint.h>

/* Learn "About Cursors": IDC_ARROW is MAKEINTRESOURCE(32512) (OFFICIAL). It is the only identifier answered. */
#define FLE_CURSOR_IDC_ARROW 32512

/* Handles of the virtual standard cursors (decision 0059, a PROVISIONAL choice): FLE_CURSOR_HANDLE_BASE + 4 * (idc - 32512 + 1),
 * so a given IDC_ identifier has the same handle on every call and the handle does not depend on the order of the calls. The
 * pages give no value for a cursor handle. Learn "About Cursors" says that HCURSOR and HICON refer to the same underlying
 * object type, and Learn "Interprocess communication" (64-bit Windows programming) says that only the lower 32 bits of a
 * handle are significant for the handle kinds it lists (windows, GDI pens and brushes; cursors are not listed), so the
 * values stay below 0x80000000 (truncation and sign extension give the same 64-bit value). The region lies directly after the
 * one of the virtual GDI objects (fle_gdi.h: 0x7FBA0000 + 1 MiB) and does not overlap it, nor the one of the virtual screen
 * DCs (fle_gdc.h: 0x7FA90000 + 1 MiB). */
#define FLE_CURSOR_HANDLE_BASE UINT64_C(0x7FCA0000)
#define FLE_CURSOR_REGION_SIZE UINT64_C(0x00100000)

/* The other twelve standard cursors that the guest loads through user32!LoadImageW(NULL, IDC_*, IMAGE_CURSOR, 32, 32,
 * LR_SHARED) (decision 0061; fle_loadimage.c). Learn "About Cursors" (OFFICIAL) gives the numbers. LoadCursorW still answers
 * IDC_ARROW only (decision 0059): the guest calls LoadCursorW for no other identifier. */
#define FLE_CURSOR_IDC_IBEAM       32513
#define FLE_CURSOR_IDC_WAIT        32514
#define FLE_CURSOR_IDC_CROSS       32515
#define FLE_CURSOR_IDC_UPARROW     32516
#define FLE_CURSOR_IDC_SIZENWSE    32642
#define FLE_CURSOR_IDC_SIZENESW    32643
#define FLE_CURSOR_IDC_SIZEWE      32644
#define FLE_CURSOR_IDC_SIZENS      32645
#define FLE_CURSOR_IDC_SIZEALL     32646
#define FLE_CURSOR_IDC_NO          32648
#define FLE_CURSOR_IDC_APPSTARTING 32650
#define FLE_CURSOR_IDC_HELP        32651

/* user32!LoadCursorW: HCURSOR f(HINSTANCE hInstance, LPCWSTR lpCursorName). rcx, rdx. */
void fle_imp_LoadCursorW(void);

/* user32!GetClipCursor (decision 0098, PROVISIONAL): BOOL f(LPRECT lpRect). rcx. This environment never confines the cursor (ClipCursor is not
 * answered and the guest does not import it), so the RECT receives "the dimensions of the screen" (Learn): the virtual display of decision 0053
 * with the origin of decision 0057, the same four numbers as the monitor rectangle of decision 0065 (fle_monitor.h). Returns 1. A NULL lpRect
 * stops by name (the page defines no failure for it). */
void fle_imp_GetClipCursor(void);

/* Inspection (for tests and tools; pure, none of them moves any state). The handle that LoadCursorW answers for an IDC_
 * identifier (0 when that identifier is not answered), and the identifier of a handle that LoadCursorW answers (0 when h is
 * not such a handle). */
uint64_t fle_cursor_handle_of(uint32_t idc);
uint32_t fle_cursor_idc_of(uint64_t h);

/* Decision 0113 wall 8 (pure): 1 when h is a shared cursor of the virtual cursor tables: one of the standard cursors (fle_cursor_image_idc_of: LoadCursorW, LoadImageW with a
 * NULL hInst), or a handle of the form LoadImageW(hInst, id, IMAGE_CURSOR, 32, 32, LR_SHARED) answers for a module's RT_GROUP_CURSOR id: 0x7FDA0000 + 0x1000 * k +
 * 4 * (id - 32762 + 1) with k = 0 (the lifted DLL) or 1..8 (a real module) and id in 32762 ..= 32767 (the constants of the header of the module that makes them). LoadImageW is
 * stateless (a handle is a function of the arguments), so this says "has the form", not "was issued". Else 0. */
int fle_cursor_is_shared_handle(uint64_t h);

/* The same for the thirteen identifiers that LoadImageW(NULL, ...) answers (IDC_ARROW and the twelve above; 0 for any other
 * identifier, IDC_HAND included: the guest loads IDC_HAND only from its own module). The handle of an identifier is the one
 * formula of fle_cursor_handle_of, so IDC_ARROW has the same handle in LoadCursorW and in LoadImageW (decision 0061). */
uint64_t fle_cursor_image_handle_of(uint32_t idc);
uint32_t fle_cursor_image_idc_of(uint64_t h);

#endif
