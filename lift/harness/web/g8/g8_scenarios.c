/* Scenarios of decision 0144 (docs/decisions/0144-g8a-open-project.md, PROVISIONAL): how a file reaches the engine (runtime/fle_appfile.c), with the REAL runtime files (fle_thread.c, fle_win.c, fle_input.c, fle_env.c ...) and a STUB
 * guest window procedure (a C function in a replacement fle_dispatch_table is a "guest function": the lifted engine is not involved). Native only (tests/wall_fr18.rs builds and runs it and compares each scenario's stdout with the lines
 * written by hand from the Learn pages and the choices of the decision).
 *
 * usage: <program> list | <scenario name>
 * Everything goes to stdout, unbuffered. Nothing printed depends on the host: no address, no time. FLE_CMDLINE comes from the environment of the process (the test sets it per run). */
#define _GNU_SOURCE
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_event.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_input.h"
#include "fle_cursor.h"
#include "fle_monitor.h"
#include "fle_appfile.h"

void fle_imp_GetCommandLineW(void);
void fle_imp_GetCommandLineA(void);
void fle_imp_DefWindowProcW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))

#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
#define CALL5(f, a, b, c, d, e) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)

#define VA_MAIN 0x2000
#define VA_OTHER 0x2010
#define VA_THREAD 0x2020
static uint64_t g_main, g_other;
static int log_msgs = 1, show_thread;

/* print the UTF-16 text at p: ASCII as it is, every other unit as \uXXXX */
static void show16(uint64_t p, unsigned maxu)
{
    unsigned i;

    for (i = 0; i < maxu; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);

        if (!c) break;
        if (c >= 0x20 && c < 0x7f) putchar((int)c);
        else printf("\\u%04X", (unsigned)c);
    }
}

/* the guest's window procedure of the main form: WM_COPYDATA is answered with 1 (what the engine's handler f_11d1c80 stores in TMessage.Result) after the structure is printed; the creation messages as Learn says;
 * everything else is the default processing of the runtime */
static void wndproc_main(void)
{
    uint64_t hw = R[1], msg = R[2], wp = R[8], lp = R[9];

    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83 || msg == 0x01) { R[0] = 0; return; }
    if (msg == 0x4A) {
        uint64_t dw = fle_ld(lp, 8), cb = fle_ld(lp + 8, 4), lpd = fle_ld(lp + 16, 8);

        if (log_msgs) {
            printf("  main msg 4a WM_COPYDATA wp %llx dwData %llu cbData %llu lpData=", U(wp), U(dw), U(cb));
            show16(lpd, 600);
            printf(" last_unit_is_nul %d", (int)(fle_ld(lpd + cb - 2, 2) == 0));
            if (show_thread) printf(" thread %u", (unsigned)fle_thread_current());
            printf("\n");
        }
        (void)hw;
        R[0] = 1;
        return;
    }
    R[0] = 0;
}
static void wndproc_other(void)
{
    R[0] = R[2] == 0x81 ? 1 : 0;
}

