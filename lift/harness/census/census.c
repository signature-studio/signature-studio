/* Census of the unit table (see census.h and docs/decisions/0097-census-diagnostic.md). Harness only: the generated C and the runtime
 * are not changed; the one runtime change is the NULL-by-default hook fle_dispatch_hook in fle_dispatch.h.
 *
 * How a unit is abandoned. f_4120c0 (InitUnits) calls the proc of unit i with `fle_dispatch(t)` at 0x412126 after it has written
 * InitCount = i + 1; the return address it stores for that call is 0x412129. fle_dispatch_hook sees that call (return address slot
 * 0x412129 at [R[4]], InitCount - 1 is the index, the table entry equals the target), saves the guest registers and the x87/MMX/MXCSR
 * state, arms a timer and does sigsetjmp. Everything that ends a unit abnormally lands in one signal handler (abort() raises SIGABRT,
 * so fle_stop, fle_dispatch_miss, fle_unsupported, fle_jumptab_miss, fle_rd, ... need no change) or in the function-entry limit hook
 * of trace_native.c; the handler records what it knows (message, signal, fault address, the top of the shadow call stack) and
 * siglongjmps back. The hook then puts the saved state back, sets rax = 0, and returns to f_4120c0 as if the unit had returned.
 *
 * What is NOT undone: memory the unit wrote, the runtime's tables (handles, registry, locks, a real module's load state, the
 * callback nesting counter). Whatever comes after an abandoned unit runs on that state. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <setjmp.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>
#include <ucontext.h>
#include <unistd.h>

#include "a4.h"
#include "boot.h"
#include "census.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_mxcsr.h"
#include "fle_runtime.h"
#include "fle_x87.h"
#include "fle_x87ctl.h"

#define CTX_VA UINT64_C(0x15EAC90)
#define COUNT_VA UINT64_C(0x15EAC98)
#define UNIT_RET UINT64_C(0x412129) /* the return address f_4120c0 pushes for `call proc` at 0x412126 */
#define MAX_UNITS (1u << 16)
#define MAX_IMAGES 9 /* FLEngine + RM_MAX (8, fle_realmod.c) real modules; census_add_image ignores an image beyond this (decision 0106: was 8) */
#define MSG_MAX 2048
#define ULL(x) ((unsigned long long)(x))

extern uint64_t MM[8];
extern uint8_t fle_df;

static int on;
static int fd = -1;
static double unit_seconds = 20.0;
static uint64_t unit_calls = HARNESS_MAX_CALLS;
static uint64_t img_lo[MAX_IMAGES], img_sz[MAX_IMAGES];
static int nimg;
static volatile sig_atomic_t in_unit;
static sigjmp_buf jb;
static struct sigaction old_act[32];
static int finished;
static uint64_t total_ok, total_stop, total_fault, total_hang;
static uint8_t *seen_unit;
static uint64_t tab_hdr, tab_units, tab_n;

/* what the handler fills in for the unit that is being abandoned */
static struct {
    uint64_t idx, proc, fault_addr;
    int sig;
    const char *state;
    int stack_overflow;
    char msg[MSG_MAX];
    uint64_t fr_va[3], fr_ret0;
    int nfr;
    size_t depth;
} cur;

static int in_images(uint64_t va, uint64_t n) {
    for (int i = 0; i < nimg; i++) {
        if (va >= img_lo[i] && va - img_lo[i] <= img_sz[i] && n <= img_sz[i] - (va - img_lo[i])) return 1;
    }
    return 0;
}

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
}

static void out(const char *s, size_t n) {
    while (n) {
        ssize_t w = write(fd, s, n);
        if (w <= 0) return;
        s += w;
        n -= (size_t)w;
    }
}

static void outs(const char *s) { out(s, strlen(s)); }

/* tabs and newlines would break the TSV */
static void clean(char *d, const char *s, size_t cap) {
    size_t i = 0;
    for (; *s && i + 1 < cap; s++) d[i++] = (*s == '\t' || *s == '\n' || *s == '\r') ? ' ' : *s;
    while (i > 0 && d[i - 1] == ' ') i--;
    d[i] = 0;
}

