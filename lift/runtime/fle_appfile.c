/* Decision 0144 (PROVISIONAL): see fle_appfile.h. */
#include <stdlib.h>
#include "fle_test_env.h"
#include <string.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

#include "fle_appfile.h"
#include "fle_env.h"
#include "fle_input.h"
#include "fle_monitor.h"
#include "fle_screen.h"
#include "fle_runtime.h"
#include "fle_thread.h"

/* UTF-8 -> UTF-16. Returns the number of units written (without the NUL), or -1 for a malformed sequence or when `cap` units do not hold the text and its NUL. A 4-byte sequence becomes a surrogate pair. */
static long utf8_to_utf16(const char *s, uint16_t *out, unsigned cap)
{
    unsigned n = 0;
    const unsigned char *p = (const unsigned char *)s;

    while (*p) {
        uint32_t cp;
        unsigned len, i;

        if (*p < 0x80) { cp = *p; len = 1; }
        else if ((*p & 0xE0) == 0xC0) { cp = *p & 0x1Fu; len = 2; }
        else if ((*p & 0xF0) == 0xE0) { cp = *p & 0x0Fu; len = 3; }
        else if ((*p & 0xF8) == 0xF0) { cp = *p & 0x07u; len = 4; }
        else return -1;
        for (i = 1; i < len; i++) {
            if ((p[i] & 0xC0) != 0x80) return -1;
            cp = (cp << 6) | (p[i] & 0x3Fu);
        }
        if ((len == 2 && cp < 0x80) || (len == 3 && cp < 0x800) || (len == 4 && (cp < 0x10000 || cp > 0x10FFFF)) || (cp >= 0xD800 && cp < 0xE000)) return -1;
        p += len;
        if (cp >= 0x10000) {
            if (n + 2 >= cap) return -1;
            cp -= 0x10000;
            out[n++] = (uint16_t)(0xD800 + (cp >> 10));
            out[n++] = (uint16_t)(0xDC00 + (cp & 0x3FF));
        } else {
            if (n + 1 >= cap) return -1;
            out[n++] = (uint16_t)cp;
        }
    }
    out[n] = 0;
    return (long)n;
}

/* ---- A ---- */
static const char *cmd_text(void)
{
    static int done;
    static const char *text;

    if (!done) {
        const char *e = FLE_TEST_GETENV("FLE_CMDLINE");

        done = 1;
        if (e && e[0]) text = e;
    }
    return text;
}

const uint16_t *fle_appfile_cmdline_w(void)
{
    static uint16_t *w;
    const char *t = cmd_text();
    size_t n;

    if (!t) return NULL;
    if (w) return w;
    n = strlen(t);
    w = (uint16_t *)malloc((n + 1) * sizeof *w);
    if (!w) fle_stop("GetCommandLineW", "out of memory (FLE_CMDLINE)");
    if (utf8_to_utf16(t, w, (unsigned)(n + 1)) < 0) fle_stop("GetCommandLineW", "FLE_CMDLINE is not valid UTF-8");
    return w;
}

const char *fle_appfile_cmdline_a(void)
{
    const char *t = cmd_text();
    const unsigned char *p;

    if (!t) return NULL;
    for (p = (const unsigned char *)t; *p; p++)
        if (*p >= 0x80) fle_stop("GetCommandLineA", "FLE_CMDLINE has a character above 0x7F: the conversion to the 8-bit process code page is not modelled for it (decision 0144)");
    return t;
}

/* ---- B ---- */
typedef struct {
    fle_sent sm;       /* the sent message: stays in the receiving thread's queue until processed */
    uint8_t *mem;      /* COPYDATASTRUCT (24 bytes) + the UTF-16 text; freed when the request is processed and a later call finds it */
    int used;
} req_t;

static req_t reqs[FLE_APPFILE_MAX_PENDING];
static int serial;
static int serials[FLE_APPFILE_MAX_PENDING];

uint64_t fle_appfile_main_window(void)
{
    uint64_t hs[64];
    unsigned n = fle_win_input_toplevels(hs, 64), i;

    for (i = 0; i < n; i++) {
        fle_win_iview v;

        if (fle_win_input_view(hs[i], &v) && v.live && !strcmp(v.cls, FLE_APPFILE_MAIN_CLASS)) return hs[i];
    }
    return 0;
}

static void reap(void)
{
    unsigned i;

    for (i = 0; i < FLE_APPFILE_MAX_PENDING; i++)
        if (reqs[i].used && reqs[i].sm.done) {
            free(reqs[i].mem);
            reqs[i].mem = NULL;
            reqs[i].used = 2;   /* processed; the result stays readable */
        }
}

