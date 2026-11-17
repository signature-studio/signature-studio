/* Harness-side tracing for the return-address probe. Same role as
 * lift/harness/trace.c, plus the guest return address at each lifted function
 * entry. A lifted call stores its guest return VA at R[4] before it enters the
 * callee, so the entry hook reads it with fle_ld(R[4], 8). Not part of the
 * generated output.
 * This is the tracked copy of the native trace.c of the earlier native runs (lift/out-loc/trace.c = out-pdata/trace.c,
 * which are gitignored), plus the A4 survey hooks (a4.h; marked "A4" below). build_native.sh uses it by default. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "fle_dispatch.h"
#include "fle_runtime.h"
#include "a4.h"
#include "boot.h"
#include "census/census.h"

/* a4.c (decision 0113); weak because a build that links this trace without a4.c (the census tests) has no A4 report to feed. */
void a4_sample(void) __attribute__((weak));

#define SHADOW_MAX 4096
#define SEEN_SIZE 262144
#define LOG_MAX 256

static void *shadow[SHADOW_MAX];
static uint64_t shadow_ret[SHADOW_MAX];
static size_t depth;
static uint64_t calls;
static uint64_t distinct;
static uintptr_t seen[SEEN_SIZE];
static uintptr_t lifted[SEEN_SIZE];
static int lifted_ready;
/* Decision 0168 (opt-in): FLE_COV_FILE=<path> writes, at exit and at every report, the entry count of every lifted function ("F va count") and of every
 * (caller, callee) pair seen on the shadow stack ("E caller_va callee_va count"; the first function is 0 when the stack is empty). Unset: nothing is recorded. */
#define COV_PAIRS 262144
static const char *cov_path;
static int cov_state;   /* 0 = not looked at, 1 = on, 2 = off */
static uintptr_t cov_fn[SEEN_SIZE];
static uint64_t cov_cnt[SEEN_SIZE];
static uintptr_t cov_pc[COV_PAIRS], cov_pe[COV_PAIRS];
static uint64_t cov_pn[COV_PAIRS];
static uint64_t cov_total;
static int cov_va_ready;
static uintptr_t cov_vk[SEEN_SIZE];
static uint64_t cov_vv[SEEN_SIZE];
static uint64_t log_callee[LOG_MAX];
static uint64_t log_ret[LOG_MAX];
static size_t log_n;
static char last_msg[512] = "(none)";
/* Census (decision 0097): the full text of the last runtime message (last_msg above is cut at 511 bytes and is what the stop report
 * prints; it is not changed), the per-unit entry limit, and the hook that takes the limit over. All three are inert unless the census
 * harness (harness/census/census.c) sets them. */
static char full_msg[4096] = "";
static uint64_t call_limit = HARNESS_MAX_CALLS;
void (*trace_limit_hook)(void) = NULL;
int __real_fputs(const char *s, FILE *f);
static volatile sig_atomic_t reporting;
static char altbuf[65536];

int __wrap_fputs(const char *s, FILE *f) {
    snprintf(last_msg, sizeof last_msg, "%s", s);
    snprintf(full_msg, sizeof full_msg, "%s", s);
    return __real_fputs(s, f);
}

int __wrap_fprintf(FILE *f, const char *fmt, ...) {
    va_list ap;
    int r;
    va_start(ap, fmt);
    vsnprintf(last_msg, sizeof last_msg, fmt, ap);
    va_end(ap);
    va_start(ap, fmt);
    vsnprintf(full_msg, sizeof full_msg, fmt, ap);
    va_end(ap);
    va_start(ap, fmt);
    r = vfprintf(f, fmt, ap);
    va_end(ap);
    return r;
}

__attribute__((no_instrument_function))
static void hset_add(uintptr_t *t, uintptr_t a) {
    size_t h = (a >> 4) & (SEEN_SIZE - 1);
    while (t[h] && t[h] != a) h = (h + 1) & (SEEN_SIZE - 1);
    t[h] = a;
}

__attribute__((no_instrument_function))
static int hset_has(const uintptr_t *t, uintptr_t a) {
    size_t h = (a >> 4) & (SEEN_SIZE - 1);
    while (t[h]) {
        if (t[h] == a) return 1;
        h = (h + 1) & (SEEN_SIZE - 1);
    }
    return 0;
}

