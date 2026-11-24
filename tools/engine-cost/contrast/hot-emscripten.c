// Stage 0 contrast, the emscripten side: hot is an EM_JS import, so its JS body reaches wasm through the emscripten glue.
// The same loop is in hot-direct.c. contrast.mjs builds this file with emcc.
#include <emscripten.h>

EM_JS(int, hot, (int x), { return x + 1; })

int run(int n) {
  unsigned int s = 0;
  for (int i = 0; i < n; i++) s += (unsigned int)hot(i);
  return (int)s;
}
