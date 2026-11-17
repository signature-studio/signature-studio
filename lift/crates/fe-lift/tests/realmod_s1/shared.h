/* Synthetic guests of the real-module regression test (tests/realmod.rs; the stage 1 guests of docs/investigations/stage1-two-module.md).
 * Both modules append events to one log page that the driver maps at 0x3000000 (a fixed address that neither image uses), so the
 * order of events across the two modules can be read back in one sequence. */
#ifndef REALMOD_SHARED_H
#define REALMOD_SHARED_H

typedef unsigned long long u64;
typedef unsigned int u32;

struct logent { u32 tag; u32 pad; u64 val; };
struct shlog { u32 n; u32 pad; struct logent e[1000]; };
#define SHLOG ((volatile struct shlog *)0x3000000ull)

/* The value is evaluated first: it may itself log (a callback), and the index must be read after that. */
#define LOG(T_, V_) do { u64 v_ = (u64)(V_); volatile struct shlog *L_ = SHLOG; u32 i_ = L_->n; L_->e[i_].tag = (T_); L_->e[i_].val = v_; L_->n = i_ + 1; } while (0)

/* One call per wrapper: gcc otherwise keeps an IAT pointer in a callee-saved register and emits `call rsi`, which fe-decode cannot
 * resolve to the import (the call then goes to fle_dispatch with the slot VA and stops). Each wrapper is `call/jmp [slot]` only. */
#include <windows.h>
#define NOINL __attribute__((noinline))
static NOINL HMODULE W_LoadLibraryW(LPCWSTR n) { return LoadLibraryW(n); }
static NOINL HMODULE W_LoadLibraryExW(LPCWSTR n, HANDLE f, DWORD fl) { return LoadLibraryExW(n, f, fl); }
static NOINL BOOL W_FreeLibrary(HMODULE h) { return FreeLibrary(h); }
static NOINL FARPROC W_GetProcAddress(HMODULE h, LPCSTR n) { return GetProcAddress(h, n); }
static NOINL HMODULE W_GetModuleHandleW(LPCWSTR n) { return GetModuleHandleW(n); }
static NOINL HMODULE W_GetModuleHandleA(LPCSTR n) { return GetModuleHandleA(n); }
static NOINL DWORD W_GetModuleFileNameW(HMODULE h, LPWSTR b, DWORD n) { return GetModuleFileNameW(h, b, n); }
static NOINL int W_LoadStringW(HINSTANCE h, UINT id, LPWSTR b, int n) { return LoadStringW(h, id, b, n); }
static NOINL DWORD W_GetLastError(void) { return GetLastError(); }
static NOINL void W_SetLastError(DWORD e) { SetLastError(e); }
static NOINL DWORD W_GetCurrentProcessId(void) { return GetCurrentProcessId(); }
static NOINL BOOL W_EnumDisplayMonitors(HDC d, LPCRECT r, MONITORENUMPROC p, LPARAM l) { return EnumDisplayMonitors(d, r, p, l); }

#endif