static void arm(double seconds) {
    struct itimerval it;
    memset(&it, 0, sizeof it);
    it.it_value.tv_sec = (time_t)seconds;
    it.it_value.tv_usec = (suseconds_t)((seconds - (double)(time_t)seconds) * 1e6);
    if (seconds > 0 && it.it_value.tv_sec == 0 && it.it_value.tv_usec == 0) it.it_value.tv_usec = 1;
    setitimer(ITIMER_REAL, &it, NULL);
}

static const char *sig_name(int sig) {
    switch (sig) {
    case SIGABRT: return "SIGABRT";
    case SIGSEGV: return "SIGSEGV";
    case SIGBUS: return "SIGBUS";
    case SIGILL: return "SIGILL";
    case SIGFPE: return "SIGFPE";
    case SIGALRM: return "SIGALRM";
    case -1: return "CALLS";
    default: return "SIG?";
    }
}

/* ---- the unit table (InitContext) ---- */
static int read_table(void) {
    uint64_t hdr, units, n;
    if (tab_hdr) return 1;
    hdr = fle_ld(CTX_VA, 8);
    if (hdr == 0 || !in_images(hdr, 16)) return 0;
    n = fle_ld(hdr, 4);
    units = fle_ld(hdr + 8, 8);
    if (n > MAX_UNITS || !in_images(units, 16 * n)) return 0;
    tab_hdr = hdr;
    tab_units = units;
    tab_n = n;
    seen_unit = calloc(n ? n : 1, 1);
    return seen_unit != NULL;
}

/* "runnable", "empty", "noproc", "unreadable": the test of f_4120c0 (decision 0060) */
static const char *unit_kind(uint64_t i) {
    uint64_t proc = fle_ld(tab_units + 16 * i, 8);
    if (proc == 0) return "noproc";
    if (!in_images(proc, 8)) return "unreadable";
    return fle_ld(proc, 8) != 0 ? "runnable" : "empty";
}

/* ---- the import behind a message (decoded from the guest bytes, outside any signal handler) ---- */
static const char *thunk_import(uint64_t va) {
    uint8_t b[8];
    if (!in_images(va, 8)) return NULL;
    for (int i = 0; i < 8; i++) b[i] = (uint8_t)fle_ld(va + (uint64_t)i, 1);
    for (int k = 0; k <= 2; k++) {
        if (b[k] == 0xff && (b[k + 1] == 0x25 || b[k + 1] == 0x15)) { /* jmp/call qword ptr [rip+rel32] */
            int32_t rel;
            memcpy(&rel, b + k + 2, 4);
            return image_import_name(va + (uint64_t)k + 6 + (uint64_t)(int64_t)rel);
        }
    }
    return NULL;
}

static const char *import_of_message(const char *m) {
    const char *p;
    if ((p = strstr(m, "unsupported instruction at 0x")) != NULL) return thunk_import(strtoull(p + strlen("unsupported instruction at "), NULL, 16));
    if ((p = strstr(m, "indirect call to 0x")) != NULL) return image_import_name(strtoull(p + strlen("indirect call to "), NULL, 16));
    return NULL;
}

/* ---- the TSV ---- */
static void write_header(void) {
    outs("# census (decision 0097): NOT A4 progress. An abandoned unit (stop, fault, hang) did not complete.\n");
    outs("unit\tproc\tstate\tsignal\tfault_addr\tstack_overflow\timport\ttop\tcaller1\tcaller2\ttop_guest_return\tentries\tnew_fns\tms\tcb_depth_leak\tmessage\n");
}

