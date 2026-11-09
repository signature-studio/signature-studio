/* LD_PRELOAD helper of the test runner (decision 0149).
 *
 * runner.sh sets RLIMIT_CORE to 1 byte for the test executable and everything it starts: a test program that aborts on purpose then does not wait
 * for the system core-dump handler (apport, a queue of about one event per second for the whole machine). A few generated test programs
 * (cursor.rs, loadimage.rs, wall_fr10.rs) call setrlimit(RLIMIT_CORE, {0, 0}) themselves to "switch core dumps off"; 0 does NOT stop a piped handler
 * (only 1 does), so that call would bring the handler back. This library turns a request for a core limit of 0 into a limit of 1: the same
 * intent (no core file), and no handler. Nothing else is changed. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <sys/resource.h>

static void clamp(int resource, const struct rlimit *in, struct rlimit *out) {
    *out = *in;
    if (resource == (int)RLIMIT_CORE) {
        if (out->rlim_cur == 0) out->rlim_cur = 1;
        if (out->rlim_max == 0) out->rlim_max = 1;
    }
}

int setrlimit(__rlimit_resource_t resource, const struct rlimit *rl) {
    static int (*real)(__rlimit_resource_t, const struct rlimit *);
    if (!real) real = dlsym(RTLD_NEXT, "setrlimit");
    struct rlimit n;
    if (rl) { clamp((int)resource, rl, &n); rl = &n; }
    return real(resource, rl);
}
int setrlimit64(__rlimit_resource_t resource, const struct rlimit64 *rl) {
    static int (*real)(__rlimit_resource_t, const struct rlimit64 *);
    if (!real) real = dlsym(RTLD_NEXT, "setrlimit64");
    struct rlimit64 n;
    if (rl) { n = *rl; if ((int)resource == (int)RLIMIT_CORE) { if (n.rlim_cur == 0) n.rlim_cur = 1; if (n.rlim_max == 0) n.rlim_max = 1; } rl = &n; }
    return real(resource, rl);
}

int prlimit(__pid_t pid, enum __rlimit_resource resource, const struct rlimit *nl, struct rlimit *ol) {
    static int (*real)(__pid_t, enum __rlimit_resource, const struct rlimit *, struct rlimit *);
    if (!real) real = dlsym(RTLD_NEXT, "prlimit");
    struct rlimit n;
    if (nl) { clamp((int)resource, nl, &n); nl = &n; }
    return real(pid, resource, nl, ol);
}
int prlimit64(__pid_t pid, enum __rlimit_resource resource, const struct rlimit64 *nl, struct rlimit64 *ol) {
    static int (*real)(__pid_t, enum __rlimit_resource, const struct rlimit64 *, struct rlimit64 *);
    if (!real) real = dlsym(RTLD_NEXT, "prlimit64");
    struct rlimit64 n;
    if (nl) { n = *nl; if ((int)resource == (int)RLIMIT_CORE) { if (n.rlim_cur == 0) n.rlim_cur = 1; if (n.rlim_max == 0) n.rlim_max = 1; } nl = &n; }
    return real(pid, resource, nl, ol);
}
