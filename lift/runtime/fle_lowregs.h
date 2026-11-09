/* The guest register file R and the flags FL (and the XMM file X, fle_sse.h) of the wasm build (decisions 0155, 0234; decision 0243: this is the only layout, there is no -D for it).
 * R, FL and X are the first 0x300 bytes of ONE thread-local block `fle_ctx` (runtime/fle_ctx_tls.c), R +0x000, FL +0x100, X +0x200, so that another WebAssembly instance (a second Worker on
 * the same shared memory) has its own guest register file. In wasm every access to a data symbol is `i32.const <index>; i64.load offset=<address>` and the address of a symbol (GLOBAL_BASE =
 * 2^28) takes 5 bytes of LEB128; the block is addressed from __tls_base. The generated C must not define the names: tools lowregs.py (lift/harness/two/lowregs.py) puts their definitions and
 * extern declarations in `#ifndef FLE_LOWREGS` / `#ifndef FLE_LOWREGS_X`, which fle_runtime.h defines for the wasm build. The generated C is the same text (it uses the names R, FL, X); the
 * direct wasm of fe-wasmgen (--tls-ctx) addresses the block as `global.get __tls_base` + the offset of `fle_ctx`. */
#ifndef FLE_LOWREGS_H
#define FLE_LOWREGS_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" { extern thread_local unsigned char fle_ctx[]; }
#else
extern _Thread_local unsigned char fle_ctx[];
#endif
#define FLE_CTX_R  0x000
#define FLE_CTX_FL 0x100
#define FLE_CTX_X  0x200
#define FLE_CTX_SIZE 0x300
#define R (*(uint64_t (*)[16])(fle_ctx + FLE_CTX_R))
#define FL (*(fle_eflags *)(fle_ctx + FLE_CTX_FL))
#endif
