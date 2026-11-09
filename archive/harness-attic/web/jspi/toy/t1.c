#include <setjmp.h>
#include <stdio.h>
#include <emscripten.h>
EM_ASYNC_JS(int, susp, (int k), { await new Promise(r=>setTimeout(r,0)); return k+1; });
static jmp_buf jb;
__attribute__((noinline)) void inner(void) { printf("inner %d\n", susp(1)); if (susp(2)==3) longjmp(jb, 1); }
__attribute__((noinline)) void via(void) { if (setjmp(jb)==0) inner(); else printf("longjmp landed\n"); }
int main(void) { printf("a %d\n", susp(0)); via(); printf("b\n"); return 7; }
