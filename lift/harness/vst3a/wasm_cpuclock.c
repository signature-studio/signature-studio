/* Decision 0177 (srm1): the CPU clock of the wasm build of the Serum 2 harness. emscripten answers every clock id with performance.now() (wall time); main_vst3a.c measures the
 * process phase with clock_gettime(CLOCK_THREAD_CPUTIME_ID). The wasm link wraps clock_gettime (--wrap=clock_gettime, build_vst3a_wasm.sh): that id is answered by the CPU time of the
 * calling thread of node (process.threadCpuUsage(), microseconds); the other ids are unchanged. Where that function does not exist (a browser Worker) the JS side answers
 * performance.now() / 1000 (wall time; the caller says so). Nothing else is changed: the lifted code and the runtime do not see this. */
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#include <time.h>

EM_JS(double, srm1_thread_cpu_s, (void), {
    if (typeof process !== 'undefined' && typeof process.threadCpuUsage === 'function') {
        var u = process.threadCpuUsage();
        return (u.user + u.system) * 1e-6;
    }
    return performance.now() * 1e-3;
});

int __real_clock_gettime(clockid_t id, struct timespec *t);

int __wrap_clock_gettime(clockid_t id, struct timespec *t)
{
    if (id == CLOCK_THREAD_CPUTIME_ID) {
        double s = srm1_thread_cpu_s();
        t->tv_sec = (time_t)s;
        t->tv_nsec = (long)((s - (double)t->tv_sec) * 1e9);
        return 0;
    }
    return __real_clock_gettime(id, t);
}
#endif
