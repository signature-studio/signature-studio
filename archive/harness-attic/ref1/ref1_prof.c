/* Decision 0229 (ref1): the sampling profiler of lift/harness/parity/parity_prof.c with the guest thread id of the sample (fle_thread_current(), 0 while no thread table exists) in bits 48..63 of the second word,
 * so that a native run can say which guest thread the CPU belongs to. Same record layout and PARITY_PROF / PARITY_PROF_US variables; read by tools/profsym2.py. Harness code, native only. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdlib.h>
#include <signal.h>
#include <sys/time.h>
#include <ucontext.h>
#include <unistd.h>
#include <fcntl.h>
extern uint64_t fle_sys_clock_ticks(void);
extern unsigned fle_thread_current(void) __attribute__((weak));
void parity_prof_anchor(void) {}
typedef struct { uint64_t pc, ms; } rec;
static rec buf[4096]; static unsigned n; static int fd = -1;
static void handler(int s, siginfo_t *i, void *u) {
    ucontext_t *c = (ucontext_t *)u; (void)s; (void)i;
    buf[n].pc = (uint64_t)c->uc_mcontext.gregs[REG_RIP];
    buf[n].ms = (fle_sys_clock_ticks() / 10000u) | ((uint64_t)(fle_thread_current ? fle_thread_current() & 0xffffu : 0u) << 48);
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
