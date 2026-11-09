/* Decision 0210 (api2, opt-in test tool): the HDROP / DragQueryFileW / DragQueryPoint / DragFinish cases, run against Wine 10.0 (API2_WINE) and, natively, against ANY implementation of the three functions that is linked
 * in (build.sh drag WORKDIR IMPL.c ...). api2 itself no longer ships these three functions: lift/out-crash1 (decision 0209, fle_crash1_drag.c) provides them; this file is the Wine-checked oracle for that implementation. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
#include <shlobj.h>
#define LASTERR() ((unsigned)GetLastError())
static HGLOBAL g_alloc(unsigned n) { return GlobalAlloc(GMEM_FIXED, n); }
static void *g_lock(HGLOBAL h) { return GlobalLock(h); }
static void g_unlock(HGLOBAL h) { GlobalUnlock(h); }
#else
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_glob.h"
#include "fle_api2.h"
uint64_t R[16];
typedef unsigned int UINT, DWORD, BOOL;
typedef uint16_t WCHAR;
typedef void *HDROP, *HGLOBAL;
typedef struct { int32_t x, y; } POINT;
typedef struct { DWORD pFiles; POINT pt; BOOL fNC; BOOL fWide; } DROPFILES;
void fle_imp_DragQueryFileW(void);
void fle_imp_DragQueryPoint(void);
void fle_imp_DragFinish(void);
#define A(x) ((uint64_t)(uintptr_t)(x))
#define C(f, ...) fle_api2_call(fle_imp_##f, __VA_ARGS__)
static UINT DragQueryFileW(HDROP h, UINT i, WCHAR *o, UINT cch) { return (UINT)C(DragQueryFileW, A(h), i, A(o), cch, 0, 0, 0, 0); }
static BOOL DragQueryPoint(HDROP h, POINT *p) { return (BOOL)C(DragQueryPoint, A(h), A(p), 0, 0, 0, 0, 0, 0); }
static void DragFinish(HDROP h) { (void)C(DragFinish, A(h), 0, 0, 0, 0, 0, 0, 0); }
static HGLOBAL g_alloc(unsigned n) { return (HGLOBAL)C(GlobalAlloc, 0 /* GMEM_FIXED: the handle is the address, which is also what an HDROP made by a host side is */, n, 0, 0, 0, 0, 0, 0); }
static void *g_lock(HGLOBAL h) { return (void *)C(GlobalLock, A(h), 0, 0, 0, 0, 0, 0, 0); }
static void g_unlock(HGLOBAL h) { (void)C(GlobalUnlock, A(h), 0, 0, 0, 0, 0, 0, 0); }
#endif

#ifdef API2_DRAG_REGISTER_HEADER
#include API2_DRAG_REGISTER_HEADER   /* an implementation that only accepts HDROPs made by its host side (crash1: fle_drop_register) */
#endif

static void show_w(const WCHAR *w) { for (; *w; w++) printf("%c", *w < 128 ? (char)*w : '?'); }

static HGLOBAL make_drop(int wide, int nc, int px, int py, const char *const *names, int n)
{
    unsigned total = 20u, i, off;
    HGLOBAL h;
    unsigned char *p;
    DROPFILES df;
    for (i = 0; i < (unsigned)n; i++) total += (unsigned)(strlen(names[i]) + 1) * (wide ? 2u : 1u);
    total += wide ? 2u : 1u;
    h = g_alloc(total);
    p = (unsigned char *)g_lock(h);
    memset(p, 0, total);
    memset(&df, 0, sizeof df);
    df.pFiles = 20; df.pt.x = px; df.pt.y = py; df.fNC = (BOOL)nc; df.fWide = (BOOL)wide;
    memcpy(p, &df, 20);
    off = 20;
    for (i = 0; i < (unsigned)n; i++) {
        unsigned j, L = (unsigned)strlen(names[i]);
        for (j = 0; j <= L; j++) {
            if (wide) { p[off] = (unsigned char)names[i][j]; p[off + 1] = 0; off += 2; }
            else { p[off++] = (unsigned char)names[i][j]; }
        }
    }
    g_unlock(h);
#if defined(API2_DRAG_REGISTER_HEADER) && !defined(API2_WINE)
    (void)fle_drop_register((uint64_t)(uintptr_t)h, 0);
#endif
    return h;
}

static void dq(const char *label, HDROP h, UINT idx, int use_buf, UINT cch)
{
    WCHAR b[40];
    UINT r;
    int i;
    for (i = 0; i < 40; i++) b[i] = 0x7A7A;
    r = DragQueryFileW(h, idx, use_buf ? b : NULL, cch);
    printf("DragQueryFileW %s = %u", label, r);
    if (use_buf && r && cch) { printf(" \""); show_w(b); printf("\" next=%04x", (unsigned)b[r < 39 ? r + 1 : 39]); }
    printf("\n");
}

int main(void)
{
    static const char *const files[] = { "C:\\a\\one.txt", "D:\\two", "E:\\x y\\three.wav" };
    HGLOBAL hw, ha, hnc;
    POINT pt;

    setvbuf(stdout, NULL, _IONBF, 0);
    hw = make_drop(1, 0, 11, 22, files, 3);
    ha = make_drop(0, 0, 33, 44, files, 3);
    hnc = make_drop(1, 1, -5, 7, files, 1);
    dq("wide-count", hw, 0xFFFFFFFFu, 0, 0);
    dq("wide-len0", hw, 0, 0, 0);
    dq("wide-len2", hw, 2, 0, 0);
    dq("wide-copy0", hw, 0, 1, 40);
    dq("wide-copy1", hw, 1, 1, 40);
    dq("wide-copy2", hw, 2, 1, 40);
    dq("wide-trunc5", hw, 0, 1, 5);
    dq("wide-exact", hw, 1, 1, 7);
    dq("wide-cch1", hw, 0, 1, 1);
    dq("wide-cch0-buf", hw, 0, 1, 0);
    dq("wide-oob", hw, 3, 1, 40);
    dq("wide-oob-nobuf", hw, 3, 0, 0);
    dq("ansi-count", ha, 0xFFFFFFFFu, 0, 0);
    dq("ansi-len", ha, 2, 0, 0);
    dq("ansi-copy", ha, 0, 1, 40);
    dq("ansi-trunc", ha, 2, 1, 6);
    printf("DragQueryPoint client = %d", (int)DragQueryPoint(hw, &pt)); printf(" (%d,%d)\n", (int)pt.x, (int)pt.y);
    printf("DragQueryPoint ansi = %d", (int)DragQueryPoint(ha, &pt)); printf(" (%d,%d)\n", (int)pt.x, (int)pt.y);
    printf("DragQueryPoint nonclient = %d", (int)DragQueryPoint(hnc, &pt)); printf(" (%d,%d)\n", (int)pt.x, (int)pt.y);
    DragFinish(hw);
    DragFinish(ha);
    DragFinish(hnc);
    printf("DragFinish done\n");
    return 0;
}