static void row(uint64_t idx, uint64_t proc, const char *state, const char *sig, uint64_t fault, int sovf, const char *imp,
                const uint64_t *fr, int nfr, uint64_t ret0, uint64_t entries, uint64_t newfns, double ms, unsigned leak,
                const char *msg) {
    char line[MSG_MAX + 1024], m[MSG_MAX], fa[24], f0[24], f1[24], f2[24], r0[24];
    if (fault) snprintf(fa, sizeof fa, "0x%llx", ULL(fault)); else snprintf(fa, sizeof fa, "-");
    if (nfr > 0) snprintf(f0, sizeof f0, "0x%llx", ULL(fr[0])); else snprintf(f0, sizeof f0, "-");
    if (nfr > 1) snprintf(f1, sizeof f1, "0x%llx", ULL(fr[1])); else snprintf(f1, sizeof f1, "-");
    if (nfr > 2) snprintf(f2, sizeof f2, "0x%llx", ULL(fr[2])); else snprintf(f2, sizeof f2, "-");
    if (nfr > 0) snprintf(r0, sizeof r0, "0x%llx", ULL(ret0)); else snprintf(r0, sizeof r0, "-");
    clean(m, msg, sizeof m);
    snprintf(line, sizeof line, "%llu\t0x%llx\t%s\t%s\t%s\t%d\t%s\t%s\t%s\t%s\t%s\t%llu\t%llu\t%.1f\t%u\t%s\n", ULL(idx), ULL(proc), state,
             sig ? sig : "-", fa, sovf, imp ? imp : "-", f0, f1, f2, r0, ULL(entries), ULL(newfns), ms, leak, m[0] ? m : "-");
    outs(line);
}

/* ---- the abnormal end of a unit: record, then jump back to the hook ---- */
static void note_stop(int sig, const char *state, uint64_t fault, int sovf) {
    uint64_t ret = 0, va;
    cur.sig = sig;
    cur.state = state;
    cur.fault_addr = fault;
    cur.stack_overflow = sovf;
    cur.depth = trace_depth();
    cur.nfr = 0;
    cur.fr_ret0 = 0;
    for (size_t k = 0; k < 3; k++) {
        if (!trace_frame(k, &va, &ret)) break;
        cur.fr_va[k] = va;
        if (k == 0) cur.fr_ret0 = ret;
        cur.nfr++;
    }
    snprintf(cur.msg, sizeof cur.msg, "%s", trace_full_message());
}

static void chain_or_ignore(int sig, siginfo_t *si, void *uc) {
    struct sigaction *o = &old_act[sig];
    if (sig == SIGALRM) return; /* a timer that fires just after the unit returned */
    if (!finished) census_end("stop outside a unit");
    if (o->sa_flags & SA_SIGINFO) {
        if (o->sa_sigaction) o->sa_sigaction(sig, si, uc);
    } else if (o->sa_handler != SIG_DFL && o->sa_handler != SIG_IGN && o->sa_handler) {
        o->sa_handler(sig);
    }
    _exit(128 + sig);
}

static void on_signal(int sig, siginfo_t *si, void *ucv) {
    uint64_t fault = 0;
    int sovf = 0;
    if (!in_unit) {
        chain_or_ignore(sig, si, ucv);
        return;
    }
    in_unit = 0;
    arm(0);
    if (sig == SIGSEGV || sig == SIGBUS) {
        ucontext_t *uc = ucv;
        uint64_t rsp = (uint64_t)uc->uc_mcontext.gregs[REG_RSP];
        fault = (uint64_t)(uintptr_t)si->si_addr;
        /* a fault right next to the host stack pointer is the host stack running out (deep recursion), not a guest pointer */
        if (fault + 65536 >= rsp && fault <= rsp + 65536) sovf = 1;
    }
    switch (sig) {
    case SIGABRT: note_stop(sig, "stop", 0, 0); break;
    case SIGALRM: note_stop(sig, "hang", 0, 0); break;
    default: note_stop(sig, "fault", fault, sovf); break;
    }
    if (sig == SIGALRM) snprintf(cur.msg, sizeof cur.msg, "wall time limit of the unit (%.1f s)", unit_seconds);
    else if (sig != SIGABRT)
        snprintf(cur.msg, sizeof cur.msg, "%s fault address %#llx%s", sig_name(sig), ULL(fault), sovf ? " (host stack)" : "");
    siglongjmp(jb, sig);
}

static void on_limit(void) {
    if (!in_unit) return;
    in_unit = 0;
    arm(0);
    note_stop(-1, "hang", 0, 0);
    snprintf(cur.msg, sizeof cur.msg, "function-entry limit of the unit (%llu)", ULL(unit_calls));
    siglongjmp(jb, -1);
}

