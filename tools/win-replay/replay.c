/*
 * replay.c - read-only Windows API replay for the A2b check (decision 0046 section 3).
 *
 * Reads cases.tsv (made by gen_cases.py from the recorded trace), calls only the
 * Windows APIs named in the whitelist table below, and writes observed.tsv.
 * It contains no FL Studio code and needs nothing but the Windows system DLLs.
 *
 * What it does NOT do: create windows, create threads, write files (other than its own
 * output), write the registry, change system settings, use the network, touch the clipboard
 * or the global atom table. Calls change only state of this process (heap, TLS/FLS, GDI
 * objects on memory DCs, thread locale / error mode, restored afterwards).
 *
 * Build (x64 only):
 *   MSVC:   cl /nologo /O2 /W3 /MT replay.c /Fe:replay.exe /link /MANIFEST:NO
 *   mingw:  x86_64-w64-mingw32-gcc -O2 -Wall -o replay.exe replay.c
 */
#define _CRT_SECURE_NO_WARNINGS
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <ctype.h>
#include <wchar.h>

#ifndef _WIN64
#error "replay.c is x64 only (the traced module is FLEngine_x64.dll)"
#endif

#define REPLAY_VERSION "1"
#define MAXARGS 13
#define MAXBUFS 16
#define MAXVARS 2048
#define MAXREG 8192
#define SENTINEL 0x5EED5EEDu

typedef unsigned long long u64;
typedef unsigned char u8;

/* ------------------------------------------------------------------ string builder */
typedef struct { char *p; size_t n, cap; } SB;
static void sb_reserve(SB *s, size_t add) {
    if (s->n + add + 1 > s->cap) {
        size_t nc = s->cap ? s->cap : 256;
        while (nc < s->n + add + 1) nc *= 2;
        s->p = (char *)realloc(s->p, nc);
        if (!s->p) { fprintf(stderr, "out of memory\n"); exit(2); }
        s->cap = nc;
    }
}
static void sb_cat(SB *s, const char *t) { size_t l = strlen(t); sb_reserve(s, l); memcpy(s->p + s->n, t, l + 1); s->n += l; }
static void sb_printf(SB *s, const char *fmt, ...) {
    char tmp[4096]; va_list ap;
    va_start(ap, fmt); vsnprintf(tmp, sizeof tmp, fmt, ap); va_end(ap);
    tmp[sizeof tmp - 1] = 0; sb_cat(s, tmp);
}
static void sb_clear(SB *s) { s->n = 0; if (s->p) s->p[0] = 0; }
static void sb_hex(SB *s, const u8 *b, size_t n) {
    static const char hx[] = "0123456789abcdef"; size_t i;
    sb_reserve(s, n * 2);
    for (i = 0; i < n; i++) { s->p[s->n++] = hx[b[i] >> 4]; s->p[s->n++] = hx[b[i] & 15]; }
    s->p[s->n] = 0;
}
static void sb_sep(SB *s, const char *sep) { if (s->n) sb_cat(s, sep); }

/* ------------------------------------------------------------------ small helpers */
static void scpy(char *d, size_t cap, const char *s) { size_t n = strlen(s); if (n >= cap) n = cap - 1; memcpy(d, s, n); d[n] = 0; }
static int hexval(int c) { if (c >= '0' && c <= '9') return c - '0'; if (c >= 'a' && c <= 'f') return c - 'a' + 10; if (c >= 'A' && c <= 'F') return c - 'A' + 10; return -1; }
static long hex2bin(const char *s, u8 *out, size_t cap) {
    size_t n = 0;
    while (s[0] && s[1]) {
        int a = hexval(s[0]), b = hexval(s[1]);
        if (a < 0 || b < 0 || n >= cap) return -1;
        out[n++] = (u8)(a * 16 + b); s += 2;
    }
    return s[0] ? -1 : (long)n;
}
static size_t pct_decode(const char *s, u8 *out, size_t cap) {
    size_t n = 0;
    while (*s && n < cap) {
        if (*s == '%' && hexval(s[1]) >= 0 && hexval(s[2]) >= 0) { out[n++] = (u8)(hexval(s[1]) * 16 + hexval(s[2])); s += 3; }
        else out[n++] = (u8)*s++;
    }
    return n;
}
static u64 parse_num(const char *s) {
    if (*s == '-') return (u64)strtoll(s, NULL, 0);
    return strtoull(s, NULL, 0);
}
static const char *hx(u64 v, char *buf) { if (v < 10) sprintf(buf, "%llu", v); else sprintf(buf, "0x%llx", v); return buf; }
static void clean_cat(SB *s, const char *t) {
    for (; *t; t++) {
        if (*t == '\t') sb_cat(s, "\\t"); else if (*t == '\n') sb_cat(s, "\\n"); else if (*t == '\r') sb_cat(s, "\\r");
        else { char c[2]; c[0] = *t; c[1] = 0; sb_cat(s, c); }
    }
}
static void cat_wesc(SB *s, const wchar_t *w, size_t maxn) {
    size_t i;
    for (i = 0; i < maxn && w[i]; i++) {
        unsigned c = w[i];
        if (c >= 0x20 && c < 0x7f && c != '\\') { char t[2]; t[0] = (char)c; t[1] = 0; sb_cat(s, t); }
        else if (c == '\\') sb_cat(s, "\\");
        else sb_printf(s, "\\u%04x", c);
    }
}
static void cat_aesc(SB *s, const char *a, size_t maxn) {
    size_t i;
    for (i = 0; i < maxn && a[i]; i++) {
        unsigned c = (u8)a[i];
        if (c >= 0x20 && c < 0x7f) { char t[2]; t[0] = (char)c; t[1] = 0; sb_cat(s, t); }
        else sb_printf(s, "\\u%04x", c);
    }
}

/* ------------------------------------------------------------------ privacy masking */
static int g_mask = 1;
static char g_user[128], g_comp[128], g_profile[MAX_PATH];
static void ci_replace(char *s, size_t cap, const char *needle, const char *repl) {
    size_t nl = strlen(needle), rl = strlen(repl);
    size_t i;
    if (nl < 2) return;
    for (i = 0; s[i];) {
        if (_strnicmp(s + i, needle, nl) == 0) {
            size_t tail = strlen(s + i + nl);
            if (strlen(s) - nl + rl + 1 > cap) return;
            memmove(s + i + rl, s + i + nl, tail + 1);
            memcpy(s + i, repl, rl); i += rl;
        } else i++;
    }
}
static void mask_text(char *s, size_t cap) {
    if (!g_mask) return;
    if (g_profile[0]) ci_replace(s, cap, g_profile, "<PROFILE>");
    if (g_user[0]) ci_replace(s, cap, g_user, "<USER>");
    if (g_comp[0]) ci_replace(s, cap, g_comp, "<COMPUTER>");
}

/* ------------------------------------------------------------------ variables, buffers, regions */
enum { VF_NONE = 0, VF_MEMDC = 1, VF_OWNED = 2, VF_CODE = 4, VF_REGKEY = 8, VF_HEAP = 16 };
typedef struct { char name[48]; u64 val; u8 *buf; size_t size; unsigned flags; int used; } Var;
static Var g_vars[MAXVARS];
static int g_nvars;
static Var *var_find(const char *name) { int i; for (i = 0; i < g_nvars; i++) if (!strcmp(g_vars[i].name, name)) return &g_vars[i]; return NULL; }
static Var *var_set(const char *name, u64 val, u8 *buf, size_t size, unsigned flags) {
    Var *v = var_find(name);
    if (!v) { if (g_nvars >= MAXVARS) return NULL; v = &g_vars[g_nvars++]; memset(v, 0, sizeof *v); scpy(v->name, sizeof v->name, name); }
    v->val = val; v->buf = buf; v->size = size; v->flags = flags; return v;
}
static void var_forget(u64 val) { int i; for (i = 0; i < g_nvars; i++) if (g_vars[i].val == val && !g_vars[i].buf && val) { g_vars[i].val = 0; g_vars[i].flags = 0; } }
static Var *var_by_val(u64 val) { int i; for (i = g_nvars - 1; i >= 0; i--) if (g_vars[i].val == val && !g_vars[i].buf) return &g_vars[i]; return NULL; }

typedef struct { u64 ptr; size_t size; } Region;
static Region g_reg[MAXREG]; static int g_nreg;
static void reg_add(u64 p, size_t n) { if (p && g_nreg < MAXREG) { g_reg[g_nreg].ptr = p; g_reg[g_nreg].size = n; g_nreg++; } }
static void reg_del(u64 p) { int i; for (i = 0; i < g_nreg; i++) if (g_reg[i].ptr == p) { g_reg[i] = g_reg[--g_nreg]; return; } }
static int reg_ok(u64 p, size_t n) { int i; for (i = 0; i < g_nreg; i++) if (p >= g_reg[i].ptr && p + n <= g_reg[i].ptr + g_reg[i].size) return 1; return 0; }
static int reg_has(u64 p) { int i; for (i = 0; i < g_nreg; i++) if (g_reg[i].ptr == p) return 1; return 0; }

typedef struct { int arg; u8 *user; size_t size; u8 *before; u8 *raw; int owned; } Buf;
typedef struct { u64 a[MAXARGS]; int n; Buf b[MAXBUFS]; int nb; const char *why; char note[256]; } Ctx;

static u8 *alloc16(size_t n, u8 **raw) {
    u8 *r = (u8 *)calloc(1, n + 96);
    u8 *u;
    if (!r) { fprintf(stderr, "out of memory\n"); exit(2); }
    u = (u8 *)(((size_t)r + 31) & ~(size_t)15);
    memset(u + n, 0xAB, 32);
    *raw = r; return u;
}
static Buf *ctx_buf(Ctx *c, int arg, u8 *user, size_t size, u8 *raw, int owned) {
    Buf *b;
    if (c->nb >= MAXBUFS) return NULL;
    b = &c->b[c->nb++];
    b->arg = arg; b->user = user; b->size = size; b->raw = raw; b->owned = owned;
    b->before = (u8 *)malloc(size ? size : 1); memcpy(b->before, user, size);
    return b;
}
static void ctx_free(Ctx *c) { int i; for (i = 0; i < c->nb; i++) free(c->b[i].before); c->nb = 0; }

/* ------------------------------------------------------------------ system DLL allowlist */
static const char *k_sysdlls[] = {
    "kernel32", "kernelbase", "ntdll", "user32", "gdi32", "advapi32", "shell32", "shfolder", "shcore", "ole32", "oleaut32",
    "comctl32", "version", "msacm32", "avrt", "powrprof", "dwmapi", "imm32", "msctf", "ucrtbase", "win32u", NULL };
static int is_sysdll(const char *name) {
    char t[128]; size_t l = strlen(name); int i;
    if (l == 0 || l >= sizeof t) return 0;
    for (i = 0; name[i]; i++) { unsigned char ch = (unsigned char)name[i]; if (!(isalnum(ch) || ch == '-' || ch == '_' || ch == '.')) return 0; t[i] = (char)tolower(ch); }
    t[l] = 0;
    if (l > 4 && !strcmp(t + l - 4, ".dll")) t[l - 4] = 0;
    if (!strncmp(t, "api-ms-win-", 11)) return 1;
    for (i = 0; k_sysdlls[i]; i++) if (!strcmp(t, k_sysdlls[i])) return 1;
    return 0;
}
typedef struct { char name[64]; HMODULE h; int tried; DWORD le; } ModCache;
static ModCache g_mods[96]; static int g_nmods;
static HMODULE get_module(const char *name, DWORD *le) {
    int i; wchar_t w[130];
    if (!_stricmp(name, "self")) { *le = 0; return GetModuleHandleW(NULL); }
    if (!is_sysdll(name)) { *le = ERROR_ACCESS_DENIED; return NULL; }
    for (i = 0; i < g_nmods; i++) if (!_stricmp(g_mods[i].name, name)) { *le = g_mods[i].le; return g_mods[i].h; }
    for (i = 0; name[i] && i < 120; i++) w[i] = (wchar_t)(u8)name[i];
    w[i] = 0;
    SetLastError(0);
    {
        HMODULE h = LoadLibraryExW(w, NULL, 0x800 /* LOAD_LIBRARY_SEARCH_SYSTEM32 */);
        DWORD e = h ? 0 : GetLastError();
        if (g_nmods < 96) { scpy(g_mods[g_nmods].name, sizeof g_mods[g_nmods].name, name); g_mods[g_nmods].h = h; g_mods[g_nmods].le = e; g_nmods++; }
        *le = e; return h;
    }
}

