#include <setjmp.h>
#include <stdio.h>
#include <emscripten.h>
static jmp_buf jb1, jb2;
__attribute__((noinline)) void inner2(void) { printf("inner2\n"); }
__attribute__((noinline)) void via2(void) { if (setjmp(jb2) == 0) inner2(); }
__attribute__((noinline)) void inner1(void) { printf("inner1\n"); via2(); }
__attribute__((noinline)) void via1(void) { if (setjmp(jb1) == 0) inner1(); }
int main(void) { via1(); printf("done\n"); return 0; }