__attribute__((no_instrument_function))
static int is_lifted(void *fn) {
    if (!lifted_ready) {
        for (uint64_t i = 0; i < fle_dispatch_count; i++) {
            hset_add(lifted, (uintptr_t)fle_dispatch_table[i].fn);
        }
        lifted_ready = 1;
    }
    return hset_has(lifted, (uintptr_t)fn);
}

__attribute__((no_instrument_function)) static void cov_dump(void);
__attribute__((no_instrument_function))
static void cov_count(void *caller, void *fn) {
    size_t h = ((uintptr_t)fn >> 4) & (SEEN_SIZE - 1);
    while (cov_fn[h] && cov_fn[h] != (uintptr_t)fn) h = (h + 1) & (SEEN_SIZE - 1);
    cov_fn[h] = (uintptr_t)fn;
    cov_cnt[h]++;
    size_t k = ((((uintptr_t)caller >> 4) * 0x9E3779B1u) ^ ((uintptr_t)fn >> 4)) & (COV_PAIRS - 1);
    while ((cov_pc[k] != (uintptr_t)caller || cov_pe[k] != (uintptr_t)fn) && (cov_pc[k] || cov_pe[k])) k = (k + 1) & (COV_PAIRS - 1);
    cov_pc[k] = (uintptr_t)caller;
    cov_pe[k] = (uintptr_t)fn;
    cov_pn[k]++;
    if ((cov_total++ & 0x3fffff) == 0x3fffff) cov_dump();   /* every 4M entries, so that a run that ends in _exit or abort leaves a recent file */
}

__attribute__((no_instrument_function))
static void cov_init(void) {
    cov_path = getenv("FLE_COV_FILE");
    cov_state = (cov_path && cov_path[0]) ? 1 : 2;
    if (cov_state == 1) atexit(cov_dump);
}

__attribute__((no_instrument_function))
static uint64_t cov_va_of(uintptr_t f) {
    if (!cov_va_ready) {
        for (uint64_t i = 0; i < fle_dispatch_count; i++) {
            uintptr_t g = (uintptr_t)fle_dispatch_table[i].fn;
            size_t h = (g >> 4) & (SEEN_SIZE - 1);
            while (cov_vk[h] && cov_vk[h] != g) h = (h + 1) & (SEEN_SIZE - 1);
            cov_vk[h] = g;
            cov_vv[h] = fle_dispatch_table[i].va;
        }
        cov_va_ready = 1;
    }
    if (!f) return 0;
    size_t h = (f >> 4) & (SEEN_SIZE - 1);
    while (cov_vk[h] && cov_vk[h] != f) h = (h + 1) & (SEEN_SIZE - 1);
    return cov_vk[h] == f ? cov_vv[h] : 0;
}

__attribute__((no_instrument_function))
static void cov_dump(void) {
    if (cov_state != 1) return;
    FILE *o = fopen(cov_path, "w");
    if (!o) return;
    char line[160];
    for (size_t h = 0; h < SEEN_SIZE; h++) {
        if (!cov_fn[h]) continue;
        int n = snprintf(line, sizeof line, "F %llx %llu\n", (unsigned long long)cov_va_of(cov_fn[h]), (unsigned long long)cov_cnt[h]);
        fwrite(line, 1, (size_t)n, o);
    }
    for (size_t k = 0; k < COV_PAIRS; k++) {
        if (!cov_pc[k] && !cov_pe[k]) continue;
        int n = snprintf(line, sizeof line, "E %llx %llx %llu\n", (unsigned long long)cov_va_of(cov_pc[k]), (unsigned long long)cov_va_of(cov_pe[k]), (unsigned long long)cov_pn[k]);
        fwrite(line, 1, (size_t)n, o);
    }
    {   /* FLE_COV_PEEK=<hex address>[,<hex address>...]: the 32-bit guest values at those addresses ("V addr value"), e.g. the IPP CPU selector variables */
        const char *pk = getenv("FLE_COV_PEEK");
        while (pk && *pk) {
            char *end;
            unsigned long long ad = strtoull(pk, &end, 16);
            if (end == pk) break;
            int n = snprintf(line, sizeof line, "V %llx %llx\n", ad, (unsigned long long)(uint32_t)fle_ld(ad, 4));
            fwrite(line, 1, (size_t)n, o);
            pk = (*end == ',') ? end + 1 : end;
        }
    }
    fclose(o);
}

