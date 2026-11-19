/* Walls that opening a project through File -> Open... reached (decision 0178, PROVISIONAL). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0].
 * Each function is reached only through a row of harness/seeds/dlg1-imports.txt (an earlier build has no row and stops at the import thunk as before). */
#ifndef FLE_WALL_DLG1_H
#define FLE_WALL_DLG1_H

/* mpr.dll!WNetGetUniversalNameW: DWORD f(LPCWSTR lpLocalPath rcx, DWORD dwInfoLevel rdx, LPVOID lpBuffer r8, LPDWORD lpBufferSize r9). */
void fle_imp_WNetGetUniversalNameW(void);

/* kernel32!GetFileInformationByHandle: BOOL f(HANDLE hFile rcx, LPBY_HANDLE_FILE_INFORMATION lpFileInformation rdx).
 * kernel32!SetEndOfFile: BOOL f(HANDLE hFile rcx). */
void fle_imp_GetFileInformationByHandle(void);
void fle_imp_SetEndOfFile(void);

/* EXPLORATION ONLY (rows in harness/seeds/dlg1-explore-imports.txt, never in dlg1-imports.txt): msacm32.dll!acmDriverEnum: MMRESULT f(ACMDRIVERENUMCB fnCallback rcx, DWORD_PTR dwInstance rdx, DWORD fdwEnum r8d). */
void fle_imp_acmDriverEnum(void);
/* msacm32.dll!acmStreamOpen: MMRESULT f(LPHACMSTREAM phas rcx, HACMDRIVER had rdx, LPWAVEFORMATEX pwfxSrc r8, LPWAVEFORMATEX pwfxDst r9, LPWAVEFILTER pwfltr, DWORD_PTR dwCallback, DWORD_PTR dwInstance, DWORD fdwOpen). */
void fle_imp_acmStreamOpen(void);
/* msacm32.dll!acmFormatSuggest: MMRESULT f(HACMDRIVER had rcx, LPWAVEFORMATEX pwfxSrc rdx, LPWAVEFORMATEX pwfxDst r8, DWORD cbwfxDst r9, DWORD fdwSuggest). */
void fle_imp_acmFormatSuggest(void);
/* msacm32.dll!acmDriverClose: MMRESULT f(HACMDRIVER had rcx, DWORD fdwClose edx). */
void fle_imp_acmDriverClose(void);

#endif