int fle_appfile_open(const char *path_utf8)
{
    uint64_t h = fle_appfile_main_window();
    fle_win_iview v;
    unsigned i, slot = FLE_APPFILE_MAX_PENDING;
    uint16_t *w;
    long units;
    req_t *r;

    if (!h || !fle_win_input_view(h, &v)) return -1;
    if (!path_utf8 || !path_utf8[0]) return -2;
    reap();
    for (i = 0; i < FLE_APPFILE_MAX_PENDING; i++)
        if (!reqs[i].used) { slot = i; break; }
    if (slot == FLE_APPFILE_MAX_PENDING)
        for (i = 0; i < FLE_APPFILE_MAX_PENDING; i++)
            if (reqs[i].used == 2) { slot = i; break; }
    if (slot == FLE_APPFILE_MAX_PENDING) return -3;
    r = &reqs[slot];
    memset(r, 0, sizeof *r);
    r->mem = (uint8_t *)calloc(1, 24 + 2 * (FLE_APPFILE_PATH_MAX + 1u));
    if (!r->mem) return -4;
    w = (uint16_t *)(r->mem + 24);
    units = utf8_to_utf16(path_utf8, w, FLE_APPFILE_PATH_MAX + 1u);
    if (units <= 0) {
        free(r->mem);
        r->mem = NULL;
        return -2;
    }
    /* COPYDATASTRUCT (Learn): ULONG_PTR dwData; DWORD cbData; PVOID lpData (x64: 8, 4 + 4 padding, 8) */
    {
        uint64_t dw = FLE_APPFILE_DWDATA_PATH, cb = ((uint64_t)units + 1u) * 2u, lp = (uint64_t)(uintptr_t)w;

        memcpy(r->mem, &dw, 8);
        memcpy(r->mem + 8, &cb, 8);
        memcpy(r->mem + 16, &lp, 8);
    }
    r->sm.hwnd = h;
    r->sm.msg = FLE_WM_COPYDATA;
    r->sm.wp = 0;
    r->sm.lp = (uint64_t)(uintptr_t)r->mem;
    r->used = 1;
    serials[slot] = ++serial;
    fle_thread_send(v.owner, &r->sm);
    return serials[slot];
}

int fle_appfile_state(int request, uint64_t *result)
{
    unsigned i;

    reap();
    for (i = 0; i < FLE_APPFILE_MAX_PENDING; i++)
        if (reqs[i].used && serials[i] == request) {
            if (result) *result = reqs[i].sm.result;
            return reqs[i].used == 2 || reqs[i].sm.done ? 2 : 1;
        }
    return 0;
}

/* the export of the browser build (the Worker calls it while the engine is suspended at its idle wait, as it does fle_g3_key): a UTF-8 path of the virtual volume; the return value is fle_appfile_open's */
EMSCRIPTEN_KEEPALIVE int fle_app_open(const char *path_utf8) { return fle_appfile_open(path_utf8); }
EMSCRIPTEN_KEEPALIVE int fle_app_open_state(int request) { return fle_appfile_state(request, NULL); }

/* ---- the APIs that opening a project reaches (decision 0144, in the order the walls came) ---- */

/* shell32!SHAddToRecentDocs (Learn): void f(UINT uFlags ecx, LPCVOID pv rdx). "Notifies the system that an item has been accessed"; "Set this parameter to NULL to clear all usage data"; "Return value: None".
 * The forms of pv are the SHARD values (shlobj_core.h: SHARD_PIDL 1, SHARD_PATHA 2, SHARD_PATHW 3; the AppUserModelID forms 4-7 and the IShellLink form 8 are Windows 7 additions). The virtual machine has no
 * Recent folder, jump list or Start menu to update, so the call has no effect the guest can see (S-g8a-recentdocs); the arguments are checked: a path form must point at a NUL-terminated string that can be read, a form
 * that is not modelled stops the run by name. */
void fle_imp_SHAddToRecentDocs(void)
{
    static const char api[] = "SHAddToRecentDocs";
    uint32_t flags = (uint32_t)R[1];
    uint64_t pv = R[2];

    if (pv == 0) return;   /* "Set this parameter to NULL to clear all usage data": nothing is kept */
    switch (flags) {
    case 2: {   /* SHARD_PATHA */
        uint64_t p = pv;
        while (fle_ld(p, 1)) p++;
        return;
    }
    case 3: {   /* SHARD_PATHW */
        uint64_t p = pv;
        while (fle_ld(p, 2)) p += 2;
        return;
    }
    default:
        fle_stop(api, "this SHARD form of the item is not modelled (decision 0144: only SHARD_PATHA 2, SHARD_PATHW 3 and a NULL item)");
    }
}

/* shcore!GetDpiForMonitor (Learn, shellscalingapi.h): HRESULT f(HMONITOR hmonitor rcx, MONITOR_DPI_TYPE dpiType edx, UINT *dpiX r8, UINT *dpiY r9). S_OK with the DPI of the monitor; E_INVALIDARG "The handle, DPI type, or pointers
 * passed in are not valid". The page gives the value by the awareness of the calling process: "PROCESS_DPI_UNAWARE: 96 because the app is unaware of any other scale factors"; the process of this environment is DPI
 * unaware (fle_proc.c GetProcessDpiAwareness) and its one virtual monitor is 96 DPI (decision 0053), so 96 for dpiX and dpiY ("The values of *dpiX and *dpiY are identical"). The three MONITOR_DPI_TYPE values
 * (MDT_EFFECTIVE_DPI 0, MDT_ANGULAR_DPI 1, MDT_RAW_DPI 2: shellscalingapi.h) all answer 96: the page says only that ANGULAR and RAW ignore the desktop scaling slider (S-g8a-dpi, a choice without a source for the
 * unaware case). The engine delay-loads this function and, when GetProcAddress gave NULL, the delay-load helper raised 0xC06D007F (wall 3 of decision 0144). */
void fle_imp_GetDpiForMonitor(void)
{
    uint64_t h = R[1], px = R[8], py = R[9];
    uint32_t type = (uint32_t)R[2];

    if (h != FLE_MONITOR_HANDLE || type > 2u || px == 0 || py == 0) {
        R[0] = 0x80070057u;   /* E_INVALIDARG */
        return;
    }
    fle_st(px, 4, (uint32_t)FLE_SCREEN_PX_DPI);   /* decision 0212: 96 unless FLE_SCREEN_DPI says another (fle_screen.h) */
    fle_st(py, 4, (uint32_t)FLE_SCREEN_PX_DPI);
    R[0] = 0;
}
