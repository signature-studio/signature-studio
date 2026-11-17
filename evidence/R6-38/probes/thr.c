#include <pthread.h>
#include <stdio.h>
static void *f(void *a) { (void)a; return NULL; }
int main(void) { pthread_t t; int rc = pthread_create(&t, NULL, f, NULL); printf("rc=%d\n", rc); if (rc == 0) pthread_join(t, NULL); return 0; }
