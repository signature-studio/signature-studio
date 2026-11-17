/* perf2 (decision 0138, PROVISIONAL): replaces harness/fruity_idle.c in a "no idle hook" wasm build. Without fle_thread_idle_hook the scheduler of fle_thread.c does what the native run with FLE_FRUITY_NO_IDLE_STOP=1 does when the first thread waits
 * for input with an empty queue and only a sleeping thread is left: it moves the virtual clock to the next deadline and goes on (the frames of thread 2 continue). fle_host_post_input is only exported because the link line of link-web.sh names it. */
#include <emscripten.h>
EMSCRIPTEN_KEEPALIVE int fle_host_post_input(unsigned lid, unsigned msg, unsigned wparam, unsigned lparam) {
    (void)lid; (void)msg; (void)wparam; (void)lparam;
    return -9;
}
