/* A5 oracle, native side: executes the real machine code of FLEngine_x64.dll (mapped at its
 * ImageBase by harness/image.c) under the MS x64 ABI through ms_call.S. Pure-compute
 * functions only: an IAT slot holds its own slot VA, which is not executable, so a call
 * through an import faults (native_fault). Usage: native_drv CASES OUT PE */
#include <string.h>

#include "oracle_common.h"

extern void oracle_ms_call(const oracle_cpu *in, uint64_t fn, uint64_t sp);

static int exec_native(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out) {
    (void)pe;
    oracle_ms_call(&c->in, c->fn, OR_STACK_TOP);
    *out = oracle_ret_cpu; /* rsp after ret is OR_STACK_TOP, the same value lift_drv reports */
    return 0;
}

int main(int argc, char **argv) {
    return oracle_main(argc, argv, "native", exec_native);
}
