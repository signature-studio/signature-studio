/* Harness-side tracing: function-entry counting (-finstrument-functions on
 * out-dispatch/after.c only), a shadow call stack for the stop report, the run
 * limits, and the report for abort and fault signals. The message from the
 * runtime's fprintf calls is captured by __wrap_fprintf (link option
 * --wrap=fprintf) and printed with the report. Not part of the generated output.
 *
 * Printing from the signal handler is best effort. It is only reached when the
 * run is already stopping. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "fle_dispatch.h"
#include "boot.h"

#define SHADOW_MAX 4096
#define SEEN_SIZE 16384

static void *shadow[SHADOW_MAX];
static size_t depth;
static uint64_t calls;
static uint64_t distinct;
static uintptr_t seen[SEEN_SIZE];
static uintptr_t lifted[SEEN_SIZE];
static int lifted_ready;
static char last_msg[512] = "(none)";
int __real_fputs(const char *s, FILE *f);

int __wrap_fputs(const char *s, FILE *f) {
    snprintf(last_msg, sizeof last_msg, "%s", s);
    return __real_fputs(s, f);
}
static volatile sig_atomic_t reporting;
static char altbuf[65536];

int __wrap_fprintf(FILE *f, const char *fmt, ...) {
    va_list ap;
    int r;
    va_start(ap, fmt);
    vsnprintf(last_msg, sizeof last_msg, fmt, ap);
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

__attribute__((no_instrument_function))
void __cyg_profile_func_enter(void *fn, void *call_site) {
    (void)call_site;
    if (!is_lifted(fn)) return;
    calls++;
    if (!hset_has(seen, (uintptr_t)fn)) {
        hset_add(seen, (uintptr_t)fn);
        distinct++;
    }
    if (depth < SHADOW_MAX) shadow[depth] = fn;
    depth++;
    if (calls > HARNESS_MAX_CALLS) {
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
        printf("  #%zu %#llx %s\n", i, (unsigned long long)va, fn_name(va, nb, sizeof nb));
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