/* Decision 0184 (prune2, opt-in, used only by a fork-probe build): a probe's child writes its own coverage file "<FLE_COV_FILE>.<probe name>" (its counts include the parent's up to the fork) */
__attribute__((no_instrument_function))
void trace_cov_probe_begin(const char *name) {
    if (cov_state == 0) cov_init();
    if (cov_state != 1) return;
    char *q = (char *)malloc(strlen(cov_path) + strlen(name) + 2);
    if (!q) return;
    sprintf(q, "%s.%s", cov_path, name);
    cov_path = q;
}
__attribute__((no_instrument_function))
void trace_cov_probe_end(void) { cov_dump(); }

__attribute__((no_instrument_function))
void __cyg_profile_func_enter(void *fn, void *call_site) {
    uint64_t ret;
    (void)call_site;
    if (!is_lifted(fn)) return;
    if (a4_sample) a4_sample();   /* decision 0113: InitContext is zeroed by the program's exit path; keep what it held (weak: not every link has a4.c) */
    ret = fle_ld(R[4], 8);
    calls++;
    if (cov_state == 0) cov_init();
    if (cov_state == 1) cov_count(depth && depth <= SHADOW_MAX ? shadow[depth - 1] : 0, fn);
    if (!hset_has(seen, (uintptr_t)fn)) {
        hset_add(seen, (uintptr_t)fn);
        distinct++;
    }
    if (depth < SHADOW_MAX) {
        shadow[depth] = fn;
        shadow_ret[depth] = ret;
    }
    if (log_n < LOG_MAX) {
        log_callee[log_n] = 0;
        log_ret[log_n] = ret;
        for (uint64_t i = 0; i < fle_dispatch_count; i++) {
            if ((void *)fle_dispatch_table[i].fn == fn) {
                log_callee[log_n] = fle_dispatch_table[i].va;
                break;
            }
        }
        log_n++;
    }
    depth++;
    if (calls > call_limit) {
        if (trace_limit_hook) trace_limit_hook();
        trace_report("limit: function entry count");
        _exit(3);
    }
}

__attribute__((no_instrument_function))
void __cyg_profile_func_exit(void *fn, void *call_site) {
    (void)call_site;
    if (!is_lifted(fn)) return;
    if (depth) depth--;
}

/* A4 survey hooks (a4.c). */
__attribute__((no_instrument_function))
int trace_fn_seen(void *fn) { return hset_has(seen, (uintptr_t)fn); }

__attribute__((no_instrument_function))
int trace_fn_on_stack(void *fn) {
    size_t n = depth < SHADOW_MAX ? depth : SHADOW_MAX;
    for (size_t i = 0; i < n; i++) {
        if (shadow[i] == fn) return 1;
    }
    return 0;
}

uint64_t trace_distinct_count(void) { return distinct; }
uint64_t trace_call_count(void) { return calls; }

/* Census accessors (census/census.h). */
const char *trace_full_message(void) { return full_msg; }
void trace_message_reset(void) { full_msg[0] = 0; }
size_t trace_depth(void) { return depth; }
void trace_set_depth(size_t d) { depth = d; }
void trace_set_call_limit(uint64_t n) { call_limit = n; }

static uint64_t fn_to_va(void *fn);

/* k = 0 is the top of the shadow stack. Returns 0 when the frame is not recorded (k >= depth, or deeper than SHADOW_MAX). */
int trace_frame(size_t k, uint64_t *va, uint64_t *ret) {
    size_t n = depth < SHADOW_MAX ? depth : SHADOW_MAX;
    if (k >= n) return 0;
    *va = fn_to_va(shadow[n - 1 - k]);
    *ret = shadow_ret[n - 1 - k];
    return 1;
}

static uint64_t fn_to_va(void *fn) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) {
        if ((void *)fle_dispatch_table[i].fn == fn) return fle_dispatch_table[i].va;
    }
    return 0;
}

