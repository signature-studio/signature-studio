/* measurement only: emscripten_return_address (the import that clang's -finstrument-functions hook makes through __builtin_return_address) as a wasm function that returns 0, so that the hook does not call JS */
void *emscripten_return_address(int level) { (void)level; return 0; }
