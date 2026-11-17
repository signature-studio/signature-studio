// mt1 experiment (iii-b): does JSPI (WebAssembly.Suspending / promising) work on a pthread (a Worker on the shared Memory)?
// Two pthreads each call an async JS import 5 times (setTimeout) and also call each other's "cooperative" pattern: the thread suspends its JSPI stack while the other keeps running.
#include <emscripten.h>
#include <pthread.h>
#include <stdio.h>
#include <stdint.h>

EM_ASYNC_JS(int, delay_js, (int ms), { await new Promise((r) => setTimeout(r, ms)); return ms; });

static volatile int progress[2];
static void *thr(void *p)
{
    int id = (int)(intptr_t)p, sum = 0;
    for (int i = 0; i < 5; i++) { sum += delay_js(10 + id); progress[id]++; }
    printf("thread %d done sum=%d progress=%d,%d\n", id, sum, progress[0], progress[1]);
    return (void *)(intptr_t)sum;
}

int main(void)
{
    pthread_t t[2];
    void *r0, *r1;
    for (int i = 0; i < 2; i++) pthread_create(&t[i], 0, thr, (void *)(intptr_t)i);
    // the main (proxied) thread also suspends while it waits
    int m = delay_js(5);
    pthread_join(t[0], &r0); pthread_join(t[1], &r1);
    printf("main m=%d r0=%d r1=%d\n", m, (int)(intptr_t)r0, (int)(intptr_t)r1);
    printf("DONE\n");
    return 0;
}
