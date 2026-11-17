/* The virtual clipboard (decision 0171, wall ia1-W1: user32!IsClipboardFormatAvailable at the first mouse click on the main window).
 *
 * One process, one clipboard, no host clipboard: the clipboard of this environment starts EMPTY (no format) and holds what the guest puts there with
 * SetClipboardData. Nothing is read from or written to the host's clipboard. Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx),
 * R[8] (r8), R[9] (r9); the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret.
 *
 * Rules are from Microsoft Learn (OpenClipboard, CloseClipboard, EmptyClipboard, IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats,
 * GetClipboardData, SetClipboardData, Standard Clipboard Formats / Synthesized Clipboard Formats); what the pages leave open is a choice named S-ia1-clip-N in
 * docs/decisions/0171-user-operation-walls.md. */
#ifndef FLE_CLIP_H
#define FLE_CLIP_H

#include <stdint.h>

/* user32!OpenClipboard: BOOL f(HWND hWndNewOwner rcx). CloseClipboard: BOOL f(void). EmptyClipboard: BOOL f(void).
 * IsClipboardFormatAvailable: BOOL f(UINT format ecx). CountClipboardFormats: int f(void). EnumClipboardFormats: UINT f(UINT format ecx).
 * GetClipboardData: HANDLE f(UINT uFormat ecx). SetClipboardData: HANDLE f(UINT uFormat ecx, HANDLE hMem rdx). */
void fle_imp_OpenClipboard(void);
void fle_imp_CloseClipboard(void);
void fle_imp_EmptyClipboard(void);
void fle_imp_IsClipboardFormatAvailable(void);
void fle_imp_CountClipboardFormats(void);
void fle_imp_EnumClipboardFormats(void);
void fle_imp_GetClipboardData(void);
void fle_imp_SetClipboardData(void);

/* ole32!OleSetClipboard: HRESULT f(IDataObject *rcx). OleIsCurrentClipboard: HRESULT f(IDataObject *rcx). OleFlushClipboard: HRESULT f(void). */
void fle_imp_OleSetClipboard(void);
void fle_imp_OleIsCurrentClipboard(void);
void fle_imp_OleFlushClipboard(void);
uint64_t fle_clip_ole_object(void);

/* Inspection for tests and diagnostics: number of formats held, the format at index i (0 when out of range), whether the clipboard is open. */
unsigned fle_clip_count(void);
uint32_t fle_clip_format(unsigned i);
int fle_clip_is_open(void);

#endif
