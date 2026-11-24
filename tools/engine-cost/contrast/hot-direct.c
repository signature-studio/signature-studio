// Stage 0 contrast, the direct side: hot is an ordinary wasm import (env.hot), supplied by the JS importObject with no glue.
// The same loop is in hot-emscripten.c. contrast.mjs builds this file with clang and wasm-ld.
extern int hot(int x);

int run(int n) {
  unsigned int s = 0;
  for (int i = 0; i < n; i++) s += (unsigned int)hot(i);
  return (int)s;
}