/* ---- the state that a unit may leave behind and that the harness can put back ---- */
typedef struct {
    uint64_t r[16];
    fle_x87_state x87;
    fle_f80 st[8];
    uint64_t mm[8];
    uint8_t mmx_active, df;
    uint32_t mxcsr;
} cpu_state;

static void cpu_save(cpu_state *s) {
    memcpy(s->r, R, sizeof s->r);
    s->x87 = fle_x87;
    memcpy(s->st, fle_x87_r, sizeof s->st);
    memcpy(s->mm, MM, sizeof s->mm);
    s->mmx_active = fle_x87_mmx_active;
    s->df = fle_df;
    s->mxcsr = fle_mxcsr;
}

static void cpu_restore(const cpu_state *s) {
    memcpy(R, s->r, sizeof s->r);
    fle_x87 = s->x87;
    memcpy(fle_x87_r, s->st, sizeof s->st);
    memcpy(MM, s->mm, sizeof s->mm);
    fle_x87_mmx_active = s->mmx_active;
    fle_df = s->df;
    fle_mxcsr = s->mxcsr;
}

static cpu_state saved;

static void run_unit(uint64_t idx, uint64_t va, void (*fn)(void)) {
    uint64_t calls0 = trace_call_count(), dist0 = trace_distinct_count();
    unsigned cb0 = fle_callback_depth();
    size_t depth0 = trace_depth();
    double t0 = now_ms(), ms;
    int sig;

    cpu_save(&saved);
    memset(&cur, 0, sizeof cur);
    trace_message_reset();
    trace_set_call_limit(calls0 + unit_calls);
    seen_unit[idx] = 1;
    arm(unit_seconds);
    sig = sigsetjmp(jb, 1);
    if (sig == 0) {
        in_unit = 1;
        fn();
        in_unit = 0;
        arm(0);
        ms = now_ms() - t0;
        total_ok++;
        row(idx, va, "ok", NULL, 0, 0, NULL, NULL, 0, 0, trace_call_count() - calls0, trace_distinct_count() - dist0, ms,
            fle_callback_depth() - cb0, "");
        return;
    }
    /* abandoned: the handler (or on_limit) has filled `cur` */
    in_unit = 0;
    arm(0);
    ms = now_ms() - t0;
    trace_set_depth(depth0);
    cpu_restore(&saved);
    R[0] = 0;
    if (strcmp(cur.state, "stop") == 0) total_stop++;
    else if (strcmp(cur.state, "fault") == 0) total_fault++;
    else total_hang++;
    cur.idx = idx;
    cur.proc = va;
    row(idx, va, cur.state, sig_name(cur.sig), cur.fault_addr, cur.stack_overflow, import_of_message(cur.msg), cur.fr_va, cur.nfr,
        cur.fr_ret0, trace_call_count() - calls0, trace_distinct_count() - dist0, ms, fle_callback_depth() - cb0, cur.msg);
}

static void hook(uint64_t va, void (*fn)(void)) {
    uint64_t cnt, idx;
    if (in_unit || fle_ld(R[4], 8) != UNIT_RET || !read_table()) {
        fn();
        return;
    }
    cnt = fle_ld(COUNT_VA, 4);
    if (cnt == 0 || cnt > tab_n) {
        fn();
        return;
    }
    idx = cnt - 1;
    if (fle_ld(tab_units + 16 * idx, 8) != va) {
        fn();
        return;
    }
    run_unit(idx, va, fn);
}

