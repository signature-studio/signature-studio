/* Decision 0234 (mt3, B-prime stage 1; decision 0243: the wasm build always has it): the thread-local block that holds the guest register file R (+0x000), the flags FL (+0x100) and the XMM file X (+0x200) (runtime/fle_lowregs.h).
 * One block per WebAssembly instance (per Worker that runs guest code); the cooperative guest threads of one instance still share it and still save and restore it by value (fle_thread.c). The direct wasm of
 * fe-wasmgen --tls-ctx addresses it as `global.get __tls_base; load offset=<offset of fle_ctx in the TLS segment + the field>`; the build reads that offset from the link map (symbol fle_ctx).
 * On native this file is empty. */
#ifdef __EMSCRIPTEN__
#include "fle_runtime.h"   /* FLE_LOWREGS (the wasm target), fle_lowregs.h, fle_flags.h */
/* The flags start undefined (FLE_UNDEF = 0xff in each of the six bytes of fle_eflags), as the initial FL of the generated C does for the native arrays; here it is the initialised TLS image, so every thread (instance) starts so. */
_Thread_local unsigned char fle_ctx[FLE_CTX_SIZE] __attribute__((aligned(64), used)) = {
    [FLE_CTX_FL + 0] = 0xff, [FLE_CTX_FL + 1] = 0xff, [FLE_CTX_FL + 2] = 0xff, [FLE_CTX_FL + 3] = 0xff, [FLE_CTX_FL + 4] = 0xff, [FLE_CTX_FL + 5] = 0xff,
};
_Static_assert(sizeof(fle_eflags) == 6, "fle_ctx_tls.c: the six flag bytes");
#endif
