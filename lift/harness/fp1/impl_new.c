/* The runtime's fle_mxcsr_flags.h with the fast path (decision 0143), counting hits and misses. */
#include "fle_runtime.h"
#include "fle_sse.h"
#include "mxfp_common.h"
#define PFX fle_mx_
#define MPFX FLE_MX_
#define TAG new
#define MXFP_HAS_HADDPD 1
#include "mxfp_ops.h"
