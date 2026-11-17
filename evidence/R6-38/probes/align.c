#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
int main(void) { void *p = aligned_alloc(16, 64); void *q = NULL; int rc = posix_memalign(&q, 16, 64); printf("aligned=%d posix=%d rc=%d\n", (int)(((uintptr_t)p) % 16 == 0), (int)(((uintptr_t)q) % 16 == 0), rc); free(p); free(q); return 0; }
