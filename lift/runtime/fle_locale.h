/* Emulated locale and the kernel32 NLS answers (docs/decisions/0016-locale.md). As in fle_imports.h, each
 * stub reads its arguments from R[] (Windows x64 ABI; stack arguments at R[4] + 0x28 and up, because the
 * guest return address is still on the stack), sets the Win32 return value in R[0] and leaves the guest
 * return address for the caller's ret. A case that the MSDN pages leave open, or that needs data of a
 * locale other than the emulated one, stops the run (abort) instead of inventing a value. */
#ifndef FLE_LOCALE_H
#define FLE_LOCALE_H

#include <stdint.h>

/* The one place that holds the emulated locale. These are settings of this lift environment, not values
 * given by the specification: the MSDN pages leave the default locale, the ANSI code page and the OEM
 * code page to the system. Nothing else in the runtime hard-codes them. */
typedef struct {
    uint32_t lcid;  /* user default = system default = initial thread locale, UI language */
    uint32_t acp;   /* system ANSI code page (GetACP, CP_ACP, CP_THREAD_ACP) */
    uint32_t oemcp; /* system OEM code page (CP_OEMCP) */
} fle_locale_t;

extern const fle_locale_t fle_emulated_locale;

/* kernel32!SetThreadLocale: BOOL SetThreadLocale(LCID). ecx. */
void fle_imp_SetThreadLocale(void);
/* kernel32!GetThreadLocale: LCID GetThreadLocale(void). */
void fle_imp_GetThreadLocale(void);
/* kernel32!IsValidLocale: BOOL IsValidLocale(LCID, DWORD dwFlags). ecx, edx. */
void fle_imp_IsValidLocale(void);
/* kernel32!GetUserDefaultUILanguage / GetSystemDefaultUILanguage: LANGID f(void). */
void fle_imp_GetUserDefaultUILanguage(void);
void fle_imp_GetSystemDefaultUILanguage(void);
/* kernel32!GetACP: UINT GetACP(void). */
void fle_imp_GetACP(void);
/* kernel32!GetOEMCP: UINT GetOEMCP(void). kernel32!IsValidCodePage: BOOL f(UINT ecx). Only the emulated ANSI and OEM code pages are
 * valid; any other identifier stops the run (decision 0078). */
void fle_imp_GetOEMCP(void);
void fle_imp_IsValidCodePage(void);
/* kernel32!GetStringTypeW: BOOL f(DWORD dwInfoType, LPCWCH, int cchSrc, LPWORD). CT_CTYPE1 for the range D of decision 0087 only (Wine's values,
 * unofficial); anything else stops by name. */
void fle_imp_GetStringTypeW(void);
/* kernel32!IsDBCSLeadByteEx: BOOL IsDBCSLeadByteEx(UINT CodePage, BYTE TestChar). ecx, dl. */
void fle_imp_IsDBCSLeadByteEx(void);
/* kernel32!GetCPInfo: BOOL GetCPInfo(UINT CodePage, LPCPINFO). ecx, rdx. */
void fle_imp_GetCPInfo(void);
/* kernel32!GetLocaleInfoW: int GetLocaleInfoW(LCID, LCTYPE, LPWSTR, int). ecx, edx, r8, r9d. */
void fle_imp_GetLocaleInfoW(void);
/* kernel32!LCMapStringW: int LCMapStringW(LCID, DWORD, LPCWSTR, int, LPWSTR, int). */
void fle_imp_LCMapStringW(void);
/* kernel32!CompareStringW: int CompareStringW(LCID, DWORD, PCNZWCH, int, PCNZWCH, int). */
void fle_imp_CompareStringW(void);
/* kernel32!MultiByteToWideChar: int f(UINT, DWORD, LPCCH, int, LPWSTR, int). */
void fle_imp_MultiByteToWideChar(void);
/* kernel32!WideCharToMultiByte: int f(UINT, DWORD, LPCWCH, int, LPSTR, int, LPCCH, LPBOOL). */
void fle_imp_WideCharToMultiByte(void);

/* kernel32!GetSystemDefaultLocaleName: int f(LPWSTR lpLocaleName, int cchLocaleName). rcx, edx. Decision 0099 (provisional): the LOCALE_SNAME value of the
 * emulated locale ("en-US", 6 characters with the NUL); a NULL or too small buffer is 0 and ERROR_INSUFFICIENT_BUFFER; a negative size stops. */
void fle_imp_GetSystemDefaultLocaleName(void);
/* kernel32!LocaleNameToLCID: LCID f(LPCWSTR lpName rcx, DWORD dwFlags edx) (decision 0112, PROVISIONAL). NULL, the system-default name, the emulated locale's name and the
 * empty name are answered; any other name stops by name. */
void fle_imp_LocaleNameToLCID(void);

#endif
