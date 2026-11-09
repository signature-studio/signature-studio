/* Synthetic second module (stands in for dsp_ippv2_x64.dll). Built with mingw as a DLL at ImageBase 0x180000000, then rebased
 * to 0x2000000 with fe-pe rebase. No CRT. The harness picks the DllMain behaviour per run through the mode word SHLOG->pad:
 *   bit 0 (1): ATTACH returns 0 (after W_SetLastError(0x5678)), every time
 *   bit 1 (2): ATTACH loads "mod2.dll" (itself) and "kernel32.dll", then frees kernel32 (re-entry into the loader)
 *   bit 3 (8): ATTACH returns 0 only on the first attach (g_attach_count == 1)
 *   bit 4 (16): ATTACH logs GetModuleHandleA/W of its own name and of "kernel32" (decision 0091)        */
#include <windows.h>
#include "shared.h"

__declspec(dllexport) volatile int g_attach_count = 0;
__declspec(dllexport) volatile int g_detach_count = 0;

__attribute__((noinline)) __declspec(dllexport) int Add(int a, int b) { return a + b; }

/* A function pointer in .data: one DIR64 relocation. */
static int (*volatile g_fnptr)(int, int) = Add;

__attribute__((noinline)) __declspec(dllexport) int Twice(int x) { return Add(x, x); }
__attribute__((noinline)) __declspec(dllexport) void *GetStoredPtr(void) { return (void *)g_fnptr; }
__attribute__((noinline)) __declspec(dllexport) int CallStored(int a, int b) { return g_fnptr(a, b); }
/* Calls back into the other module (a pointer the host passes in): the target is only reachable through the shared dispatch table. */
__attribute__((noinline)) __declspec(dllexport) int RunCb(int (*cb)(int), int x) { return cb(x) + 1; }

static BOOL CALLBACK mon_cb(HMONITOR m, HDC dc, LPRECT r, LPARAM d)
{
    LOG('DECB', d);
    LOG('DEC1', m);
    LOG('DEC2', dc);
    LOG('DEC3', r->right);
    return TRUE;
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD reason, LPVOID reserved)
{
    u32 mode = SHLOG->pad;
    LOG('DENT', reason);
    if (reason == 1) {
        HMODULE k;
        FARPROC f;
        BOOL er;
        g_attach_count++;
        LOG('DHIN', h);
        LOG('DRES', reserved);
        LOG('DADD', Add(20, 22));
        W_SetLastError(0x1111);
        LOG('DLE ', W_GetLastError());
        k = W_GetModuleHandleW(L"kernel32.dll");
        f = W_GetProcAddress(k, "GetCurrentProcessId");
        LOG('DPEQ', ((DWORD(WINAPI *)(void))f)() == W_GetCurrentProcessId());
        er = W_EnumDisplayMonitors(NULL, NULL, mon_cb, 0xABCD);
        LOG('DEMR', er);
        if (mode & 16) {
            /* decision 0091: the module's own name during its own DllMain, A next to W (decision 0082: "not loaded") */
            W_SetLastError(5);
            LOG('DGAN', W_GetModuleHandleA(NULL));
            LOG('DGAM', W_GetModuleHandleA("mod2.dll"));
            LOG('DGAE', W_GetLastError());
            W_SetLastError(5);
            LOG('DGWM', W_GetModuleHandleW(L"mod2.dll"));
            LOG('DGWE', W_GetLastError());
            LOG('DGAK', W_GetModuleHandleA("kernel32"));
        }
        if (mode & 2) {
            HMODULE self = W_LoadLibraryW(L"mod2.dll");
            HMODULE kk = W_LoadLibraryW(L"kernel32.dll");
            LOG('DSLF', self);
            LOG('DK32', kk);
            LOG('DFK ', W_FreeLibrary(kk));
        }
        if (mode & 1) {
            W_SetLastError(0x5678);
            return 0;
        }
        if ((mode & 8) && g_attach_count == 1) {
            W_SetLastError(0x5678);
            return 0;
        }
        return 1;
    }
    if (reason == 0) {
        g_detach_count++;
        LOG('DRS0', reserved);
    }
    return 1;
}
