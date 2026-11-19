/* Walls that the Fruity Wrapper (Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll) reached inside the engine (decision 0183, PROVISIONAL). Same convention as fle_imports.h: arguments in R[1] (rcx),
 * R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]. Each function is reached only through a row of harness/seeds/wrp1-imports.txt (a build without the row stops at the import thunk as before). */
#ifndef FLE_WALL_WRP1_H
#define FLE_WALL_WRP1_H

/* Job objects (Learn: CreateJobObjectW, SetInformationJobObject, CloseHandle). Handles: FLE_JOB_HANDLE_BASE + 4 * (i + 1), never reused (a choice: the page gives no value).
 * kernel32!CreateJobObjectW: HANDLE f(LPSECURITY_ATTRIBUTES rcx, LPCWSTR lpName rdx).
 * kernel32!SetInformationJobObject: BOOL f(HANDLE hJob rcx, JOBOBJECTINFOCLASS class edx, LPVOID info r8, DWORD cb r9d). */
#define FLE_JOB_HANDLE_BASE UINT64_C(0x7FFAC0000000)
#define FLE_JOB_REGION_SIZE UINT64_C(0x10000000)
void fle_imp_CreateJobObjectW(void);
void fle_imp_SetInformationJobObject(void);

/* Number of live job objects (for tests). */
unsigned fle_job_live(void);

#endif
