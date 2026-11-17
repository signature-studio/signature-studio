/* The instructions themselves on the host CPU (tests/ref/fle_mxcsr_flags_host.h of fe-lift), x86-64 only. */
#include "fle_runtime.h"
#include "fle_sse.h"
#include "mxfp_common.h"
#include "fle_mxcsr_flags_host.h"
#define PFX fle_hx_
#define MPFX FLE_HX_
#define TAG host
#include "mxfp_ops.h"