/* ------------------------------------------------------------------ whitelist */
enum { DEC_NONE = 0, DEC_WOUT, DEC_AOUT, DEC_SYSINFO, DEC_OSVIW, DEC_OSVIA, DEC_TZ, DEC_SYSTIME, DEC_FILETIME, DEC_U64, DEC_STARTUP,
       DEC_CPINFO, DEC_SPI, DEC_U32, DEC_DEVMODE, DEC_MONINFO, DEC_RECT, DEC_POINT, DEC_LPI, DEC_LANGS, DEC_DIBSECTION, DEC_LOGFONT,
       DEC_CBTEXT, DEC_CMDW, DEC_CMDA, DEC_ENVBLOCK, DEC_COMPNAME };
enum { HK_NONE = 0, HK_ALLOC_HEAP, HK_ALLOC_LOCAL, HK_VALLOC, HK_VPROTECT, HK_MODFILE, HK_GDIOBJ, HK_CHARPTR, HK_DIB, HK_BSTR, HK_REALLOC,
       HK_FREE_HEAP, HK_LOADLIB, HK_FREELIB, HK_REGOPEN, HK_FINDFIRST, HK_BSTRRE, HK_BSTRFREE };
#define F_HELPER 1u   /* not in the trace; used only to read state back */
#define F_NOHEX  2u   /* never print the raw output bytes (private data); decoded text is masked */
typedef struct { const char *name, *dll, *sig; char ret; int dec, deca, hook; unsigned flags; const char *ulabel; } Api;
#define A(n, d, s, r, dc, da, hk, fl, ul) { n, d, s, r, dc, da, hk, fl, ul }
static const Api k_api[] = {
    /* kernel32: text and locale */
    A("WideCharToMultiByte", "kernel32", "ddWdOdAO", 'd', 0, 0, 0, 0, 0),
    A("MultiByteToWideChar", "kernel32", "ddAdOd", 'd', 0, 0, 0, 0, 0),
    A("CompareStringW", "kernel32", "ddWdWd", 'd', 0, 0, 0, 0, 0),
    A("LCMapStringW", "kernel32", "ddWdOd", 'd', 0, 0, 0, 0, 0),
    A("GetStringTypeW", "kernel32", "dWdO", 'd', 0, 0, 0, 0, 0),
    A("GetLocaleInfoW", "kernel32", "ddOd", 'd', DEC_WOUT, 2, 0, 0, 0),
    A("IsValidLocale", "kernel32", "dd", 'd', 0, 0, 0, 0, 0),
    A("GetACP", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetOEMCP", "kernel32", "", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetCPInfo", "kernel32", "dO", 'd', DEC_CPINFO, 1, 0, 0, 0),
    A("IsValidCodePage", "kernel32", "d", 'd', 0, 0, 0, 0, 0),
    A("GetThreadLocale", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("SetThreadLocale", "kernel32", "d", 'd', 0, 0, 0, 0, 0),
    A("GetUserDefaultUILanguage", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetThreadUILanguage", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetThreadPreferredUILanguages", "kernel32", "dOOB", 'd', DEC_LANGS, 2, 0, 0, 0),
    A("GetSystemDefaultLocaleName", "kernel32", "Od", 'd', DEC_WOUT, 0, 0, 0, 0),
    A("GetUserDefaultLCID", "kernel32", "", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetSystemDefaultLCID", "kernel32", "", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetUserDefaultLocaleName", "kernel32", "Od", 'd', DEC_WOUT, 0, 0, F_HELPER, 0),
    A("MulDiv", "kernel32", "ddd", 'd', 0, 0, 0, 0, 0),
    /* kernel32: version, system, process, time */
    A("GetVersion", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetVersionExW", "kernel32", "B", 'd', DEC_OSVIW, 0, 0, 0, 0),
    A("GetVersionExA", "kernel32", "B", 'd', DEC_OSVIA, 0, 0, 0, 0),
    A("VerifyVersionInfoW", "kernel32", "Sdi", 'd', 0, 0, 0, 0, 0),
    A("VerSetConditionMask", "kernel32", "idd", 'q', 0, 0, 0, 0, 0),
    A("RtlGetVersion", "ntdll", "O", 'd', DEC_OSVIW, 0, 0, F_HELPER, 0),
    A("GetSystemInfo", "kernel32", "O", 'v', DEC_SYSINFO, 0, 0, 0, 0),
    A("GetNativeSystemInfo", "kernel32", "O", 'v', DEC_SYSINFO, 0, 0, 0, 0),
    A("GetLogicalProcessorInformation", "kernel32", "OB", 'd', DEC_LPI, 0, 0, 0, 0),
    A("GetComputerNameExW", "kernel32", "dOB", 'd', DEC_COMPNAME, 1, 0, F_NOHEX, 0),
    A("GetEnvironmentVariableW", "kernel32", "WOd", 'd', DEC_WOUT, 1, 0, F_NOHEX, 0),
    A("GetEnvironmentStringsW", "kernel32", "", 'q', DEC_ENVBLOCK, 0, 0, F_NOHEX, 0),
    A("FreeEnvironmentStringsW", "kernel32", "P", 'd', 0, 0, 0, 0, 0),
    A("GetCommandLineW", "kernel32", "", 'q', DEC_CMDW, 0, 0, F_NOHEX, 0),
    A("GetCommandLineA", "kernel32", "", 'q', DEC_CMDA, 0, 0, F_NOHEX, 0),
    A("GetStartupInfoW", "kernel32", "O", 'v', DEC_STARTUP, 0, 0, F_NOHEX, 0),
    A("GetStartupInfoA", "kernel32", "O", 'v', DEC_STARTUP, 0, 0, F_NOHEX, 0),
    A("GetModuleFileNameW", "kernel32", "iOd", 'd', DEC_WOUT, 1, 0, F_NOHEX, 0),
    A("GetModuleFileNameA", "kernel32", "iOd", 'd', DEC_AOUT, 1, 0, F_NOHEX, 0),
    A("GetTempPathW", "kernel32", "dO", 'd', DEC_WOUT, 1, 0, F_NOHEX, 0),
    A("GetTimeZoneInformation", "kernel32", "O", 'd', DEC_TZ, 0, 0, 0, 0),
    A("GetLocalTime", "kernel32", "O", 'v', DEC_SYSTIME, 0, 0, 0, 0),
    A("GetSystemTimeAsFileTime", "kernel32", "O", 'v', DEC_FILETIME, 0, 0, 0, 0),
    A("GetTickCount", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("QueryPerformanceCounter", "kernel32", "O", 'd', DEC_U64, 0, 0, 0, 0),
    A("QueryPerformanceFrequency", "kernel32", "O", 'd', DEC_U64, 0, 0, 0, 0),
    A("GetCurrentProcess", "kernel32", "", 'q', 0, 0, 0, 0, 0),
    A("GetCurrentThread", "kernel32", "", 'q', 0, 0, 0, 0, 0),
    A("GetCurrentProcessId", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetCurrentThreadId", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("GetStdHandle", "kernel32", "d", 'q', 0, 0, 0, 0, 0),
    A("GetLastError", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("SetLastError", "kernel32", "d", 'v', 0, 0, 0, 0, 0),
    A("SetErrorMode", "kernel32", "d", 'd', 0, 0, 0, 0, 0),
    A("GetErrorMode", "kernel32", "", 'd', 0, 0, 0, F_HELPER, 0),
    A("SetThreadErrorMode", "kernel32", "dO", 'd', DEC_U32, 1, 0, 0, "oldMode"),
    A("SetHandleCount", "kernel32", "d", 'd', 0, 0, 0, 0, 0),
    A("SetDllDirectoryW", "kernel32", "W", 'd', 0, 0, 0, 0, 0),
    /* kernel32: modules */
    A("GetModuleHandleW", "kernel32", "W", 'q', 0, 0, HK_MODFILE, 0, 0),
    A("GetModuleHandleA", "kernel32", "A", 'q', 0, 0, HK_MODFILE, 0, 0),
    A("GetProcAddress", "kernel32", "iA", 'q', 0, 0, 0, 0, 0),
    A("LoadLibraryW", "kernel32", "W", 'q', 0, 0, HK_LOADLIB, 0, 0),
    A("LoadLibraryA", "kernel32", "A", 'q', 0, 0, HK_LOADLIB, 0, 0),
    A("LoadLibraryExW", "kernel32", "Wid", 'q', 0, 0, HK_LOADLIB, 0, 0),
    A("FreeLibrary", "kernel32", "i", 'd', 0, 0, HK_FREELIB, 0, 0),
    /* kernel32: memory, TLS/FLS, sync (process-private) */
    A("GetProcessHeap", "kernel32", "", 'q', 0, 0, 0, 0, 0),
    A("HeapCreate", "kernel32", "dii", 'q', 0, 0, 0, 0, 0),
    A("HeapAlloc", "kernel32", "idi", 'q', 0, 0, HK_ALLOC_HEAP, 0, 0),
    A("HeapReAlloc", "kernel32", "idPi", 'q', 0, 0, HK_REALLOC, 0, 0),
    A("HeapSize", "kernel32", "idP", 'q', 0, 0, 0, 0, 0),
    A("HeapFree", "kernel32", "idP", 'd', 0, 0, HK_FREE_HEAP, 0, 0),
    A("HeapSetInformation", "kernel32", "idPi", 'd', 0, 0, 0, 0, 0),
    A("LocalAlloc", "kernel32", "di", 'q', 0, 0, HK_ALLOC_LOCAL, 0, 0),
    A("LocalFree", "kernel32", "i", 'q', 0, 0, HK_FREE_HEAP, F_HELPER, 0),
    A("VirtualAlloc", "kernel32", "iidd", 'q', 0, 0, HK_VALLOC, 0, 0),
    A("VirtualProtect", "kernel32", "iidO", 'd', DEC_U32, 3, HK_VPROTECT, 0, "oldProtect"),
    A("TlsAlloc", "kernel32", "", 'd', 0, 0, 0, 0, 0),
    A("TlsSetValue", "kernel32", "dP", 'd', 0, 0, 0, 0, 0),
    A("TlsGetValue", "kernel32", "d", 'q', 0, 0, 0, 0, 0),
    A("TlsFree", "kernel32", "d", 'd', 0, 0, 0, F_HELPER, 0),
    A("FlsAlloc", "kernel32", "F", 'd', 0, 0, 0, 0, 0),
    A("FlsSetValue", "kernel32", "dP", 'd', 0, 0, 0, 0, 0),
    A("FlsGetValue", "kernel32", "d", 'q', 0, 0, 0, 0, 0),
    A("FlsFree", "kernel32", "d", 'd', 0, 0, 0, F_HELPER, 0),
    A("InitializeCriticalSection", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("InitializeCriticalSectionAndSpinCount", "kernel32", "Pd", 'd', 0, 0, 0, 0, 0),
    A("EnterCriticalSection", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("LeaveCriticalSection", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("DeleteCriticalSection", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("AcquireSRWLockExclusive", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("ReleaseSRWLockExclusive", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("InitializeSListHead", "kernel32", "P", 'v', 0, 0, 0, 0, 0),
    A("EncodePointer", "kernel32", "P", 'q', 0, 0, 0, 0, 0),
    A("DecodePointer", "kernel32", "P", 'q', 0, 0, 0, 0, 0),
    /* kernel32: file system and version resources (read-only queries on paths) */
    A("GetFileAttributesW", "kernel32", "W", 'd', 0, 0, 0, 0, 0),
    A("GetFileAttributesExW", "kernel32", "WdO", 'd', 0, 0, 0, 0, 0),
    A("FindFirstFileW", "kernel32", "WO", 'q', 0, 0, HK_FINDFIRST, 0, 0),
    A("GetFileVersionInfoSizeW", "version", "WO", 'd', 0, 0, 0, 0, 0),
    /* advapi32: registry, read-only (access mask forced to KEY_READ) */
    A("RegOpenKeyExW", "advapi32", "iWddO", 'd', 0, 0, HK_REGOPEN, 0, 0),
    A("RegOpenKeyExA", "advapi32", "iAddO", 'd', 0, 0, HK_REGOPEN, 0, 0),
    A("RegQueryValueExW", "advapi32", "iWPOOB", 'd', 0, 0, 0, 0, 0),
    A("RegQueryInfoKeyW", "advapi32", "iOBPOOOOOOOO", 'd', 0, 0, 0, 0, 0),
    A("RegCloseKey", "advapi32", "i", 'd', 0, 0, 0, 0, 0),
    /* shell */
    A("SHGetFolderPathW", "shell32", "ididO", 'd', DEC_WOUT, 4, 0, F_NOHEX, 0),
    A("SHGetMalloc", "shell32", "O", 'd', 0, 0, 0, 0, 0),
    /* user32 */
    A("GetSystemMetrics", "user32", "d", 'd', 0, 0, 0, 0, 0),
    A("SystemParametersInfoW", "user32", "ddBd", 'd', DEC_SPI, 2, 0, 0, 0),
    A("GetSysColor", "user32", "d", 'd', 0, 0, 0, 0, 0),
    A("CharLowerBuffW", "user32", "Bd", 'd', 0, 0, 0, 0, 0),
    A("CharUpperBuffW", "user32", "Bd", 'd', 0, 0, 0, 0, 0),
    A("CharLowerW", "user32", "B", 'q', 0, 0, HK_CHARPTR, 0, 0),
    A("CharNextW", "user32", "W", 'q', 0, 0, HK_CHARPTR, 0, 0),
    A("LoadStringW", "user32", "idOd", 'd', DEC_WOUT, 2, 0, 0, 0),
    A("LoadImageW", "user32", "iPdddd", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("LoadIconW", "user32", "iP", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("LoadCursorW", "user32", "iP", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("DestroyCursor", "user32", "i", 'd', 0, 0, 0, 0, 0),
    A("DestroyIcon", "user32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("CreateIconIndirect", "user32", "S", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("IntersectRect", "user32", "OSS", 'd', DEC_RECT, 0, 0, 0, 0),
    A("GetClassInfoW", "user32", "iWO", 'd', 0, 0, 0, 0, 0),
    A("GetClipboardFormatNameW", "user32", "dOd", 'd', DEC_WOUT, 1, 0, 0, 0),
    A("GetDC", "user32", "i", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("ReleaseDC", "user32", "ii", 'd', 0, 0, 0, 0, 0),
    A("GetClipCursor", "user32", "O", 'd', DEC_RECT, 0, 0, 0, 0),
    A("GetKeyboardLayout", "user32", "d", 'q', 0, 0, 0, 0, 0),
    A("EnumDisplayMonitors", "user32", "iSFP", 'd', DEC_CBTEXT, 0, 0, 0, 0),
    A("EnumDisplaySettingsW", "user32", "WdO", 'd', DEC_DEVMODE, 2, 0, 0, 0),
    A("GetMonitorInfoW", "user32", "iO", 'd', DEC_MONINFO, 1, 0, 0, 0),
    A("MonitorFromPoint", "user32", "id", 'q', 0, 0, 0, F_HELPER, 0),
    /* gdi32 (memory DCs, memory bitmaps) */
    A("GetDeviceCaps", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("GetStockObject", "gdi32", "d", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreateCompatibleDC", "gdi32", "i", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("DeleteDC", "gdi32", "i", 'd', 0, 0, 0, 0, 0),
    A("DeleteObject", "gdi32", "i", 'd', 0, 0, 0, 0, 0),
    A("SelectObject", "gdi32", "ii", 'q', 0, 0, 0, 0, 0),
    A("SelectPalette", "gdi32", "iid", 'q', 0, 0, 0, 0, 0),
    A("GetObjectW", "gdi32", "idO", 'd', DEC_DIBSECTION, 2, 0, 0, 0),
    A("CreateBitmap", "gdi32", "ddddS", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreateBrushIndirect", "gdi32", "S", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreatePenIndirect", "gdi32", "S", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreatePalette", "gdi32", "S", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreateFontIndirectW", "gdi32", "S", 'q', 0, 0, HK_GDIOBJ, 0, 0),
    A("CreateDIBSection", "gdi32", "iSdOid", 'q', 0, 0, HK_DIB, 0, 0),
    A("GdiFlush", "gdi32", "", 'd', 0, 0, 0, 0, 0),
    A("GetCurrentPositionEx", "gdi32", "iO", 'd', DEC_POINT, 1, 0, 0, 0),
    A("MoveToEx", "gdi32", "iddO", 'd', 0, 0, 0, 0, 0),
    A("GetDIBColorTable", "gdi32", "iddO", 'd', 0, 0, 0, 0, 0),
    A("PatBlt", "gdi32", "iddddd", 'd', 0, 0, 0, 0, 0),
    A("StretchDIBits", "gdi32", "iddddddddSSdd", 'd', 0, 0, 0, 0, 0),
    A("SetBkColor", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("SetTextColor", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("SetBkMode", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("SetROP2", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("SetStretchBltMode", "gdi32", "id", 'd', 0, 0, 0, 0, 0),
    A("UnrealizeObject", "gdi32", "i", 'd', 0, 0, 0, 0, 0),
    A("EnumFontFamiliesExW", "gdi32", "iSFPd", 'd', DEC_CBTEXT, 0, 0, 0, 0),
    A("GetObjectType", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetTextColor", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetBkColor", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetBkMode", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetROP2", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetMapMode", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetStretchBltMode", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetPolyFillMode", "gdi32", "i", 'd', 0, 0, 0, F_HELPER, 0),
    A("GetCurrentObject", "gdi32", "id", 'q', 0, 0, 0, F_HELPER, 0),
    /* oleaut32 / ole32 / shcore / msacm32 */
    A("SysAllocStringLen", "oleaut32", "Wd", 'q', 0, 0, HK_BSTR, 0, 0),
    A("SysReAllocStringLen", "oleaut32", "BWd", 'd', 0, 0, HK_BSTRRE, 0, 0),
    A("SysFreeString", "oleaut32", "P", 'v', 0, 0, HK_BSTRFREE, 0, 0),
    A("OleInitialize", "ole32", "P", 'd', 0, 0, 0, 0, 0),
    A("OleUninitialize", "ole32", "", 'v', 0, 0, 0, F_HELPER, 0),
    A("GetProcessDpiAwareness", "shcore", "iO", 'd', DEC_U32, 1, 0, 0, "awareness"),
    A("acmMetrics", "msacm32", "idO", 'd', DEC_U32, 2, 0, 0, "value"),
};
#undef A
#define NAPI ((int)(sizeof k_api / sizeof k_api[0]))
static const Api *api_find(const char *n) { int i; for (i = 0; i < NAPI; i++) if (!strcmp(k_api[i].name, n)) return &k_api[i]; return NULL; }

/* ------------------------------------------------------------------ builtin callbacks (read-only enumeration) */
static SB g_cb;
static int g_cb_count;
static BOOL CALLBACK mon_cb(HMONITOR m, HDC dc, LPRECT r, LPARAM lp) {
    (void)m; (void)dc; (void)lp;
    g_cb_count++;
    if (g_cb_count <= 8) { sb_sep(&g_cb, " "); sb_printf(&g_cb, "mon%d(%ld, %ld, %ld, %ld)", g_cb_count, r->left, r->top, r->right, r->bottom); }
    return TRUE;
}
static int CALLBACK font_cb(const LOGFONTW *lf, const TEXTMETRICW *tm, DWORD type, LPARAM lp) {
    (void)tm; (void)type; (void)lp;
    g_cb_count++;
    if (g_cb_count <= 6) { sb_sep(&g_cb, " "); sb_cat(&g_cb, "face=\""); cat_wesc(&g_cb, lf->lfFaceName, 32); sb_cat(&g_cb, "\""); }
    return 1;
}
static void selfcode_marker(void) { }

/* ------------------------------------------------------------------ token parsing */
static char *split_field(char **p) {
    char *s = *p, *c;
    if (!s) return NULL;
    c = strchr(s, ':');
    if (c) { *c = 0; *p = c + 1; } else *p = NULL;
    return s;
}
static char *next_tok(char **save, char sep) {
    char *s = *save, *e;
    if (!s) return NULL;
    e = strchr(s, sep);
    if (e) { *e = 0; *save = e + 1; } else *save = NULL;
    return s;
}
static int apply_patches(u8 *buf, size_t n, const char *spec) {
    char tmp[8192]; char *tok, *save;
    if (strlen(spec) >= sizeof tmp) return -1;
    strcpy(tmp, spec); save = tmp;
    while ((tok = next_tok(&save, ',')) != NULL) {
        char *eq = strchr(tok, '=');
        size_t off; long len;
        if (!eq) return -1;
        *eq = 0; off = (size_t)strtoul(tok, NULL, 0);
        if (off > n) return -1;
        len = hex2bin(eq + 1, buf + off, n - off);
        if (len < 0) return -1;
    }
    return 0;
}
static int make_buf(Ctx *c, int idx, size_t n, int fill, const u8 *init, size_t initn, const char *name, u64 *out) {
    u8 *raw, *u = alloc16(n, &raw);
    if (fill >= 0) memset(u, fill, n);
    if (init) memcpy(u, init, initn < n ? initn : n);
    if (!ctx_buf(c, idx, u, n, raw, 1)) return -1;
    if (name && *name) var_set(name, (u64)(size_t)u, u, n, VF_NONE);
    *out = (u64)(size_t)u; return 0;
}
static int parse_tok(Ctx *c, int idx, char *tok, u64 *out) {
    size_t tl = strlen(tok);
    *out = 0;
    if (!strcmp(tok, "NULL")) return 0;
    if (tok[0] == '$') {
        Var *v = var_find(tok + 1);
        if (!v) { c->why = "unknown-variable"; return -1; }
        if (v->buf) { if (!ctx_buf(c, idx, v->buf, v->size, NULL, 0)) return -1; }
        *out = v->val; return 0;
    }
    if (tl < 2 || tok[1] != ':') { c->why = "bad-token"; return -1; }
    {
        char kind = tok[0]; char *rest = tok + 2;
        switch (kind) {
        case 'd': *out = (u64)(unsigned)parse_num(rest); return 0;
        case 'i': *out = parse_num(rest); return 0;
        case 'W': {
            u8 *tmp = (u8 *)malloc(strlen(rest) + 4); size_t n = pct_decode(rest, tmp, strlen(rest) + 1); int wl, r;
            wchar_t *w;
            tmp[n] = 0;
            w = (wchar_t *)calloc(n + 8, sizeof(wchar_t));
            wl = n ? MultiByteToWideChar(CP_UTF8, 0, (const char *)tmp, (int)n, w, (int)n + 4) : 0;
            r = make_buf(c, idx, (size_t)(wl + 8) * 2, -1, (u8 *)w, (size_t)wl * 2, NULL, out);
            free(tmp); free(w); return r;
        }
        case 'U': {
            size_t hl = strlen(rest); u8 *tmp = (u8 *)malloc(hl / 2 + 2); long n = hex2bin(rest, tmp, hl / 2 + 1); int r;
            if (n < 0) { free(tmp); c->why = "bad-hex"; return -1; }
            r = make_buf(c, idx, (size_t)n + 16, -1, tmp, (size_t)n, NULL, out); free(tmp); return r;
        }
        case 'A': {
            u8 *tmp = (u8 *)malloc(strlen(rest) + 4); size_t n = pct_decode(rest, tmp, strlen(rest) + 1); int r;
            r = make_buf(c, idx, n + 8, -1, tmp, n, NULL, out); free(tmp); return r;
        }
        case 'X': {
            size_t hl = strlen(rest); u8 *tmp = (u8 *)malloc(hl / 2 + 2); long n = hex2bin(rest, tmp, hl / 2 + 1); int r;
            if (n < 0) { free(tmp); c->why = "bad-hex"; return -1; }
            r = make_buf(c, idx, (size_t)n + 8, -1, tmp, (size_t)n, NULL, out); free(tmp); return r;
        }
        case 'Z': case 'O': {
            char *p = rest, *f1 = split_field(&p), *f2 = split_field(&p), *f3 = split_field(&p);
            size_t n = (size_t)strtoul(f1, NULL, 0); const char *patch = NULL, *name = NULL; u8 *raw, *u;
            if (n > (1u << 22)) { c->why = "buffer-too-large"; return -1; }
            if (f2 && strchr(f2, '=')) patch = f2; else if (f2 && *f2) name = f2;
            if (f3 && strchr(f3, '=')) patch = f3; else if (f3 && *f3) name = f3;
            u = alloc16(n, &raw);
            if (kind == 'O') memset(u, 0xCD, n);
            if (patch && apply_patches(u, n, patch)) { free(raw); c->why = "bad-patch"; return -1; }
            if (!ctx_buf(c, idx, u, n, raw, 1)) return -1;
            if (name && *name) var_set(name, (u64)(size_t)u, u, n, VF_NONE);
            *out = (u64)(size_t)u; return 0;
        }
        case 'D': {
            char *p = rest, *f1 = split_field(&p), *f2 = split_field(&p);
            u8 init[8]; u64 v = parse_num(f1);
            memset(init, 0, 8); memcpy(init, &v, 8);
            return make_buf(c, idx, 16, 0, init, 8, f2, out);
        }
        case 'M': {
            DWORD le; HMODULE h = get_module(rest, &le);
            if (!h) { c->why = "module-not-found"; snprintf(c->note, sizeof c->note, "le=%lu", (unsigned long)le); return -2; }
            *out = (u64)(size_t)h; return 0;
        }
        case 'F':
            if (!strcmp(rest, "monitors")) { *out = (u64)(size_t)mon_cb; return 0; }
            if (!strcmp(rest, "fonts")) { *out = (u64)(size_t)font_cb; return 0; }
            if (!strcmp(rest, "none")) { *out = 0; return 0; }
            c->why = "unknown-callback"; return -1;
        default: c->why = "bad-token-type"; return -1;
        }
    }
}

/* ------------------------------------------------------------------ policy: argument checks for the few APIs that could do more than read */
static int wstr_is_unc_or_device(const wchar_t *w) {
    if (!w) return 0;
    if (w[0] == L'\\' && w[1] == L'\\') return 1;
    if (w[0] == L'/' && w[1] == L'/') return 1;
    return 0;
}
static int policy(const Api *ap, Ctx *c) {
    const char *n = ap->name;
    u64 *a = c->a;
    if (!strncmp(n, "Heap", 4) && strcmp(n, "HeapCreate")) {
        Var *hv = var_by_val(a[0]);
        if (!hv || !(hv->flags & VF_HEAP)) { c->why = "heap-handle-not-ours"; return -1; }
    }
    if (!strcmp(n, "SystemParametersInfoW")) {
        unsigned act = (unsigned)a[0];
        /* get-only actions found in the trace (names from winuser.h) */
        switch (act) {
        case 0x101a: /* SPI_GETCURSORSHADOW */ case 0x1f: /* SPI_GETICONTITLELOGFONT */ case 0x200a: /* SPI_GETFONTSMOOTHINGTYPE */
        case 0x200c: /* SPI_GETFONTSMOOTHINGCONTRAST */ case 0x2012: /* SPI_GETFONTSMOOTHINGORIENTATION */ case 0x29: /* SPI_GETNONCLIENTMETRICS */
        case 0x30: /* SPI_GETWORKAREA */ case 0x68: /* SPI_GETWHEELSCROLLLINES */
        case 0x1: /* SPI_GETBEEP */ case 0x4a: /* SPI_GETFONTSMOOTHING */ case 0x3: /* SPI_GETMOUSE (3 ints) */ case 0x1000: /* SPI_GETACTIVEWINDOWTRACKING */
        case 0x1002: /* SPI_GETMENUANIMATION */ case 0x1008: /* SPI_GETGRADIENTCAPTIONS */ case 0x100a: /* SPI_GETKEYBOARDCUES */ case 0x1012: /* SPI_GETMENUFADE */
        case 0x2000: /* SPI_GETFOREGROUNDLOCKTIMEOUT */ case 0x2006: /* SPI_GETCARETWIDTH */ case 0x2010: /* SPI_GETFOCUSBORDERHEIGHT */ case 0x200e: /* SPI_GETFOCUSBORDERWIDTH */
            break;
        default: c->why = "spi-action-not-allowed"; return -1;
        }
        if (a[3] != 0) { c->why = "spi-fWinIni-nonzero"; return -1; }
        if (!a[2]) { c->why = "spi-null-param"; return -1; }
    } else if (!strcmp(n, "RegOpenKeyExW") || !strcmp(n, "RegOpenKeyExA")) {
        char *s = c->note;
        sprintf(s, "sam_orig=0x%x", (unsigned)a[3]);
        a[3] = 0x20019; /* KEY_READ */
        if (!(a[0] == 0xffffffff80000001ull || a[0] == 0x80000001ull || a[0] == 0xffffffff80000002ull || a[0] == 0x80000002ull || (var_by_val(a[0]) && (var_by_val(a[0])->flags & VF_REGKEY)))) { c->why = "reg-root-not-allowed"; return -1; }
    } else if (!strcmp(n, "RegQueryValueExW") || !strcmp(n, "RegQueryInfoKeyW") || !strcmp(n, "RegCloseKey")) {
        Var *v = var_by_val(a[0]);
        if (!v || !(v->flags & VF_REGKEY)) { c->why = "reg-handle-not-ours"; return -1; }
    } else if (!strcmp(n, "SHGetFolderPathW")) {
        if (a[1] & 0x8000) { c->why = "csidl-flag-create"; return -1; }
        if (a[3] != 0 || a[2] != 0) { c->why = "token-or-hwnd-nonzero"; return -1; }
    } else if (!strcmp(n, "LoadLibraryW") || !strcmp(n, "LoadLibraryExW") || !strcmp(n, "GetModuleHandleW")) {
        char nm[160]; const wchar_t *w = (const wchar_t *)(size_t)a[0]; int i;
        if (!w) { if (!strcmp(n, "GetModuleHandleW")) return 0; c->why = "null-module-name"; return -1; }
        for (i = 0; w[i] && i < 150; i++) nm[i] = (char)(w[i] < 128 ? w[i] : '?');
        nm[i] = 0;
        if (!is_sysdll(nm)) { c->why = "not-a-system-dll"; return -1; }
        if (!strcmp(n, "LoadLibraryExW") && ((a[2] & ~0x800ull) || a[1])) { c->why = "loadlibraryex-flags"; return -1; }
    } else if (!strcmp(n, "LoadLibraryA") || !strcmp(n, "GetModuleHandleA")) {
        const char *s = (const char *)(size_t)a[0];
        if (!s) { if (!strcmp(n, "GetModuleHandleA")) return 0; c->why = "null-module-name"; return -1; }
        if (!is_sysdll(s)) { c->why = "not-a-system-dll"; return -1; }
    } else if (!strcmp(n, "FreeLibrary")) {
        Var *v = var_by_val(a[0]);
        if (!v || !(v->flags & VF_OWNED)) { c->why = "module-not-loaded-by-us"; return -1; }
    } else if (!strcmp(n, "GetModuleFileNameW") || !strcmp(n, "GetModuleFileNameA")) {
        if (a[0]) {
            int i, ok = 0;
            if (a[0] == (u64)(size_t)GetModuleHandleW(NULL)) ok = 1;
            for (i = 0; i < g_nmods; i++) if ((u64)(size_t)g_mods[i].h == a[0]) ok = 1;
            { Var *v = var_by_val(a[0]); if (v && (v->flags & VF_OWNED)) ok = 1; }
            if (!ok) { c->why = "module-handle-not-ours"; return -1; }
        }
    } else if (!strcmp(n, "GetFileAttributesW") || !strcmp(n, "GetFileAttributesExW") || !strcmp(n, "FindFirstFileW") || !strcmp(n, "GetFileVersionInfoSizeW")) {
        if (wstr_is_unc_or_device((const wchar_t *)(size_t)a[0])) { c->why = "unc-path-not-allowed"; return -1; }
        if (!strcmp(n, "GetFileAttributesExW") && a[1] != 0) { c->why = "info-level"; return -1; }
    } else if (!strcmp(n, "GetEnvironmentVariableW")) {
        static const wchar_t *ok[] = { L"LANG", L"SystemRoot", L"windir", L"OS", L"PROCESSOR_ARCHITECTURE", L"NUMBER_OF_PROCESSORS", L"PATHEXT", L"ComSpec", L"REPLAY_NO_SUCH_VAR", NULL };
        const wchar_t *w = (const wchar_t *)(size_t)a[0]; int i, f = 0;
        for (i = 0; ok[i]; i++) if (w && !wcscmp(w, ok[i])) f = 1;
        if (!f) { c->why = "env-name-not-allowed"; return -1; }
    } else if (!strcmp(n, "SetThreadLocale")) {
        if (!(a[0] == 0x400 || a[0] == 0x409 || var_by_val(a[0]))) { c->why = "locale-not-allowed"; return -1; }
    } else if (!strcmp(n, "SetThreadErrorMode")) {
        if (a[0] > 1) { c->why = "threaderrormode-bits"; return -1; }
    } else if (!strcmp(n, "SetErrorMode")) {
        if (a[0] & ~0x8003ull) { c->why = "errormode-bits"; return -1; }
    } else if (!strcmp(n, "SetDllDirectoryW")) {
        const wchar_t *w = (const wchar_t *)(size_t)a[0];
        if (w && wcscmp(w, L"Shared\\") && w[0]) { c->why = "dlldir-not-allowed"; return -1; }
    } else if (!strcmp(n, "OleInitialize")) {
        if (a[0]) { c->why = "ole-reserved-nonnull"; return -1; }
    } else if (!strcmp(n, "VirtualAlloc")) {
        if (a[0] != 0 || (a[2] & ~0x103000ull) || a[3] != 4 || a[1] == 0 || a[1] > (16u << 20)) { c->why = "valloc-args"; return -1; }
    } else if (!strcmp(n, "VirtualProtect")) {
        Var *v = var_find("code");
        if (!v || !(v->flags & VF_CODE) || a[0] < v->val || a[0] + a[1] > v->val + 0x1000 || a[1] > 16) { c->why = "vprotect-not-our-code-page"; return -1; }
        if (a[2] != 0x40 && a[2] != 0x20 && a[2] != 0x04) { c->why = "vprotect-prot"; return -1; }
    } else if (!strcmp(n, "HeapAlloc") || !strcmp(n, "LocalAlloc")) {
        u64 sz = !strcmp(n, "HeapAlloc") ? a[2] : a[1];
        if (sz > (64u << 20)) { c->why = "alloc-too-large"; return -1; }
        if (!strcmp(n, "HeapAlloc") && (a[1] & 4)) { c->why = "heap-generate-exceptions"; return -1; }
    } else if (!strcmp(n, "HeapReAlloc")) {
        if (a[3] > (64u << 20)) { c->why = "alloc-too-large"; return -1; }
        if (a[1] & 4) { c->why = "heap-generate-exceptions"; return -1; }
        if (!reg_has(a[2])) { c->why = "ptr-not-ours"; return -1; }
    } else if (!strcmp(n, "HeapFree") || !strcmp(n, "HeapSize")) {
        if (a[2] && !reg_has(a[2])) { c->why = "ptr-not-ours"; return -1; }
    } else if (!strcmp(n, "LocalFree")) {
        if (a[0] && !reg_has(a[0])) { c->why = "ptr-not-ours"; return -1; }
    } else if (!strcmp(n, "HeapCreate")) {
        if (a[2] > (16u << 20) || (a[0] & ~1ull)) { c->why = "heapcreate-args"; return -1; }
    } else if (!strcmp(n, "HeapSetInformation")) {
        if (a[1] != 0) { c->why = "heapinfo-class"; return -1; }
    } else if (!strcmp(n, "PatBlt") || !strcmp(n, "StretchDIBits") || !strcmp(n, "MoveToEx") || !strcmp(n, "SetBkColor") || !strcmp(n, "SetTextColor") ||
               !strcmp(n, "SetBkMode") || !strcmp(n, "SetROP2") || !strcmp(n, "SetStretchBltMode") || !strcmp(n, "SelectObject") || !strcmp(n, "SelectPalette") ||
               !strcmp(n, "DeleteDC")) {
        Var *v = var_by_val(a[0]);
        if (!v || !(v->flags & VF_MEMDC)) { c->why = "dc-not-a-memory-dc-of-ours"; return -1; }
    } else if (!strcmp(n, "GetDC")) {
        if (a[0] != 0) { c->why = "getdc-hwnd-nonnull"; return -1; }
    } else if (!strcmp(n, "ReleaseDC")) {
        if (a[0] != 0) { c->why = "releasedc-hwnd-nonnull"; return -1; }
    } else if (!strcmp(n, "CreateCompatibleDC")) {
        if (a[0] != 0 && !var_by_val(a[0])) { c->why = "createcompatibledc-arg"; return -1; }
    } else if (!strcmp(n, "DeleteObject") || !strcmp(n, "DestroyCursor") || !strcmp(n, "DestroyIcon") || !strcmp(n, "UnrealizeObject")) {
        Var *v = var_by_val(a[0]);
        if (!v || !(v->flags & VF_OWNED)) { c->why = "object-not-created-by-us"; return -1; }
    } else if (!strcmp(n, "LoadImageW")) {
        if (a[0] != 0) { c->why = "loadimage-hinstance"; return -1; }
    } else if (!strcmp(n, "LoadIconW") || !strcmp(n, "LoadCursorW")) {
        if (a[0] != 0) { c->why = "load-hinstance"; return -1; }
    } else if (!strcmp(n, "LoadStringW")) {
        if (a[0] != (u64)(size_t)GetModuleHandleW(NULL)) { c->why = "loadstring-hinstance"; return -1; }
    } else if (!strcmp(n, "GetClassInfoW")) {
        if (a[0] != (u64)(size_t)GetModuleHandleW(NULL) && a[0] != 0) { c->why = "getclassinfo-hinstance"; return -1; }
    } else if (!strcmp(n, "EnumDisplayMonitors")) {
        if (a[0] != 0 || a[1] != 0) { c->why = "enumdisplaymonitors-args"; return -1; }
        if (a[2] != (u64)(size_t)mon_cb) { c->why = "callback-not-builtin"; return -1; }
    } else if (!strcmp(n, "EnumFontFamiliesExW")) {
        if (a[2] != (u64)(size_t)font_cb) { c->why = "callback-not-builtin"; return -1; }
    } else if (!strcmp(n, "EnumDisplaySettingsW")) {
        if (a[0] != 0) { c->why = "devicename-nonnull"; return -1; }
    } else if (!strcmp(n, "GetProcessDpiAwareness")) {
        if (a[0] != 0 && a[0] != (u64)-1) { c->why = "dpi-process-handle"; return -1; }
    } else if (!strcmp(n, "FlsAlloc")) {
        if (a[0] != 0) { c->why = "fls-callback"; return -1; }
    } else if (!strcmp(n, "SysFreeString")) {
        if (!a[0] || !reg_has(a[0] - 4)) { c->why = "bstr-not-ours"; return -1; }
    } else if (!strcmp(n, "SysReAllocStringLen")) {
        u64 pb = 0; Var *bv = NULL; int i;
        for (i = 0; i < g_nvars; i++) if (g_vars[i].buf && (u64)(size_t)g_vars[i].buf == a[0]) bv = &g_vars[i];
        if (!bv || bv->size < 8) { c->why = "bstr-box-not-ours"; return -1; }
        memcpy(&pb, bv->buf, 8);
        if (pb && !reg_has(pb - 4)) { c->why = "bstr-not-ours"; return -1; }
        if (a[2] > 4096) { c->why = "bstr-len"; return -1; }
    } else if (!strcmp(n, "SysAllocStringLen")) {
        if (a[1] > 4096) { c->why = "bstr-len"; return -1; }
    } else if (!strcmp(n, "TlsFree") || !strcmp(n, "FlsFree")) {
        /* helpers: arguments come from $variables */
    }
    return 0;
}

/* ------------------------------------------------------------------ decoders */
static const u8 *arg_mem(Ctx *c, int idx, size_t need) {
    int i;
    for (i = 0; i < c->nb; i++) if (c->b[i].arg == idx && c->b[i].size >= need) return c->b[i].user;
    return NULL;
}
static size_t arg_size(Ctx *c, int idx) { int i; for (i = 0; i < c->nb; i++) if (c->b[i].arg == idx) return c->b[i].size; return 0; }
static unsigned rd32(const u8 *p, size_t o) { unsigned v; memcpy(&v, p + o, 4); return v; }
static int rd32s(const u8 *p, size_t o) { int v; memcpy(&v, p + o, 4); return v; }
static unsigned rd16(const u8 *p, size_t o) { unsigned short v; memcpy(&v, p + o, 2); return v; }
static u64 rd64(const u8 *p, size_t o) { u64 v; memcpy(&v, p + o, 8); return v; }
static void cat_hx(SB *s, u64 v) { char t[40]; sb_cat(s, hx(v, t)); }
static void cat_wfield(SB *s, const u8 *p, size_t o, size_t maxw) { wchar_t w[300]; if (maxw > 299) maxw = 299; memcpy(w, p + o, maxw * 2); w[maxw] = 0; sb_cat(s, "\""); cat_wesc(s, w, maxw); sb_cat(s, "\""); }

static void decode_out(const Api *ap, Ctx *c, u64 ret, int le, SB *d) {
    const u8 *m; char t[40];
    (void)le;
    switch (ap->dec) {
    case DEC_WOUT: {
        int k = ap->deca; size_t sz = arg_size(c, k); const wchar_t *w = (const wchar_t *)arg_mem(c, k, 2);
        if (w && sz >= 2) {
            SB tmp = { 0, 0, 0 }; size_t i; char *q;
            cat_wesc(&tmp, w, sz / 2);
            q = tmp.p ? tmp.p : (char *)"";
            { char buf[4096]; scpy(buf, sizeof buf, q); buf[sizeof buf - 1] = 0; mask_text(buf, sizeof buf); sb_sep(d, " "); sb_printf(d, "out%d=\"%s\"", k, buf); }
            (void)i; free(tmp.p);
        }
        break;
    }
    case DEC_AOUT: {
        int k = ap->deca; size_t sz = arg_size(c, k); const char *a = (const char *)arg_mem(c, k, 1);
        if (a && sz) { SB tmp = { 0, 0, 0 }; char buf[4096]; cat_aesc(&tmp, a, sz); scpy(buf, sizeof buf, tmp.p ? tmp.p : ""); buf[sizeof buf - 1] = 0; mask_text(buf, sizeof buf); sb_sep(d, " "); sb_printf(d, "out%d=\"%s\"", k, buf); free(tmp.p); }
        break;
    }
    case DEC_COMPNAME: {
        const wchar_t *w = (const wchar_t *)arg_mem(c, 1, 2); size_t sz = arg_size(c, 1);
        if (w && (ret & 0xffffffff)) {
            size_t i, n = 0, up = 0, lo = 0, dg = 0, hy = 0, ot = 0;
            for (i = 0; i < sz / 2 && w[i]; i++, n++) { wchar_t ch = w[i]; if (ch >= 'A' && ch <= 'Z') up++; else if (ch >= 'a' && ch <= 'z') lo++; else if (ch >= '0' && ch <= '9') dg++; else if (ch == '-') hy++; else ot++; }
            sb_sep(d, " "); sb_printf(d, "out1=<masked len=%u upper=%u lower=%u digit=%u hyphen=%u other=%u>", (unsigned)n, (unsigned)up, (unsigned)lo, (unsigned)dg, (unsigned)hy, (unsigned)ot);
        }
        break;
    }
    case DEC_SYSINFO:
        if ((m = arg_mem(c, 0, 48)) != NULL) {
            sb_sep(d, " "); sb_printf(d, "SYSTEM_INFO{arch=%u page=", rd16(m, 0)); cat_hx(d, rd32(m, 4)); sb_cat(d, " minaddr="); cat_hx(d, rd64(m, 8));
            sb_cat(d, " maxaddr="); cat_hx(d, rd64(m, 16)); sb_cat(d, " mask="); cat_hx(d, rd64(m, 24)); sb_printf(d, " nproc=%u proctype=%u granularity=", rd32(m, 32), rd32(m, 36));
            cat_hx(d, rd32(m, 40)); sb_printf(d, " level=%u revision=", rd16(m, 44)); cat_hx(d, rd16(m, 46)); sb_cat(d, "}");
        }
        break;
    case DEC_OSVIW: case DEC_OSVIA: {
        size_t need = ap->dec == DEC_OSVIW ? 276 : 148;
        if ((m = arg_mem(c, 0, need)) != NULL) {
            sb_sep(d, " "); sb_printf(d, "OSVERSIONINFO{size=%u major=%u minor=%u build=%u platform=%u csd=", rd32(m, 0), rd32(m, 4), rd32(m, 8), rd32(m, 12), rd32(m, 16));
            if (ap->dec == DEC_OSVIW) cat_wfield(d, m, 20, 128); else { char a[129]; memcpy(a, m + 20, 128); a[128] = 0; sb_cat(d, "\""); cat_aesc(d, a, 128); sb_cat(d, "\""); }
            sb_cat(d, "}");
            if (arg_size(c, 0) >= (ap->dec == DEC_OSVIW ? 284u : 156u)) {
                size_t o = need; sb_printf(d, " EX{spmajor=%u spminor=%u suite=0x%x product=%u}", rd16(m, o), rd16(m, o + 2), rd16(m, o + 4), m[o + 6]);
            }
        }
        break;
    }
    case DEC_TZ:
        if ((m = arg_mem(c, 0, 172)) != NULL) { sb_sep(d, " "); sb_printf(d, "TIME_ZONE_INFORMATION{bias=%d standard=", rd32s(m, 0)); cat_wfield(d, m, 4, 32); sb_cat(d, " daylight="); cat_wfield(d, m, 88, 32); sb_cat(d, "}"); }
        break;
    case DEC_SYSTIME:
        if ((m = arg_mem(c, 0, 16)) != NULL) { sb_sep(d, " "); sb_printf(d, "SYSTEMTIME{%04u-%02u-%02u %02u:%02u:%02u.%03u dow=%u}", rd16(m, 0), rd16(m, 2), rd16(m, 6), rd16(m, 8), rd16(m, 10), rd16(m, 12), rd16(m, 14), rd16(m, 4)); }
        break;
    case DEC_FILETIME:
        if ((m = arg_mem(c, 0, 8)) != NULL) { u64 ft = rd64(m, 0); sb_sep(d, " "); sb_printf(d, "FILETIME{0x%llx = unix %.3f s}", ft, ((double)ft - 116444736000000000.0) / 1e7); }
        break;
    case DEC_U64:
        if ((m = arg_mem(c, 0, 8)) != NULL) { sb_sep(d, " "); sb_printf(d, "value=%llu", rd64(m, 0)); }
        break;
    case DEC_STARTUP:
        if ((m = arg_mem(c, 0, 0x68)) != NULL) { int i, nz = 0; for (i = 0; i < 0x68; i++) if (m[i]) nz++; sb_sep(d, " "); sb_printf(d, "STARTUPINFO{cb="); cat_hx(d, rd32(m, 0)); sb_printf(d, " nonzero_bytes_in_first_0x68=%d}", nz); }
        break;
    case DEC_CPINFO:
        if ((m = arg_mem(c, 1, 18)) != NULL) { int i, nz = 0; for (i = 6; i < 18; i++) if (m[i]) nz = 1; sb_sep(d, " "); sb_printf(d, "CPINFO{maxcharsize=%u defaultchar=%02x%02x leadbytes_nonzero=%s}", rd32(m, 0), m[4], m[5], nz ? "True" : "False"); }
        break;
    case DEC_SPI: {
        unsigned act = (unsigned)c->a[0];
        m = arg_mem(c, 2, 4);
        if (!m) break;
        sb_sep(d, " ");
        if (act == 0x68 || act == 0x101a || act == 0x200a || act == 0x200c || act == 0x2012 || act == 0x1 || act == 0x4a || act == 0x3 || act == 0x1000 || act == 0x1002 || act == 0x1008 ||
            act == 0x100a || act == 0x1012 || act == 0x2000 || act == 0x2006 || act == 0x2010 || act == 0x200e) { sb_cat(d, "val="); cat_hx(d, rd32(m, 0)); }
        else if (act == 0x30 && arg_size(c, 2) >= 16) sb_printf(d, "RECT{%d,%d,%d,%d}", rd32s(m, 0), rd32s(m, 4), rd32s(m, 8), rd32s(m, 12));
        else if (act == 0x1f && arg_size(c, 2) >= 92) { sb_printf(d, "LOGFONTW{height=%d weight=%d italic=%u charset=%u clip=%u quality=%u face=", rd32s(m, 0), rd32s(m, 16), m[20], m[23], m[25], m[26]); cat_wfield(d, m, 28, 32); sb_cat(d, "}"); }
        else if (act == 0x29 && arg_size(c, 2) >= 100) { sb_cat(d, "NONCLIENTMETRICSW{cbSize="); cat_hx(d, rd32(m, 0)); sb_printf(d, " border=%d scrollW=%d scrollH=%d captionW=%d captionH=%d captionFont.face=", rd32s(m, 4), rd32s(m, 8), rd32s(m, 12), rd32s(m, 16), rd32s(m, 20)); cat_wfield(d, m, 52, 32); sb_cat(d, "}"); }
        break;
    }
    case DEC_U32:
        if ((m = arg_mem(c, ap->deca, 4)) != NULL) { sb_sep(d, " "); sb_printf(d, "%s=", ap->ulabel); cat_hx(d, rd32(m, 0)); }
        break;
    case DEC_DEVMODE:
        if ((m = arg_mem(c, 2, 188)) != NULL) { sb_sep(d, " "); sb_printf(d, "DEVMODEW{bitsPerPel=%u w=%u h=%u flags=", rd32(m, 168), rd32(m, 172), rd32(m, 176)); cat_hx(d, rd32(m, 180)); sb_printf(d, " freq=%u}", rd32(m, 184)); }
        break;
    case DEC_MONINFO:
        if ((m = arg_mem(c, 1, 40)) != NULL) {
            sb_sep(d, " "); sb_printf(d, "MONITORINFO{cb=%u rcMonitor=(%d, %d, %d, %d) rcWork=(%d, %d, %d, %d) flags=", rd32(m, 0), rd32s(m, 4), rd32s(m, 8), rd32s(m, 12), rd32s(m, 16), rd32s(m, 20), rd32s(m, 24), rd32s(m, 28), rd32s(m, 32));
            cat_hx(d, rd32(m, 36)); sb_cat(d, "}");
        }
        break;
    case DEC_RECT: {
        int k = !strcmp(ap->name, "IntersectRect") ? 0 : 0;
        if ((m = arg_mem(c, k, 16)) != NULL) { sb_sep(d, " "); sb_printf(d, "RECT(%d, %d, %d, %d)", rd32s(m, 0), rd32s(m, 4), rd32s(m, 8), rd32s(m, 12)); }
        break;
    }
    case DEC_POINT:
        if ((m = arg_mem(c, ap->deca, 8)) != NULL) { sb_sep(d, " "); sb_printf(d, "POINT(%d, %d)", rd32s(m, 0), rd32s(m, 4)); }
        break;
    case DEC_LPI: {
        const u8 *lenp = arg_mem(c, 1, 4);
        m = arg_mem(c, 0, 1);
        if (lenp) {
            unsigned rl = rd32(lenp, 0);
            sb_sep(d, " ");
            if (m && (ret & 0xffffffff) == 1 && rl >= 32 && arg_size(c, 0) >= rl) {
                unsigned k; sb_printf(d, "ReturnedLength=%u entries=[", rl);
                for (k = 0; k < rl / 32; k++) {
                    const u8 *e = m + 32 * k; u64 mask = rd64(e, 0); unsigned rel = rd32(e, 8);
                    if (k) sb_cat(d, " ");
                    if (rel == 2) { sb_printf(d, "cache{mask="); cat_hx(d, mask); sb_printf(d, " L%u type=%d assoc=%u line=%u size=%u}", e[16], rd32s(e, 24), e[17], rd16(e, 18), rd32(e, 20)); }
                    else { const char *nm = rel == 0 ? "core" : rel == 1 ? "numa" : rel == 3 ? "package" : "rel"; sb_printf(d, "%s{mask=", nm); cat_hx(d, mask); sb_cat(d, "}"); }
                }
                sb_cat(d, "]");
            } else { sb_cat(d, "ReturnedLength="); cat_hx(d, rl); }
        }
        break;
    }
    case DEC_LANGS: {
        const u8 *cnt = arg_mem(c, 1, 4); const wchar_t *w = (const wchar_t *)arg_mem(c, 2, 2); size_t sz = arg_size(c, 2);
        if (w && c->a[2]) {
            size_t i = 0; int first = 1;
            sb_sep(d, " "); sb_cat(d, "langs=");
            while (i < sz / 2 && w[i]) { size_t j = i; SB tmp = { 0, 0, 0 }; while (j < sz / 2 && w[j]) j++; if (!first) sb_cat(d, "|"); first = 0; cat_wesc(&tmp, w + i, j - i); sb_cat(d, tmp.p ? tmp.p : ""); free(tmp.p); i = j + 1; }
        }
        if (cnt) { sb_sep(d, " "); sb_cat(d, "count="); cat_hx(d, rd32(cnt, 0)); }
        break;
    }
    case DEC_DIBSECTION: {
        size_t sz = arg_size(c, 2);
        m = arg_mem(c, 2, 1);
        if (!m) break;
        sb_sep(d, " ");
        if ((unsigned)c->a[1] == 104 && sz >= 104) {
            sb_printf(d, "DIBSECTION{bmType=%d bmWidth=%d bmHeight=%d bmWidthBytes=%d bmPlanes=%u bmBitsPixel=%u bmBits=%s biSize=%u biWidth=%d biHeight=%d biPlanes=%u biBitCount=%u biCompression=%u biSizeImage=%u biXPels=%d biYPels=%d biClrUsed=%u biClrImportant=%u masks=%08x,%08x,%08x hSection=%s dsOffset=%u}",
                rd32s(m, 0), rd32s(m, 4), rd32s(m, 8), rd32s(m, 12), rd16(m, 16), rd16(m, 18), rd64(m, 24) ? "nonnull" : "NULL", rd32(m, 32), rd32s(m, 36), rd32s(m, 40), rd16(m, 44), rd16(m, 46), rd32(m, 48), rd32(m, 52),
                rd32s(m, 56), rd32s(m, 60), rd32(m, 64), rd32(m, 68), rd32(m, 72), rd32(m, 76), rd32(m, 80), rd64(m, 88) ? "nonnull" : "NULL", rd32(m, 96));
        } else if ((unsigned)c->a[1] == 92 && sz >= 92) {
            sb_printf(d, "LOGFONTW{height=%d weight=%d italic=%u charset=%u clip=%u quality=%u face=", rd32s(m, 0), rd32s(m, 16), m[20], m[23], m[25], m[26]); cat_wfield(d, m, 28, 32); sb_cat(d, "}");
        } else if ((unsigned)c->a[1] == 32 && sz >= 32) {
            sb_printf(d, "BITMAP{bmType=%d bmWidth=%d bmHeight=%d bmWidthBytes=%d bmPlanes=%u bmBitsPixel=%u bmBits=%s}", rd32s(m, 0), rd32s(m, 4), rd32s(m, 8), rd32s(m, 12), rd16(m, 16), rd16(m, 18), rd64(m, 24) ? "nonnull" : "NULL");
        }
        break;
    }
    case DEC_CBTEXT:
        sb_sep(d, " "); sb_printf(d, "callbacks=%d", g_cb_count); if (g_cb.n) { sb_cat(d, " "); sb_cat(d, g_cb.p); }
        break;
    case DEC_CMDW: if (ret) { SB tmp = { 0, 0, 0 }; char buf[4096]; cat_wesc(&tmp, (const wchar_t *)(size_t)ret, 2000); scpy(buf, sizeof buf, tmp.p ? tmp.p : ""); buf[sizeof buf - 1] = 0; mask_text(buf, sizeof buf); sb_sep(d, " "); sb_printf(d, "ret->\"%s\"", buf); free(tmp.p); } break;
    case DEC_CMDA: if (ret) { SB tmp = { 0, 0, 0 }; char buf[4096]; cat_aesc(&tmp, (const char *)(size_t)ret, 2000); scpy(buf, sizeof buf, tmp.p ? tmp.p : ""); buf[sizeof buf - 1] = 0; mask_text(buf, sizeof buf); sb_sep(d, " "); sb_printf(d, "ret->\"%s\"", buf); free(tmp.p); } break;
    case DEC_ENVBLOCK:
        if (ret) {
            const wchar_t *e = (const wchar_t *)(size_t)ret; size_t nvars = 0, total = 0, eq0 = 0;
            while (e[total]) { size_t l = wcslen(e + total); if (e[total] == L'=') eq0++; nvars++; total += l + 1; }
            sb_sep(d, " "); sb_printf(d, "envblock{vars=%u chars_with_final_nul=%u leading_equals_entries=%u}", (unsigned)nvars, (unsigned)(total + 1), (unsigned)eq0);
        }
        break;
    default: break;
    }
    (void)t;
}

/* ------------------------------------------------------------------ output */
static FILE *g_out;
static char g_outpath[MAX_PATH], g_inflight[MAX_PATH + 16];
static char g_cur_id[64], g_cur_api[64] = "?";

typedef u64 (WINAPI *Fn12)(u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64);

static void put_row(const char *id, const char *api, const char *status, const char *ret, const char *le, const char *lech, const char *outs, const char *dec, const char *extra) {
    SB s = { 0, 0, 0 };
    sb_cat(&s, id); sb_cat(&s, "\t"); sb_cat(&s, api); sb_cat(&s, "\t"); clean_cat(&s, status); sb_cat(&s, "\t");
    clean_cat(&s, ret); sb_cat(&s, "\t"); sb_cat(&s, le); sb_cat(&s, "\t"); sb_cat(&s, lech); sb_cat(&s, "\t");
    clean_cat(&s, outs); sb_cat(&s, "\t"); clean_cat(&s, dec); sb_cat(&s, "\t"); clean_cat(&s, extra); sb_cat(&s, "\n");
    fputs(s.p, g_out); fflush(g_out); free(s.p);
}

static LONG WINAPI crash_filter(EXCEPTION_POINTERS *ep) {
    char st[64];
    sprintf(st, "crash:0x%08lx", (unsigned long)ep->ExceptionRecord->ExceptionCode);
    if (g_out) put_row(g_cur_id, g_cur_api, st, "", "", "", "", "", "");
    fprintf(stderr, "\nreplay: crash in case %s (%s); rerun with --resume\n", g_cur_id, st);
    ExitProcess(3);
    return EXCEPTION_EXECUTE_HANDLER;
}

static void hex_changed(SB *outs, Ctx *c, int noHex, SB *extra) {
    int i;
    for (i = 0; i < c->nb; i++) {
        Buf *b = &c->b[i]; size_t last = 0, k; int ch = 0;
        for (k = 0; k < b->size; k++) if (b->user[k] != b->before[k]) { last = k; ch = 1; }
        if (b->owned && b->raw) {
            int g; for (g = 0; g < 32; g++) if (b->user[b->size + g] != 0xAB) { sb_sep(extra, ";"); sb_printf(extra, "OVERRUN(a%d)", b->arg); break; }
        }
        if (!ch || noHex) continue;
        {
            size_t n = last + 1; int trunc = 0;
            if (n > 4096) { n = 4096; trunc = 1; }
            sb_sep(outs, ";"); sb_printf(outs, "a%d=", b->arg); sb_hex(outs, b->user, n); if (trunc) sb_cat(outs, "...");
        }
    }
}

static int exec_op(const char *id, char *api, char **toks, int nt, const char *capture) {
    /* replay-internal operations */
    Ctx c; SB outs = { 0, 0, 0 }, extra = { 0, 0, 0 }; u64 a[3] = { 0, 0, 0 }; int i, rc = 0; char retb[40]; const char *status = "ok";
    memset(&c, 0, sizeof c);
    for (i = 0; i < nt && i < 3; i++) if (parse_tok(&c, i, toks[i], &a[i]) < 0) { status = "err:bad-arg"; rc = -1; }
    strcpy(retb, "0");
    if (rc) { put_row(id, api, status, "", "", "", "", "", c.why ? c.why : ""); ctx_free(&c); return 0; }
    if (!strcmp(api, "@eq")) { sprintf(retb, "%d", a[0] == a[1]); }
    else if (!strcmp(api, "@selfcode")) {
        u64 p = ((u64)(size_t)selfcode_marker) & ~0xFFFull;
        sprintf(retb, "0x%llx", p); var_set("code", p, NULL, 0, VF_CODE);
    } else if (!strcmp(api, "@peek") || !strcmp(api, "@peekp")) {
        u64 p = a[0]; size_t n = (size_t)a[1];
        if (!strcmp(api, "@peekp")) { Var *v = nt > 0 ? var_find(toks[0] + 1) : NULL; if (v && v->buf && v->size >= 8) memcpy(&p, v->buf, 8); else p = 0; }
        if (n > 4096 || !p || !reg_ok(p, n)) { status = "skip:policy:peek-region"; }
        else { sb_cat(&outs, "m="); sb_hex(&outs, (const u8 *)(size_t)p, n); }
    } else if (!strcmp(api, "@bstr") || !strcmp(api, "@bstrp")) {
        u64 p = a[0];
        if (!strcmp(api, "@bstrp")) { Var *v = nt > 0 ? var_find(toks[0] + 1) : NULL; if (v && v->buf && v->size >= 8) memcpy(&p, v->buf, 8); else p = 0; }
        if (!p || !reg_ok(p - 4, 6)) status = "skip:policy:bstr-region";
        else { unsigned len; memcpy(&len, (void *)(size_t)(p - 4), 4); if (len > 4096 || !reg_ok(p - 4, len + 6)) status = "skip:policy:bstr-region"; else { sb_printf(&extra, "bstr_prefix=%u", len); sb_cat(&outs, "m="); sb_hex(&outs, (const u8 *)(size_t)p, len + 2); } }
    } else if (!strcmp(api, "@box")) {
        u8 *raw, *u = alloc16(8, &raw);
        memcpy(u, &a[0], 8);
        if (capture && *capture) var_set(capture, (u64)(size_t)u, u, 8, VF_NONE);
        sprintf(retb, "0x%llx", (u64)(size_t)u);
    } else if (!strcmp(api, "@poke")) {
        int k, done = 0;
        for (k = 0; k < g_nvars; k++) if (g_vars[k].buf && (u64)(size_t)g_vars[k].buf == a[0] && a[1] + 8 <= g_vars[k].size) { memcpy(g_vars[k].buf + a[1], &a[2], 8); done = 1; }
        if (!done) status = "skip:policy:poke-target";
    } else if (!strcmp(api, "@note")) { }
    else status = "skip:not-whitelisted";
    put_row(id, api, status, retb, "", "", outs.p ? outs.p : "", "", extra.p ? extra.p : "");
    free(outs.p); free(extra.p); ctx_free(&c); return 0;
}

static void post_hooks(const Api *ap, Ctx *c, u64 ret, SB *extra, const char *caseid) {
    char t[160]; (void)caseid;
    switch (ap->hook) {
    case HK_ALLOC_HEAP: case HK_ALLOC_LOCAL: {
        size_t sz = (size_t)(ap->hook == HK_ALLOC_HEAP ? c->a[2] : c->a[1]);
        if (ret) {
            unsigned al = 1; while (al < 4096 && !(ret & al)) al <<= 1;
            reg_add(ret, sz ? sz : 1);
            sb_sep(extra, ";"); sb_printf(extra, "align=%u", al);
            { size_t k, n = sz < 64 ? sz : 64; int z = 1; for (k = 0; k < n; k++) if (((u8 *)(size_t)ret)[k]) z = 0; sb_printf(extra, ";first%u_zero=%d", (unsigned)n, z); }
        }
        break;
    }
    case HK_REALLOC:
        if (ret) { unsigned al = 1; while (al < 4096 && !(ret & al)) al <<= 1; if (ret != c->a[2]) reg_del(c->a[2]); else reg_del(ret); reg_add(ret, (size_t)c->a[3]); sb_sep(extra, ";"); sb_printf(extra, "align=%u;moved=%d", al, ret != c->a[2]); }
        break;
    case HK_FREE_HEAP:
        if (c->a[ap->name[0] == 'H' ? 2 : 0]) reg_del(c->a[ap->name[0] == 'H' ? 2 : 0]);
        break;
    case HK_VALLOC:
        if (ret) {
            int z = 1; size_t k; for (k = 0; k < 64; k++) if (((u8 *)(size_t)ret)[k]) z = 0;
            sb_sep(extra, ";"); sb_printf(extra, "addr_mod_64k=0x%llx;first64_zero=%d;released=1", ret & 0xFFFF, z);
            VirtualFree((LPVOID)(size_t)ret, 0, MEM_RELEASE);
        }
        break;
    case HK_VPROTECT:
        if (ret & 0xffffffff) { const u8 *m = arg_mem(c, 3, 4); DWORD old = m ? rd32(m, 0) : 0x20, tmp = 0; VirtualProtect((LPVOID)(size_t)c->a[0], (SIZE_T)c->a[1], old, &tmp); sb_sep(extra, ";"); sb_printf(extra, "restored_prot=0x%lx", (unsigned long)old); }
        break;
    case HK_MODFILE: case HK_LOADLIB:
        if (ret) {
            wchar_t w[MAX_PATH]; DWORD n = GetModuleFileNameW((HMODULE)(size_t)ret, w, MAX_PATH);
            if (n && n < MAX_PATH) { SB tmp = { 0, 0, 0 }; char buf[4096]; cat_wesc(&tmp, w, n); scpy(buf, sizeof buf, tmp.p ? tmp.p : ""); buf[sizeof buf - 1] = 0; mask_text(buf, sizeof buf); sb_sep(extra, ";"); sb_printf(extra, "modfile=%s", buf); free(tmp.p); }
        }
        break;
    case HK_GDIOBJ:
        if (ret) {
            typedef DWORD (WINAPI *GOT)(HANDLE);
            DWORD le0; HMODULE g = get_module("gdi32", &le0); GOT f = g ? (GOT)GetProcAddress(g, "GetObjectType") : NULL;
            if (f) { sb_sep(extra, ";"); sb_printf(extra, "objtype=%lu", (unsigned long)f((HANDLE)(size_t)ret)); }
        }
        break;
    case HK_CHARPTR:
        if (ret && c->a[0] && ret >= c->a[0] && ret < c->a[0] + 4096) { sb_sep(extra, ";"); sb_printf(extra, "ret_off=%llu", ret - c->a[0]); }
        break;
    case HK_DIB: {
        const u8 *pp = arg_mem(c, 3, 8), *bmi = arg_mem(c, 1, 40);
        if (ret && pp && bmi) {
            u64 bits = rd64(pp, 0); int w = rd32s(bmi, 4), h = rd32s(bmi, 8); unsigned bc = rd16(bmi, 14);
            size_t stride = (((size_t)abs(w) * bc + 31) / 32) * 4, sz = stride * (size_t)abs(h);
            if (bits) { reg_add(bits, sz); sb_sep(extra, ";"); sb_printf(extra, "bits_align=%u;bits_size=%u", (unsigned)(bits & 0xFFF ? 1 : 4096), (unsigned)sz); }
        }
        break;
    }
    case HK_BSTR:
        if (ret) {
            unsigned len; memcpy(&len, (void *)(size_t)(ret - 4), 4);
            reg_add(ret - 4, (size_t)len + 6 < 4100 ? (size_t)len + 6 : 4100);
            sb_sep(extra, ";"); sb_printf(extra, "bstr_prefix=%u;ptr_mod_16=%u", len, (unsigned)(ret & 15));
        }
        break;
    case HK_BSTRRE: {
        const u8 *pp = arg_mem(c, 0, 8);
        if (pp && (ret & 0xffffffff) == 1) {
            u64 nb = rd64(pp, 0);
            if (nb) { unsigned len; memcpy(&len, (void *)(size_t)(nb - 4), 4); reg_add(nb - 4, (size_t)len + 6 < 4100 ? (size_t)len + 6 : 4100); sb_sep(extra, ";"); sb_printf(extra, "bstr_prefix=%u", len); }
        }
        break;
    }
    case HK_BSTRFREE:
        if (c->a[0]) reg_del(c->a[0] - 4);
        break;
    case HK_FINDFIRST:
        if (ret != (u64)(size_t)INVALID_HANDLE_VALUE && ret) { FindClose((HANDLE)(size_t)ret); sb_sep(extra, ";"); sb_cat(extra, "find_handle_closed=1"); }
        break;
    default: break;
    }
    (void)t;
}

static int exec_case(char *id, char *api, char *dll, char *argstr, char *capture) {
    Ctx c; const Api *ap; SB outs = { 0, 0, 0 }, dec = { 0, 0, 0 }, extra = { 0, 0, 0 };
    char *toks[MAXARGS + 2]; int nt = 0, i; char *p; char retb[40], leb[24], lech[4];
    HMODULE hm; DWORD mle = 0; FARPROC fp; u64 ret = 0; DWORD le;
    const char *dllname;

    memset(&c, 0, sizeof c);
    scpy(g_cur_id, sizeof g_cur_id, id); scpy(g_cur_api, sizeof g_cur_api, api);

    p = argstr;
    if (p && *p) { for (;;) { toks[nt++] = p; p = strchr(p, '|'); if (!p || nt > MAXARGS) break; *p++ = 0; } }
    if (api[0] == '@') return exec_op(id, api, toks, nt, capture);

    ap = api_find(api);
    if (!ap) { put_row(id, api, "skip:not-whitelisted", "", "", "", "", "", ""); return 0; }
    if ((int)strlen(ap->sig) != nt) { put_row(id, api, "err:argc", "", "", "", "", "", "expected-vs-given"); return 0; }
    c.n = nt;
    for (i = 0; i < nt; i++) {
        int r = parse_tok(&c, i, toks[i], &c.a[i]);
        if (r == -2) { put_row(id, api, "nomodule", "", "", "", "", "", c.note); ctx_free(&c); return 0; }
        if (r < 0) { put_row(id, api, "err:arg", "", "", "", "", "", c.why ? c.why : ""); ctx_free(&c); return 0; }
    }
    if (policy(ap, &c) < 0) { put_row(id, api, "skip:policy", "", "", "", "", "", c.why ? c.why : ""); ctx_free(&c); return 0; }

    dllname = (dll && *dll) ? dll : ap->dll;
    hm = get_module(dllname, &mle);
    if (!hm) { SB e = { 0, 0, 0 }; sb_printf(&e, "dll=%s;le=%lu", dllname, (unsigned long)mle); put_row(id, api, "nomodule", "", "", "", "", "", e.p); free(e.p); ctx_free(&c); return 0; }
    fp = GetProcAddress(hm, api);
    if (!fp) { SB e = { 0, 0, 0 }; sb_printf(&e, "dll=%s", dllname); put_row(id, api, "noexport", "", "", "", "", "", e.p); free(e.p); ctx_free(&c); return 0; }

    g_cb_count = 0; sb_clear(&g_cb);
    if (c.note[0]) sb_cat(&extra, c.note);

    {
        /* in-flight marker so --resume can skip a case that hangs or kills the process */
        FILE *f = fopen(g_inflight, "wb"); if (f) { fputs(id, f); fclose(f); }
    }
    SetLastError(SENTINEL);
    ret = ((Fn12)fp)(c.a[0], c.a[1], c.a[2], c.a[3], c.a[4], c.a[5], c.a[6], c.a[7], c.a[8], c.a[9], c.a[10], c.a[11]);
    le = GetLastError();

    if (ap->ret == 'd') ret &= 0xffffffffull;
    if (ap->ret == 'v') strcpy(retb, "-"); else sprintf(retb, "0x%llx", ret);
    sprintf(leb, "%lu", (unsigned long)le);
    strcpy(lech, le != SENTINEL ? "1" : "0");

    hex_changed(&outs, &c, (ap->flags & F_NOHEX) != 0, &extra);
    decode_out(ap, &c, ret, (int)le, &dec);
    post_hooks(ap, &c, ret, &extra, id);
    if (ap->ret == 'q' || ap->ret == 'd') {
        int k;
        for (k = 0; k < c.nb; k++) if (ret && c.b[k].owned && ret >= (u64)(size_t)c.b[k].user && ret < (u64)(size_t)c.b[k].user + c.b[k].size) { sb_sep(&extra, ";"); sb_printf(&extra, "ret_in=a%d+%llu", c.b[k].arg, ret - (u64)(size_t)c.b[k].user); }
    }
    if (capture && *capture) {
        unsigned fl = 0;
        if (!strncmp(api, "RegOpenKeyEx", 12)) {
            const u8 *hp = arg_mem(&c, 4, 8);
            u64 hv = (ret == 0 && hp) ? rd64(hp, 0) : 0;
            var_set(capture, hv, NULL, 0, hv ? VF_REGKEY : 0);
        } else {
            if (!strcmp(api, "CreateCompatibleDC") && ret) fl = VF_MEMDC | VF_OWNED;
            else if ((ap->hook == HK_GDIOBJ || ap->hook == HK_DIB) && ret && strcmp(api, "GetDC") && strcmp(api, "GetStockObject")) fl = VF_OWNED;
            else if (ap->hook == HK_LOADLIB && ret) fl = VF_OWNED;
            else if ((!strcmp(api, "GetProcessHeap") || !strcmp(api, "HeapCreate")) && ret) fl = VF_HEAP;
            var_set(capture, ret, NULL, 0, fl);
        }
    }
    if ((ap->ret == 'd' && (ret & 0xffffffff)) || ap->ret == 'q') {
        /* a closed or freed handle must not be accepted again */
        if (!strcmp(api, "DeleteDC") || !strcmp(api, "DeleteObject") || !strcmp(api, "DestroyCursor") || !strcmp(api, "DestroyIcon") || !strcmp(api, "FreeLibrary")) var_forget(c.a[0]);
    }
    if (!strcmp(api, "RegCloseKey") && ret == 0) var_forget(c.a[0]);
    put_row(id, api, "ok", retb, leb, lech, outs.p ? outs.p : "", dec.p ? dec.p : "", extra.p ? extra.p : "");
    free(outs.p); free(dec.p); free(extra.p);
    ctx_free(&c);
    return 0;
}

/* ------------------------------------------------------------------ header */
static void write_header(const char *tag, const char *casesfile) {
    OSVERSIONINFOEXW v; typedef LONG (WINAPI *RtlGetVersionFn)(OSVERSIONINFOEXW *); RtlGetVersionFn rg; SYSTEMTIME st; SB h = { 0, 0, 0 };
    HMODULE nt = GetModuleHandleW(L"ntdll.dll");
    memset(&v, 0, sizeof v); v.dwOSVersionInfoSize = sizeof v;
    rg = nt ? (RtlGetVersionFn)GetProcAddress(nt, "RtlGetVersion") : NULL;
    GetSystemTime(&st);
    {
        const char *bn = casesfile, *q;
        for (q = casesfile; *q; q++) if (*q == '\\' || *q == '/') bn = q + 1;
        casesfile = bn;
    }
    fprintf(g_out, "# replay-version\t%s\n# tag\t%s\n# cases-file\t%s\n# utc\t%04d-%02d-%02dT%02d:%02d:%02dZ\n", REPLAY_VERSION, tag, casesfile, st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
#if defined(_MSC_VER)
    fprintf(g_out, "# compiler\tMSVC %d\n", _MSC_VER);
#elif defined(__GNUC__)
    fprintf(g_out, "# compiler\tgcc %d.%d (mingw)\n", __GNUC__, __GNUC_MINOR__);
#endif
    if (rg && rg(&v) == 0) fprintf(g_out, "# RtlGetVersion\t%lu.%lu.%lu platform=%lu sp=%u.%u suite=0x%x product=%u\n", v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber, v.dwPlatformId, v.wServicePackMajor, v.wServicePackMinor, v.wSuiteMask, v.wProductType);
    fprintf(g_out, "# mask-private\t%d\n", g_mask);
    {
        SYSTEM_INFO si; GetNativeSystemInfo(&si);
        fprintf(g_out, "# native-arch\t%u\n# ACP\t%u\n# OEMCP\t%u\n", si.wProcessorArchitecture, GetACP(), GetOEMCP());
    }
    free(h.p);
}

/* ------------------------------------------------------------------ main */
static int lastline_id(const char *path, char *id, size_t cap) {
    FILE *f = fopen(path, "rb"); char line[1024]; int found = 0;
    if (!f) return 0;
    while (fgets(line, sizeof line, f)) { if (line[0] == '#') continue; { char *t = strchr(line, '\t'); if (t) { *t = 0; scpy(id, cap, line); found = 1; } } }
    fclose(f); return found;
}
static void list_whitelist(void) {
    int i, tr = 0, he = 0;
    printf("name\tdll\tsig\tret\tclass\n");
    for (i = 0; i < NAPI; i++) { printf("%s\t%s\t%s\t%c\t%s\n", k_api[i].name, k_api[i].dll, k_api[i].sig, k_api[i].ret, (k_api[i].flags & F_HELPER) ? "helper" : "traced"); if (k_api[i].flags & F_HELPER) he++; else tr++; }
    printf("# total=%d traced=%d helper=%d\n", NAPI, tr, he);
}

int main(int argc, char **argv) {
    const char *cases = "cases.tsv", *out = "observed.tsv", *tag = "default", *only = NULL;
    int resume = 0, i, dpi = 0; long limit = -1;
    FILE *cf; char *line; size_t cap = 1 << 20;
    char skip_after[64] = "";
    long done = 0;
    for (i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--cases") && i + 1 < argc) cases = argv[++i];
        else if (!strcmp(argv[i], "--out") && i + 1 < argc) out = argv[++i];
        else if (!strcmp(argv[i], "--tag") && i + 1 < argc) tag = argv[++i];
        else if (!strcmp(argv[i], "--only") && i + 1 < argc) only = argv[++i];
        else if (!strcmp(argv[i], "--limit") && i + 1 < argc) limit = atol(argv[++i]);
        else if (!strcmp(argv[i], "--resume")) resume = 1;
        else if (!strcmp(argv[i], "--no-mask")) g_mask = 0;
        else if (!strcmp(argv[i], "--dpi-aware")) dpi = 1;
        else if (!strcmp(argv[i], "--list-whitelist")) { list_whitelist(); return 0; }
        else { fprintf(stderr, "usage: replay [--cases cases.tsv] [--out observed.tsv] [--tag name] [--resume] [--no-mask] [--only API,API] [--limit N] [--dpi-aware] [--list-whitelist]\n"); return 2; }
    }
    GetEnvironmentVariableA("USERNAME", g_user, sizeof g_user);
    GetEnvironmentVariableA("COMPUTERNAME", g_comp, sizeof g_comp);
    GetEnvironmentVariableA("USERPROFILE", g_profile, sizeof g_profile);
    SetUnhandledExceptionFilter(crash_filter);
    {
        typedef BOOL (WINAPI *SDDD)(DWORD); SDDD f = (SDDD)GetProcAddress(GetModuleHandleW(L"kernel32.dll"), "SetDefaultDllDirectories");
        if (f) f(0x800 /* LOAD_LIBRARY_SEARCH_SYSTEM32 */);
        if (dpi) { typedef BOOL (WINAPI *SDA)(void); HMODULE u = LoadLibraryExW(L"user32.dll", NULL, 0x800); SDA g = u ? (SDA)GetProcAddress(u, "SetProcessDPIAware") : NULL; if (g) g(); }
    }
    sprintf(g_inflight, "%s.inflight", out);
    scpy(g_outpath, sizeof g_outpath, out);
    cf = fopen(cases, "rb");
    if (!cf) { fprintf(stderr, "cannot open %s\n", cases); return 2; }
    if (resume) {
        char last[64] = "";
        FILE *inf = fopen(g_inflight, "rb");
        if (inf) { char b[64] = ""; if (fgets(b, sizeof b, inf)) scpy(skip_after, sizeof skip_after, b); fclose(inf); }
        if (!lastline_id(out, last, sizeof last)) resume = 0;
        else scpy(g_cur_id, sizeof g_cur_id, last);
        g_out = fopen(out, resume ? "ab" : "wb");
        if (resume && skip_after[0]) {
            /* the in-flight case from the previous run hung or crashed: record it unless the crash filter already did */
            if (strcmp(skip_after, last)) put_row(skip_after, "?", "hang-or-killed", "", "", "", "", "", "skipped on resume");
            scpy(last, sizeof last, skip_after);
        }
        scpy(skip_after, sizeof skip_after, last);
    } else {
        g_out = fopen(out, "wb");
        skip_after[0] = 0;
    }
    if (!g_out) { fprintf(stderr, "cannot write %s\n", out); return 2; }
    if (!resume) { write_header(tag, cases); fprintf(g_out, "# columns\tcase_id\tapi\tstatus\tret\tlast_error\tle_changed\touts\tdecoded\textra\n"); }
    line = (char *)malloc(cap);
    {
        int seen_skip = skip_after[0] ? 0 : 1;
        int first = 1;
        while (fgets(line, (int)cap, cf)) {
            char *f[12]; int nf = 0, z; char *q = line; size_t L = strlen(line);
            static char emptyf[1] = "";
            while (L && (line[L - 1] == '\n' || line[L - 1] == '\r')) line[--L] = 0;
            if (!L || line[0] == '#') continue;
            if (first) { first = 0; if (!strncmp(line, "case_id", 7)) continue; }
            for (;;) { f[nf++] = q; q = strchr(q, '\t'); if (!q || nf >= 12) break; *q++ = 0; }
            if (nf < 3) continue;
            for (z = nf; z < 12; z++) f[z] = emptyf;
            if (!seen_skip) { if (!strcmp(f[0], skip_after)) seen_skip = 1; continue; }
            if (only) {
                char tmp[1024]; char *tok, *sv; int hit = 0;
                scpy(tmp, sizeof tmp, only); tmp[sizeof tmp - 1] = 0; sv = tmp;
                while ((tok = next_tok(&sv, ',')) != NULL) if (!strcmp(tok, f[2])) hit = 1;
                if (!hit && f[2][0] != '@') continue;
            }
            /* columns: case_id kind api dll args capture ... */
            exec_case(f[0], f[2], f[3], f[4], f[5]);
            done++;
            if (done % 200 == 0) fprintf(stderr, "\rreplay: %ld cases (%s)   ", done, f[0]);
            if (limit >= 0 && done >= limit) break;
        }
    }
    fprintf(stderr, "\rreplay: %ld cases done                \n", done);
    fclose(cf); fclose(g_out);
    remove(g_inflight);
    free(line);
    return 0;
}
