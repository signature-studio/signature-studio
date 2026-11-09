/* Synthetic host (stands in for FLEngine_x64.dll, ImageBase 0x400000 = FLE_EXE_HANDLE). Each Host* export is one scenario; the
 * driver calls it through fle_call_guest. Results go to the shared log. No CRT. */
#include <windows.h>
#include "shared.h"

typedef int (*binop_t)(int, int);
typedef int (*unop_t)(int);
typedef void *(*getp_t)(void);
typedef int (*runcb_t)(int (*)(int), int);

static FARPROC g_tab[8];
static FARPROC *volatile g_tabp = g_tab;

BOOL WINAPI DllMain(HINSTANCE h, DWORD reason, LPVOID reserved) { return TRUE; }

/* Not exported: reachable only by its address (pdata seed), passed to the other module. */
__attribute__((noinline)) static int HostCb(int x)
{
    LOG('HCB ', x);
    return x * 2;
}

__declspec(dllexport) int HostBasic(void)
{
    HMODULE h;
    FARPROC *t;
    void *sp;
    LOG('HBEF', 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    if (!h) return 1;
    g_tab[0] = W_GetProcAddress(h, "Add");
    g_tab[1] = W_GetProcAddress(h, "Twice");
    g_tab[2] = W_GetProcAddress(h, "GetStoredPtr");
    g_tab[3] = W_GetProcAddress(h, "CallStored");
    g_tab[4] = W_GetProcAddress(h, "NoSuchExport");
    g_tab[5] = W_GetProcAddress(h, "RunCb");
    LOG('HMIS', g_tab[4]);
    LOG('HMER', W_GetLastError());
    LOG('HPAD', g_tab[0]);
    t = g_tabp;
    LOG('HADD', ((binop_t)t[0])(40, 2));
    LOG('HTWI', ((unop_t)g_tab[1])(21));
    sp = ((getp_t)t[2])();
    LOG('HSTO', sp);
    LOG('HEQ ', sp == (void *)t[0]);
    LOG('HCST', ((binop_t)t[3])(5, 6));
    LOG('HRCB', ((runcb_t)t[5])(HostCb, 7));
    return 0;
}

__declspec(dllexport) int HostFail(void)
{
    HMODULE h1, h2;
    LOG('HBEF', 0);
    h1 = W_LoadLibraryW(L"mod2.dll");
    LOG('HAF1', h1);
    LOG('HLE1', W_GetLastError());
    h2 = W_LoadLibraryW(L"mod2.dll");
    LOG('HAF2', h2);
    return 0;
}

__declspec(dllexport) int HostRefs(void)
{
    HMODULE h1, h2, h3, g;
    LOG('HBEF', 0);
    g = W_GetModuleHandleW(L"mod2.dll");
    LOG('HGM0', g);
    LOG('HGE0', W_GetLastError());
    h1 = W_LoadLibraryW(L"mod2.dll");
    LOG('HAF1', h1);
    h2 = W_LoadLibraryW(L"MOD2");
    LOG('HAF2', h2);
    h3 = W_LoadLibraryExW(L"mod2.dll", NULL, 0x800);
    LOG('HAF3', h3);
    g = W_GetModuleHandleW(L"mod2.dll");
    LOG('HGM1', g);
    LOG('HFR1', W_FreeLibrary(h1));
    LOG('HFR2', W_FreeLibrary(h2));
    LOG('HPAD', W_GetProcAddress(h3, "Add"));
    LOG('HFR3', W_FreeLibrary(h3));
    LOG('HGM2', W_GetModuleHandleW(L"mod2.dll"));
    h1 = W_LoadLibraryW(L"mod2.dll");
    LOG('HAF4', h1);
    return 0;
}

/* The same counting on a virtual module (kernel32.dll: one reference from the start). */
__declspec(dllexport) int HostRefsVirtual(void)
{
    HMODULE a;
    LOG('HBEF', 0);
    a = W_LoadLibraryW(L"kernel32.dll");
    LOG('HAF1', a);
    LOG('HFR1', W_FreeLibrary(a));
    LOG('HFR2', W_FreeLibrary(a));
    LOG('HFR3', 1);
    return 0;
}

/* Only the load, for a runtime that has no real-module table. */
__declspec(dllexport) int HostLoadOnly(void)
{
    HMODULE h;
    LOG('HBEF', 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    return 0;
}

/* FreeLibrary of the base address of a module that was never loaded through LoadLibrary. */
__declspec(dllexport) int HostFreeBase(void)
{
    LOG('HBEF', 0);
    LOG('HFRB', W_FreeLibrary((HMODULE)0x2000000ull));
    return 0;
}

/* LoadLibraryExW as the first load of the real module (the flags only steer a file search; decision 0082 names both loaders). */
__declspec(dllexport) int HostExFirst(void)
{
    HMODULE h;
    LOG('HBEF', 0);
    h = W_LoadLibraryExW(L"mod2.dll", NULL, 0x800);
    LOG('HAFT', h);
    LOG('HPAD', W_GetProcAddress(h, "Add"));
    return 0;
}

/* GetProcAddress by ordinal on the real module: a named stop. */
__declspec(dllexport) int HostOrdinal(void)
{
    HMODULE h;
    LOG('HBEF', 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    LOG('HORD', W_GetProcAddress(h, (LPCSTR)1));
    return 0;
}

/* GetProcAddress of an export forwarder ("Fwd" = kernel32.GetCurrentProcessId): a named stop. */
__declspec(dllexport) int HostFwd(void)
{
    HMODULE h;
    LOG('HBEF', 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    LOG('HFWD', W_GetProcAddress(h, "Fwd"));
    return 0;
}

/* GetModuleFileNameW: NULL keeps its answer (decision 0018); the handle of the real module answers the path of decision 0104
 * ("C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\", 48 characters, + the export directory's Name "mod2.orig.dll", 13 characters, so 61),
 * which replaced the stop of decision 0082 D6 (decision 0102 W1 added the observations of the characters and of the truncation). */
__declspec(dllexport) int HostGmfn(void)
{
    HMODULE h;
    WCHAR buf[300];
    LOG('HBEF', 0);
    LOG('HGN0', W_GetModuleFileNameW(NULL, buf, 300) > 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    LOG('HGN1', W_GetModuleFileNameW(h, buf, 300));
    LOG('HGC0', buf[0]);
    LOG('HGC1', buf[1]);
    LOG('HGCL', buf[60]);
    LOG('HGCZ', buf[61]);
    W_SetLastError(7);
    LOG('HGN2', W_GetModuleFileNameW(h, buf, 5));
    LOG('HGE2', W_GetLastError());
    LOG('HGC4', buf[4]);
    LOG('HGC3', buf[3]);
    return 0;
}

/* LoadStringW on the handle of the real module reads that module's RT_STRING resource (decision 0102 W2). */
__declspec(dllexport) int HostResMod(void)
{
    HMODULE h;
    WCHAR buf[40];
    LOG('HBEF', 0);
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    LOG('HLS1', W_LoadStringW(h, 1, buf, 40));
    LOG('HLC0', buf[0]);
    LOG('HLC7', buf[7]);
    LOG('HLZ1', buf[8]);
    LOG('HLS2', W_LoadStringW(h, 2, buf, 4));
    LOG('HLC2', buf[2]);
    LOG('HLZ2', buf[3]);
    LOG('HLS3', W_LoadStringW(h, 3, buf, 40));
    LOG('HLE3', W_GetLastError());
    return 0;
}

/* GetModuleHandleW(NULL) and the virtual handles are unchanged; the real module is not found before it is loaded. */
__declspec(dllexport) int HostHandles(void)
{
    LOG('HBEF', 0);
    LOG('HGN0', W_GetModuleHandleW(NULL));
    LOG('HGK3', W_GetModuleHandleW(L"kernel32.dll"));
    LOG('HGM0', W_GetModuleHandleW(L"mod2.dll"));
    LOG('HGE0', W_GetLastError());
    return 0;
}

/* Decision 0091: GetModuleHandleA next to GetModuleHandleW, for names that need no load (the real module is not loaded here, so with
 * and without a real-module table the answers are the same). The last error is set to 5 before each call, read after it. */
__declspec(dllexport) int HostGmhNames(void)
{
    LOG('HBEF', 0);
    W_SetLastError(5);
    LOG('HAN0', W_GetModuleHandleA(NULL));
    LOG('HAE0', W_GetLastError());
    LOG('HAK3', W_GetModuleHandleA("Kernel32"));
    LOG('HWK3', W_GetModuleHandleW(L"Kernel32"));
    LOG('HAS1', W_GetModuleHandleA("API-MS-WIN-CORE-SYNCH-L1-2-0"));
    LOG('HWS1', W_GetModuleHandleW(L"API-MS-WIN-CORE-SYNCH-L1-2-0"));
    W_SetLastError(5);
    LOG('HAM0', W_GetModuleHandleA("mod2.dll"));
    LOG('HAE1', W_GetLastError());
    W_SetLastError(5);
    LOG('HWM0', W_GetModuleHandleW(L"mod2.dll"));
    LOG('HWE1', W_GetLastError());
    W_SetLastError(5);
    LOG('HAX0', W_GetModuleHandleA("nosuch.dll"));
    LOG('HAE2', W_GetLastError());
    return 0;
}

/* Decision 0091: GetModuleHandleA of a real module before the load, after it, and (mode bit 16 of the module) inside its own DllMain,
 * each next to GetModuleHandleW (decision 0082: not found before the load and during DllMain, the base after). */
__declspec(dllexport) int HostGmhA(void)
{
    HMODULE h;
    LOG('HBEF', 0);
    W_SetLastError(5);
    LOG('HAB0', W_GetModuleHandleA("mod2.dll"));
    LOG('HAE0', W_GetLastError());
    h = W_LoadLibraryW(L"mod2.dll");
    LOG('HAFT', h);
    W_SetLastError(5);
    LOG('HAB1', W_GetModuleHandleA("MOD2"));
    LOG('HAE1', W_GetLastError());
    W_SetLastError(5);
    LOG('HAB2', W_GetModuleHandleA("mod2"));
    LOG('HWB2', W_GetModuleHandleW(L"mod2"));
    LOG('HAE2', W_GetLastError());
    /* a trailing point means "no extension": it is removed and ".dll" is not added ("mod2." is "mod2", "mod2.dll." is "mod2.dll") */
    LOG('HAB3', W_GetModuleHandleA("mod2."));
    LOG('HAE3', W_GetLastError());
    LOG('HWB3', W_GetModuleHandleW(L"mod2."));
    LOG('HAB4', W_GetModuleHandleA("mod2.dll."));
    LOG('HWB4', W_GetModuleHandleW(L"mod2.dll."));
    return 0;
}

/* FreeLibrary of the handle of the lifted DLL itself (the "FLEngine" handle 0x400000): a named stop. */
__declspec(dllexport) int HostFreeExe(void)
{
    LOG('HBEF', 0);
    LOG('HFRE', W_FreeLibrary((HMODULE)0x400000ull));
    return 0;
}

/* Decision 0081: the four API set names are modules (the load succeeds, handles N = 27..30) with nothing to find. */
__declspec(dllexport) int HostApiSet(void)
{
    HMODULE a, b, c, d;
    LOG('HBEF', 0);
    a = W_LoadLibraryExW(L"api-ms-win-core-synch-l1-2-0", NULL, 0x800);
    b = W_LoadLibraryExW(L"api-ms-win-core-fibers-l1-1-1.dll", NULL, 0x800);
    c = W_LoadLibraryExW(L"api-ms-win-core-localization-l1-2-1", NULL, 0x800);
    d = W_LoadLibraryExW(L"API-MS-WIN-APPMODEL-RUNTIME-L1-1-2.DLL", NULL, 0x800);
    LOG('HAS1', a);
    LOG('HAS2', b);
    LOG('HAS3', c);
    LOG('HAS4', d);
    LOG('HGM1', W_GetModuleHandleW(L"api-ms-win-core-synch-l1-2-0"));
    W_SetLastError(5);
    LOG('HPA1', W_GetProcAddress(a, "InitializeCriticalSectionEx"));
    LOG('HPE1', W_GetLastError());
    LOG('HPA2', W_GetProcAddress(b, "FlsAlloc"));
    LOG('HPA3', W_GetProcAddress(c, "LCMapStringEx"));
    LOG('HPA4', W_GetProcAddress(d, "AppPolicyGetProcessTerminationMethod"));
    LOG('HPE4', W_GetLastError());
    return 0;
}
