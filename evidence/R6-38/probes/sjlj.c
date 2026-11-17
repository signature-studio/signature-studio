#include <setjmp.h>
#include <stdio.h>
static jmp_buf jb;
static void deep(int n) { if (n == 3) longjmp(jb, 42); deep(n + 1); }
int main(void) {
  int r = setjmp(jb);
  if (r == 0) { deep(0); printf("no jump\n"); return 1; }
  printf("setjmp returned %d\n", r);
  return r == 42 ? 0 : 2;
}
