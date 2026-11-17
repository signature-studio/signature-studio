/* TEST STUB (not emscripten): declarations only, so the emscripten branch of the runtime can be compiled by zig. */
#ifndef STUB_EMSCRIPTEN_H
#define STUB_EMSCRIPTEN_H
#include <stddef.h>
#define EMSCRIPTEN_KEEPALIVE __attribute__((used))
#define EM_JS(ret, name, args, body) ret name args;
#define EM_ASM(...)
#endif