static void thread_entry(void);
const fle_fn_entry fle_dispatch_table[] = {
    { VA_MAIN, wndproc_main },
    { VA_OTHER, wndproc_other },
    { VA_THREAD, thread_entry },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static uint16_t cls_main[] = { 'T','F','r','u','i','t','y','L','o','o','p','s','M','a','i','n','F','o','r','m', 0 }, cls_other[] = { 'T','O','t','h','e','r', 0 };
static void regcls(uint16_t *name, uint64_t proc)
{
    static uint8_t wc[72];
    uint64_t inst = 0x400000, nmp = A(name), cur = FLE_CURSOR_HANDLE_BASE + 4u;

    memset(wc, 0, sizeof wc);
    memcpy(wc + 8, &proc, 8);
    memcpy(wc + 24, &inst, 8);
    memcpy(wc + 40, &cur, 8);
    memcpy(wc + 64, &nmp, 8);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
}
static uint64_t mkwin(uint16_t *cls, uint32_t style, int x, int y, int cx, int cy)
{
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(cls); R[8] = A(empty); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
#define WS_POPUP_ 0x80000000u
#define WS_VISIBLE_ 0x10000000u

typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
_Static_assert(sizeof(MSGX) == FLE_MSG_SIZE, "sizeof MSG");
static uint32_t PEEK(MSGX *m) { CALL5(fle_imp_PeekMessageW, A(m), 0, 0, 0, 1); return (uint32_t)R[0]; }
/* one message-retrieval call of the thread (PeekMessage with PM_REMOVE): sent messages are answered in it (Learn SendMessage: "processed only when the receiving thread executes message retrieval code") */
static void retrieve(void) { MSGX m; (void)PEEK(&m); }

static uint32_t GET(MSGX *m) { CALL4(fle_imp_GetMessageW, A(m), 0, 0, 0); return (uint32_t)R[0]; }
/* a guest thread that owns the main form and sits in GetMessage: the window's owner is not the thread that calls fle_appfile_open */
static void thread_entry(void)
{
    MSGX m;

    g_main = mkwin(cls_main, WS_POPUP_ | WS_VISIBLE_, 10, 10, 300, 200);
    printf("thread %u: main form created\n", (unsigned)fle_thread_current());
    while (GET(&m)) {}
    R[0] = 0;
}

static void boot(void)
{
    R[4] = E0;
    fle_st(E0, 8, RET0);
    fle_teb_set_stack(A(stk), A(stk) + sizeof stk);
    regcls(cls_main, VA_MAIN);
    regcls(cls_other, VA_OTHER);
}
static void world(void)
{
    g_main = mkwin(cls_main, WS_POPUP_ | WS_VISIBLE_, 10, 10, 300, 200);
    g_other = mkwin(cls_other, WS_POPUP_ | WS_VISIBLE_, 400, 10, 100, 100);
    retrieve();
}
static void state(int rq)
{
    uint64_t res = 99;
    int st = fle_appfile_state(rq, &res);

    printf("state(%d) = %d result %llu\n", rq, st, U(res == 99 ? 99 : res));
}

/* ===================================== scenarios ===================================== */
static void print_w(const uint16_t *w)
{
    unsigned i;

    for (i = 0; w[i]; i++) {
        if (w[i] >= 0x20 && w[i] < 0x7f) putchar((int)w[i]);
        else printf("\\u%04X", (unsigned)w[i]);
    }
}
static void sc_cmdline_w(void)
{
    fle_imp_GetCommandLineW();
    printf("W: ");
    print_w((const uint16_t *)(uintptr_t)R[0]);
    printf("\n");
    fle_imp_GetCommandLineW();
    printf("second call, same pointer: %d\n", (uint64_t)R[0] == (uint64_t)(uintptr_t)fle_appfile_cmdline_w() || !getenv("FLE_CMDLINE"));
}
static void sc_cmdline_a(void)
{
    fle_imp_GetCommandLineA();
    printf("A: %s\n", (const char *)(uintptr_t)R[0]);
}
static void sc_open_no_window(void)
{
    printf("open with no main window: %d\n", fle_appfile_open("C:\\x.flp"));
    printf("main window handle: %llu\n", U(fle_appfile_main_window()));
}
static void sc_open_other_class(void)
{
    g_other = mkwin(cls_other, WS_POPUP_ | WS_VISIBLE_, 400, 10, 100, 100);
    retrieve();
    printf("open with a window of another class only: %d\n", fle_appfile_open("C:\\x.flp"));
}
static void sc_open_delivery(void)
{
    int a, b;

    world();
    printf("main window found: %d\n", fle_appfile_main_window() == g_main);
    a = fle_appfile_open("C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects\\Basic.flp");
    printf("request %d\n", a);
    state(a);
    printf("(nothing is delivered until the window's thread retrieves messages)\n");
    retrieve();
    state(a);
    b = fle_appfile_open("D:\\second.flp");
    printf("request %d\n", b);
    state(b);
    retrieve();
    state(b);
    state(a);
    state(99);
}
static void sc_open_other_thread(void)
{
    int rq;

    show_thread = 1;
    CALL5(fle_imp_CreateThread, 0, 0, VA_THREAD, 0, 0);
    CALL4(fle_imp_Sleep, 10, 0, 0, 0);
    printf("main window found: %d\n", fle_appfile_main_window() == g_main && g_main != 0);
    rq = fle_appfile_open("D:\\second.flp");
    printf("request %d\n", rq);
    state(rq);
    retrieve();
    printf("(thread %u retrieved messages: the window belongs to another thread, nothing is delivered to it)\n", (unsigned)fle_thread_current());
    state(rq);
    CALL4(fle_imp_Sleep, 10, 0, 0, 0);
    state(rq);
}
static void sc_open_unicode(void)
{
    world();
    printf("request %d\n", fle_appfile_open("C:\\caf\xc3\xa9 \xf0\x9f\x98\x80.flp"));
    retrieve();
}
static void sc_open_errors(void)
{
    char big[700];
    int i, last = 0;

    world();
    printf("empty: %d\n", fle_appfile_open(""));
    printf("NULL: %d\n", fle_appfile_open(NULL));
    printf("not UTF-8 (0xff): %d\n", fle_appfile_open("C:\\\xff.flp"));
    printf("overlong 2-byte form of '/': %d\n", fle_appfile_open("C:\\\xc0\xaf.flp"));
    printf("a surrogate code point (ED A0 80): %d\n", fle_appfile_open("C:\\\xed\xa0\x80.flp"));
    memset(big, 'a', sizeof big);
    big[520] = 0;
    printf("520 units (the limit): %d\n", fle_appfile_open(big));
    retrieve();
    big[521] = 0;
    big[520] = 'a';
    printf("521 units: %d\n", fle_appfile_open(big));
    log_msgs = 0;
    for (i = 0; i < 8; i++) last = fle_appfile_open("C:\\q.flp");
    printf("8 pending, the last request number: %d\n", last);
    printf("a 9th while 8 wait: %d\n", fle_appfile_open("C:\\q.flp"));
    retrieve();
    printf("after the retrieval, a new request: %d\n", fle_appfile_open("C:\\q.flp"));
    retrieve();
}
static void sc_defproc_copydata(void)
{
    world();
    CALL4(fle_imp_DefWindowProcW, g_other, 0x4A, 0, 0);
    printf("DefWindowProcW(WM_COPYDATA) = %llu\n", U(R[0]));
}
static void sc_recentdocs_ok(void)
{
    static const uint16_t w[] = { 'C', ':', '\\', 'a', '.', 'f', 'l', 'p', 0 };
    static const char a[] = "C:\\a.flp";

    CALL4(fle_imp_SHAddToRecentDocs, 3, A(w), 0, 0);
    printf("SHARD_PATHW returned\n");
    CALL4(fle_imp_SHAddToRecentDocs, 2, A(a), 0, 0);
    printf("SHARD_PATHA returned\n");
    CALL4(fle_imp_SHAddToRecentDocs, 3, 0, 0, 0);
    printf("NULL item (clear all) returned\n");
}
static void sc_recentdocs_pidl(void)
{
    static const char a[] = "C:\\a.flp";

    CALL4(fle_imp_SHAddToRecentDocs, 1, A(a), 0, 0);
    printf("NOT REACHED\n");
}
static void sc_dpi(void)
{
    uint32_t x, y;
    int t;

    for (t = 0; t <= 2; t++) {
        x = y = 7;
        CALL4(fle_imp_GetDpiForMonitor, FLE_MONITOR_HANDLE, t, A(&x), A(&y));
        printf("type %d: hr %llx dpi %u %u\n", t, U(R[0]), x, y);
    }
    x = y = 7;
    CALL4(fle_imp_GetDpiForMonitor, FLE_MONITOR_HANDLE, 3, A(&x), A(&y));
    printf("type 3: hr %llx dpi %u %u\n", U(R[0]), x, y);
    CALL4(fle_imp_GetDpiForMonitor, FLE_MONITOR_HANDLE + 4, 0, A(&x), A(&y));
    printf("another monitor handle: hr %llx dpi %u %u\n", U(R[0]), x, y);
    CALL4(fle_imp_GetDpiForMonitor, 0, 0, A(&x), A(&y));
    printf("NULL monitor: hr %llx\n", U(R[0]));
    CALL4(fle_imp_GetDpiForMonitor, FLE_MONITOR_HANDLE, 0, 0, A(&y));
    printf("NULL dpiX: hr %llx\n", U(R[0]));
    CALL4(fle_imp_GetDpiForMonitor, FLE_MONITOR_HANDLE, 0, A(&x), 0);
    printf("NULL dpiY: hr %llx\n", U(R[0]));
}

typedef struct { const char *name; void (*fn)(void); } scenario;
static const scenario S[] = {
    { "cmdline_w", sc_cmdline_w }, { "cmdline_a", sc_cmdline_a }, { "open_no_window", sc_open_no_window }, { "open_other_class", sc_open_other_class }, { "open_delivery", sc_open_delivery },
    { "open_other_thread", sc_open_other_thread }, { "open_unicode", sc_open_unicode }, { "open_errors", sc_open_errors }, { "defproc_copydata", sc_defproc_copydata }, { "recentdocs_ok", sc_recentdocs_ok }, { "recentdocs_pidl", sc_recentdocs_pidl },
    { "dpi", sc_dpi },
};

int main(int argc, char **argv)
{
    size_t i;

    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s list | <scenario>\n", argv[0]); return 2; }
    if (!strcmp(argv[1], "list")) {
        for (i = 0; i < sizeof S / sizeof S[0]; i++) printf("%s\n", S[i].name);
        return 0;
    }
    for (i = 0; i < sizeof S / sizeof S[0]; i++) {
        if (strcmp(argv[1], S[i].name)) continue;
        boot();
        S[i].fn();
        printf("scenario %s done\n", S[i].name);
        return 0;
    }
    fprintf(stderr, "unknown scenario %s\n", argv[1]);
    return 2;
}