/* ---- begin / end ---- */
int census_begin(uint64_t img_base, uint64_t img_size) {
    const char *path = getenv("FLE_CENSUS"), *e;
    struct sigaction sa;
    const int sigs[] = {SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGALRM};

    if (!path || !*path) return 0;
    if ((e = getenv("FLE_CENSUS_UNIT_SECONDS")) != NULL && atof(e) > 0) unit_seconds = atof(e);
    if ((e = getenv("FLE_CENSUS_UNIT_CALLS")) != NULL && strtoull(e, NULL, 0) > 0) unit_calls = strtoull(e, NULL, 0);
    fd = open(path, O_WRONLY | O_CREAT | O_EXCL, 0644);
    if (fd < 0) {
        fprintf(stderr, "census: cannot create %s (an existing file is never overwritten)\n", path);
        exit(2);
    }
    on = 1;
    census_add_image(img_base, img_size);
    write_header();
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_signal;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigfillset(&sa.sa_mask);
    for (size_t i = 0; i < sizeof sigs / sizeof sigs[0]; i++) sigaction(sigs[i], &sa, &old_act[sigs[i]]);
    alarm(0); /* the harness wall-clock limit of trace_init: the census has its own, per unit */
    trace_limit_hook = on_limit;
    fle_dispatch_hook = hook;
    printf("census: ON, per-unit limits %.1f s and %llu function entries; output %s\n"
           "census: the result is NOT A4 progress (decision 0097): abandoned units did not complete\n",
           unit_seconds, ULL(unit_calls), path);
    fflush(stdout);
    return 1;
}

void census_add_image(uint64_t base, uint64_t size) {
    if (nimg < MAX_IMAGES) {
        img_lo[nimg] = base;
        img_sz[nimg] = size;
        nimg++;
    }
}

void census_end(const char *how) {
    uint64_t n_empty = 0, n_noproc = 0, n_unread = 0, n_notreached = 0;
    char buf[MSG_MAX];
    if (!on || finished) return;
    finished = 1;
    in_unit = 0;
    arm(0);
    fle_dispatch_hook = NULL;
    if (read_table()) {
        for (uint64_t i = 0; i < tab_n; i++) {
            const char *k = unit_kind(i);
            uint64_t proc = fle_ld(tab_units + 16 * i, 8);
            if (strcmp(k, "runnable") == 0) {
                if (!seen_unit[i]) {
                    n_notreached++;
                    row(i, proc, "not_reached", NULL, 0, 0, NULL, NULL, 0, 0, 0, 0, 0, 0, "");
                }
            } else {
                if (strcmp(k, "empty") == 0) n_empty++;
                else if (strcmp(k, "noproc") == 0) n_noproc++;
                else n_unread++;
                row(i, proc, k, NULL, 0, 0, NULL, NULL, 0, 0, 0, 0, 0, 0, "");
            }
        }
    }
    clean(buf, strcmp(how, "stop outside a unit") == 0 ? trace_full_message() : "", sizeof buf);
    {
        char line[MSG_MAX + 256], f[3][24];
        const char *imp = buf[0] ? import_of_message(buf) : NULL;
        uint64_t va, ret;
        for (int i = 0; i < 3; i++) {
            if (buf[0] && trace_frame((size_t)i, &va, &ret)) snprintf(f[i], sizeof f[i], "0x%llx", ULL(va));
            else snprintf(f[i], sizeof f[i], "-");
        }
        snprintf(line, sizeof line, "#end\t%s\t%s\t%s\t%s\t%s\t%s\n", how, buf[0] ? buf : "-", imp ? imp : "-", f[0], f[1], f[2]);
        outs(line);
        snprintf(line, sizeof line, "#counts\tunits=%llu ok=%llu stop=%llu fault=%llu hang=%llu empty=%llu noproc=%llu unreadable=%llu not_reached=%llu\n",
                 ULL(tab_n), ULL(total_ok), ULL(total_stop), ULL(total_fault), ULL(total_hang), ULL(n_empty), ULL(n_noproc), ULL(n_unread),
                 ULL(n_notreached));
        outs(line);
    }
    printf("census: end (%s): units=%llu ok=%llu stop=%llu fault=%llu hang=%llu empty=%llu noproc=%llu not_reached=%llu\n", how, ULL(tab_n),
           ULL(total_ok), ULL(total_stop), ULL(total_fault), ULL(total_hang), ULL(n_empty), ULL(n_noproc), ULL(n_notreached));
    printf("census: the a4 lines printed after this one describe a run in which units were ABANDONED; they are not A4 progress\n");
    fflush(stdout);
}
