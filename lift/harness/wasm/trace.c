/* wasm harness trace (decision 0045). Derived from lift/out-pdata/trace.c (the native report), with three changes:
 * no signal handlers (wasm has none; abort() is redirected here by -Wl,--wrap=abort), the hset hash is a
 * multiplicative hash ((a >> 4) clusters on wasm function-table indices: 7.5 s -> 1.5 s), and abort_kind knows the
 * address stops of fle_runtime.h. lift/harness/trace.c is not changed. */
/* Harness-side tracing for the return-address probe. Same role as
 * lift/harness/trace.c, plus the guest return address at each lifted function
 * entry. A lifted call stores its guest return VA at R[4] before it enters the
 * callee, so the entry hook reads it with fle_ld(R[4], 8). Not part of the
 * generated output. */
#define _GNU_SOURCE
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "fle_dispatch.h"
#include "fle_runtime.h"
#include "a4.h"
#include "boot.h"

/* a4.c (decision 0113), as in trace_native.c; weak because a link without a4.c has no A4 report to feed. */
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
static uint64_t log_callee[LOG_MAX];
static uint64_t log_ret[LOG_MAX];
static size_t log_n;
static char last_msg[512] = "(none)";
int __real_fputs(const char *s, FILE *f);
static volatile int reporting;

int __wrap_fputs(const char *s, FILE *f) {
    snprintf(last_msg, sizeof last_msg, "%s", s);
    return __real_fputs(s, f);
}

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
    size_t h = (size_t)(((uint64_t)a * UINT64_C(0x9E3779B97F4A7C15)) >> 40) & (SEEN_SIZE - 1);
    while (t[h] && t[h] != a) h = (h + 1) & (SEEN_SIZE - 1);
    t[h] = a;
}

__attribute__((no_instrument_function))
static int hset_has(const uintptr_t *t, uintptr_t a) {
    size_t h = (size_t)(((uint64_t)a * UINT64_C(0x9E3779B97F4A7C15)) >> 40) & (SEEN_SIZE - 1);
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
    uint64_t ret;
    (void)call_site;
    if (!is_lifted(fn)) return;
    if (a4_sample) a4_sample();   /* decision 0113: InitContext is zeroed by the program's exit path; keep what it held (weak: not every link has a4.c) */
    ret = fle_ld(R[4], 8);
    calls++;
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
    if (strstr(last_msg, "[fle_addr_hi32]")) return "fle_addr_hi32 (guest address above 32 bits, wasm)";
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

/* wasm: no signals. abort() (the runtime's stop path) is redirected here by -Wl,--wrap=abort. */
void __wrap_abort(void) {
    trace_report("SIGABRT (runtime abort, see runtime message)");
    _exit(134);
}

void trace_init(void) {}
