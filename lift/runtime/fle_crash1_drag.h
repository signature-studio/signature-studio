/* Decision 0209 (crash1): shell32's file drag-and-drop family: DragAcceptFiles, DragQueryFileW, DragQueryPoint, DragFinish.
 *
 * Learn (read 2026-11-04):
 *  - DragAcceptFiles (shellapi/nf-shellapi-dragacceptfiles) void f(HWND hWnd, BOOL fAccept): "Registers whether a window accepts dropped files"; fAccept TRUE "accepts dropped files", FALSE "to discontinue accepting
 *    dropped files"; "An application that calls DragAcceptFiles with the fAccept parameter set to TRUE has identified itself as able to process the WM_DROPFILES message". No return value, no error. The window
 *    records it in the extended style WS_EX_ACCEPTFILES 0x10 ("The window accepts drag-drop files", Extended Window Styles), which is how Windows' shell32 does it (reading of ReactOS and Wine, not of a Microsoft page).
 *  - DragQueryFileW (nf-shellapi-dragqueryfilew) UINT f(HDROP hDrop, UINT iFile, LPWSTR lpszFile, UINT cch): iFile 0xFFFFFFFF returns the number of files (iFile "returns unchanged"); an index below the number copies the
 *    name, NUL-terminated, to lpszFile and returns the characters copied without the NUL; lpszFile NULL returns the required size in characters without the NUL; "A nonzero value indicates a successful call".
 *  - DragQueryPoint (nf-shellapi-dragquerypoint) BOOL f(HDROP hDrop, POINT *ppt): fills ppt with "the coordinates of the mouse pointer at the time the file was dropped"; TRUE "if the drop occurred in the client area of
 *    the window; otherwise FALSE" (DROPFILES.fNC: "If this member is TRUE, pt specifies the screen coordinates of a point in a window's nonclient area. If it is FALSE, pt specifies the client coordinates").
 *  - DragFinish (nf-shellapi-dragfinish) void f(HDROP hDrop): "Releases memory that the system allocated for use in transferring file names to the application".
 *  - DROPFILES (shlobj_core/ns-shlobj_core-dropfiles) { DWORD pFiles; POINT pt; BOOL fNC; BOOL fWide; } followed, pFiles bytes from its start, by "a double null-terminated list of file names"; fWide zero = ANSI.
 *
 * Nothing in this environment drops a file on a window yet (WM_DROPFILES is never posted), so no HDROP exists unless a host side makes one: fle_drop_register() takes the guest address of a DROPFILES block
 * (the HDROP) and its owner's release function. The page does not say what the functions do with a handle they were not given; this implementation answers 0 / FALSE and changes nothing (a reading of Wine and ReactOS,
 * which lock the handle and fail), and DragFinish of such a handle does nothing. UNOFFICIAL where it says so.
 */
#ifndef FLE_CRASH1_DRAG_H
#define FLE_CRASH1_DRAG_H

#include <stdint.h>

#define FLE_WS_EX_ACCEPTFILES 0x00000010u

/* Makes `hdrop` (the guest address of a DROPFILES block) a handle that the four functions accept. release(hdrop) is called by DragFinish (may be NULL). Returns 1, or 0 when 32 are registered already. */
int fle_drop_register(uint64_t hdrop, void (*release)(uint64_t hdrop));
/* 1 when `hdrop` is registered and not finished. */
int fle_drop_is_live(uint64_t hdrop);

/* shell32!DragAcceptFiles: void f(HWND rcx, BOOL edx). */
void fle_imp_DragAcceptFiles(void);
/* shell32!DragQueryFileW: UINT f(HDROP rcx, UINT edx, LPWSTR r8, UINT r9d). */
void fle_imp_DragQueryFileW(void);
/* shell32!DragQueryPoint: BOOL f(HDROP rcx, POINT *rdx). */
void fle_imp_DragQueryPoint(void);
/* shell32!DragFinish: void f(HDROP rcx). */
void fle_imp_DragFinish(void);

#endif
