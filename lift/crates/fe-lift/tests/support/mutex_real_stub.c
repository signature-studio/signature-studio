/* Test-only definition of __real_pthread_mutex_unlock (#1382, fle_thread.c). The product links wrap pthread_mutex_lock and pthread_mutex_unlock (lift/harness/web/link.rsp: --wrap), and
 * the wrapper calls the libc's function under its __real_ name, which the linker resolves to that function. A native test link has no --wrap, so this gives the name the libc's function.
 * Linked by the tests that compile runtime/fle_thread.c, never into a product build. */
#include <pthread.h>

int __real_pthread_mutex_unlock(pthread_mutex_t *m)
{
    return pthread_mutex_unlock(m);
}
