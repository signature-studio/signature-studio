/* user32 answers that need no window, message queue or GDI object (docs/decisions/0030-apis-a9-a10.md; the pages
 * read are listed there). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8),
 * R[9] (r9), stack arguments through fle_arg (fle_env.h); the Win32 return value in R[0]; the guest return
 * address stays on the stack for the caller's ret.
 *
 * Nothing is read from the host. A case that the Microsoft pages leave open stops the run through fle_stop
 * (fle_env.h): "fle: <api>: <why>" and abort(). Single guest thread only (no locking).
 *
 * Case mapping is ASCII only, like fle_w_fold (fle_env.h). A UTF-16 unit above 0x7F in a string that has to be
 * converted, or that CharNextW has to step over, stops the run: the pages say the mapping of other letters is
 * the system's and give no table. The registered-string tables (messages, clipboard formats) are SETTINGS OF
 * THIS LIFT ENVIRONMENT: the first value handed out is FLE_UNW_REG_FIRST, the next one is +1; the pages only
 * give the range 0xC000-0xFFFF. */
#ifndef FLE_USER_NOWIN_H
#define FLE_USER_NOWIN_H

#include <stdint.h>

/* MSDN RegisterWindowMessageW, RegisterClipboardFormatW: "a message identifier in the range 0xC000 through
 * 0xFFFF", "values in the range 0xC000 through 0xFFFF". */
#define FLE_UNW_REG_FIRST  0xC000u
#define FLE_UNW_REG_LAST   0xFFFFu
#define FLE_UNW_REG_MAX    (FLE_UNW_REG_LAST - FLE_UNW_REG_FIRST + 1u)

/* A registered name longer than this many UTF-16 units stops the run (a limit of this runtime, not of the pages). */
#define FLE_UNW_NAME_MAX   255u

/* user32!CharLowerW, CharUpperW: LPWSTR f(LPWSTR). rcx = a NUL-terminated string (converted in place, the
 * pointer is returned) or, when the value is below 0x10000, one character (the converted character is returned).
 * user32!CharLowerBuffW, CharUpperBuffW: DWORD f(LPWSTR, DWORD). rcx, edx. Returns the number of characters
 * processed (edx). user32!CharNextW: LPWSTR f(LPCWSTR). rcx. */
void fle_imp_CharLowerW(void);
void fle_imp_CharUpperW(void);
void fle_imp_CharLowerBuffW(void);
void fle_imp_CharUpperBuffW(void);
void fle_imp_CharNextW(void);

/* user32!SetRect: BOOL f(LPRECT, int, int, int, int). rcx, edx, r8d, r9d, then yBottom on the stack.
 * IntersectRect, UnionRect: BOOL f(LPRECT dst, const RECT *a, const RECT *b). rcx, rdx, r8.
 * OffsetRect: BOOL f(LPRECT, int dx, int dy). rcx, edx, r8d. RECT is four 32-bit LONGs: left, top, right, bottom. */
void fle_imp_SetRect(void);
void fle_imp_IntersectRect(void);
void fle_imp_OffsetRect(void);
void fle_imp_UnionRect(void);
/* Decision 0126 (wall 98): user32!InflateRect: BOOL f(LPRECT, int dx, int dy); EqualRect: BOOL f(const RECT *, const RECT *); PtInRect: BOOL f(const RECT *, POINT by value in rdx); SubtractRect: BOOL f(LPRECT, const RECT *, const RECT *). */
void fle_imp_InflateRect(void);
void fle_imp_EqualRect(void);
void fle_imp_PtInRect(void);
void fle_imp_SubtractRect(void);

/* user32!RegisterWindowMessageW, RegisterClipboardFormatW: UINT f(LPCWSTR). rcx.
 * user32!GetClipboardFormatNameW: int f(UINT format, LPWSTR, int cchMaxCount). ecx, rdx, r8d. */
void fle_imp_RegisterWindowMessageW(void);
void fle_imp_RegisterClipboardFormatW(void);
void fle_imp_GetClipboardFormatNameW(void);

#endif
