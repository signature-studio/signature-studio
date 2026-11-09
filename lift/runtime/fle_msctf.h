/* msctf.dll (Text Services Framework) export that the guest resolves with GetProcAddress
 * (docs/decisions/0055-msctf-loadlibrary.md; the guest's use is in docs/investigations/msctf-caller.md).
 * msctf.dll is a virtual module (fle_modules.c, N = 26): LoadLibraryW("Msctf.dll") gives its handle. The module
 * exports SetInputScope (Learn: inputscope.h, DLL Msctf.dll, Windows XP and later), so GetProcAddress finds it. This
 * runtime has no window table to attach an input scope to, and the page's only result is S_OK, so a call stops the
 * run (fle_stop) instead of returning a made-up HRESULT. Same convention as fle_imports.h: arguments in R[1] (rcx),
 * R[2] (rdx); the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret. */
#ifndef FLE_MSCTF_H
#define FLE_MSCTF_H

/* msctf!SetInputScope: HRESULT SetInputScope(HWND hwnd, InputScope inputscope). Always stops. */
void fle_imp_SetInputScope(void);

#endif
