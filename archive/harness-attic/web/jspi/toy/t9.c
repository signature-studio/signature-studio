#include <setjmp.h>
#include <stdio.h>
static jmp_buf jb;
__attribute__((noinline)) void leaf(void) { printf("leaf\n"); }
__attribute__((noinline)) void other(void) { printf("other\n"); }
__attribute__((noinline)) void via(void) { other(); if (setjmp(jb) == 0) { leaf(); } else { printf("back\n"); } }
int main(void) { via(); return 0; }