static const char *fn_name(uint64_t va, char *buf, size_t n) {
    const char *s = image_export_name(va);
    if (s) return s;
    snprintf(buf, n, "f_%llx", (unsigned long long)va);
    return buf;
}

static const char *abort_kind(void) {
    if (strstr(last_msg, "not a known function entry")) return "fle_dispatch_miss";
    if (strstr(last_msg, "unsupported instruction")) return "fle_unsupported";
    if (strstr(last_msg, "undefined flag")) return "fle_rd (undefined flag read)";
    if (strstr(last_msg, "shift count")) return "fle_bad_count";
    return "(not a runtime message)";
}

void trace_report(const char *reason) {
    size_t n;
    char nb[64];
    if (reporting) _exit(5);
    reporting = 1;
    cov_dump();
    printf("== stop: %s\n", reason);
    printf("runtime message: %s\n", last_msg);
    printf("abort kind: %s\n", abort_kind());
    {
        const char *p = strstr(last_msg, "0x");
        if (p && strstr(last_msg, "not a known function entry")) {
            uint64_t va = strtoull(p, NULL, 16);
            const char *imp = image_import_name(va);
            printf("dispatch target: %#llx%s%s\n", (unsigned long long)va,
                   imp ? " import " : "", imp ? imp : "");
        }
    }
    printf("functions executed (distinct): %llu\n", (unsigned long long)distinct);
    printf("function entries (calls): %llu\n", (unsigned long long)calls);
    printf("call depth at stop: %zu%s\n", depth, depth > SHADOW_MAX ? " (shadow stack overflowed, top frames only)" : "");
    n = depth < SHADOW_MAX ? depth : SHADOW_MAX;
    for (size_t i = 0; i < n && i < 10; i++) {
        void *fn = shadow[n - 1 - i];
        uint64_t va = fn_to_va(fn);
        printf("  #%zu %#llx %s guest-return=%#llx\n", i, (unsigned long long)va,
               fn_name(va, nb, sizeof nb), (unsigned long long)shadow_ret[n - 1 - i]);
    }
    a4_report();
    printf("entry log (first %zu of %llu entries):\n", log_n, (unsigned long long)calls);
    for (size_t i = 0; i < log_n; i++) {
        printf("  entry %zu callee=%#llx %s guest-return=%#llx\n", i,
               (unsigned long long)log_callee[i], fn_name(log_callee[i], nb, sizeof nb),
               (unsigned long long)log_ret[i]);
    }
    fflush(stdout);
}

static void on_signal(int sig, siginfo_t *si, void *uc) {
    char reason[200];
    int code;
    (void)uc;
    switch (sig) {
    case SIGABRT:
        snprintf(reason, sizeof reason, "SIGABRT (runtime abort, see runtime message)");
        code = 134;
        break;
    case SIGSEGV:
        snprintf(reason, sizeof reason, "SIGSEGV fault address %p", si->si_addr);
        code = 139;
        break;
    case SIGBUS:
        snprintf(reason, sizeof reason, "SIGBUS fault address %p", si->si_addr);
        code = 135;
        break;
    case SIGILL:
        snprintf(reason, sizeof reason, "SIGILL");
        code = 132;
        break;
    case SIGFPE:
        snprintf(reason, sizeof reason, "SIGFPE");
        code = 136;
        break;
    case SIGALRM:
        snprintf(reason, sizeof reason, "limit: wall time %us", HARNESS_MAX_SECONDS);
        code = 4;
        break;
    default:
        snprintf(reason, sizeof reason, "signal %d", sig);
        code = 128 + sig;
        break;
    }
    trace_report(reason);
    _exit(code);
}

void trace_init(void) {
    stack_t alt;
    struct sigaction sa;
    const int sigs[] = {SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGALRM};
    alt.ss_sp = altbuf;
    alt.ss_size = sizeof altbuf;
    alt.ss_flags = 0;
    sigaltstack(&alt, NULL);
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_signal;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&sa.sa_mask);
    for (size_t i = 0; i < sizeof sigs / sizeof sigs[0]; i++) {
        sigaction(sigs[i], &sa, NULL);
    }
    alarm(HARNESS_MAX_SECONDS);
}
