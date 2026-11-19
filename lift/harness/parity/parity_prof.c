/* parity1 (decision 0226): a poor man's sampling profiler for the native build (perf is not allowed on this machine: perf_event_paranoid=4). SIGPROF every PARITY_PROF_US microseconds (default 1000) of process CPU time;
 * each sample is (interrupted RIP, virtual clock ms) appended to $PARITY_PROF (16 bytes). The first record is (address of parity_prof_anchor, 0): PIE base = anchor - the symbol's address in `nm`. Read by profsym.py. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdlib.h>
#include <signal.h>
#include <sys/time.h>
#include <ucontext.h>
#include <unistd.h>
#include <fcntl.h>
extern uint64_t fle_sys_clock_ticks(void);
void parity_prof_anchor(void) {}
typedef struct { uint64_t pc, ms; } rec;
static rec buf[4096]; static unsigned n; static int fd = -1;
static void handler(int s, siginfo_t *i, void *u) {
    ucontext_t *c = (ucontext_t *)u; (void)s; (void)i;
    buf[n].pc = (uint64_t)c->uc_mcontext.gregs[REG_RIP]; buf[n].ms = fle_sys_clock_ticks() / 10000u;
    if (++n == 4096) { ssize_t w = write(fd, buf, sizeof buf); (void)w; n = 0; }
}
__attribute__((constructor)) static void init(void) {
    const char *p = getenv("PARITY_PROF"); struct sigaction sa; struct itimerval it; long us = getenv("PARITY_PROF_US") ? atol(getenv("PARITY_PROF_US")) : 1000; rec h;
    if (!p || !*p) return;
    fd = open(p, O_WRONLY | O_CREAT | O_TRUNC, 0644); if (fd < 0) return;
    h.pc = (uint64_t)(uintptr_t)&parity_prof_anchor; h.ms = 0; { ssize_t w = write(fd, &h, sizeof h); (void)w; }
    sa.sa_sigaction = handler; sigemptyset(&sa.sa_mask); sa.sa_flags = SA_SIGINFO | SA_RESTART; sigaction(SIGPROF, &sa, 0);
    it.it_interval.tv_sec = 0; it.it_interval.tv_usec = us; it.it_value = it.it_interval; setitimer(ITIMER_PROF, &it, 0);
}
