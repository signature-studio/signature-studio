/* decision 0243: the environment variables that switch diagnostics and test entries of the harness on (FLE_G3_SCRIPT, FLE_FRUITY_STOP_AT, B2D1_CHECK ...) are read through FLE_TEST_GETENV.
 * Native (the tests): getenv. wasm (the product): nothing is read (a null constant, so the test entries are not compiled in). A wasm TEST build (lift/harness/parity/make-test-tree.sh: the node snapshot rows,
 * the reference project) is made in a COPY of the tree in which this one file is replaced by lift/harness/parity/fle_test_env_wasm.h (getenv) and the objects that include it are compiled again: there is no
 * -D and no flag, the product tree and its build are the same whether or not a test build exists. Parameters that the manifest or the page passes (FLE_FRUITY_MAX_CALLS, FLE_FRUITY_SECONDS,
 * the seed lists, FLE_MONITOR_W / H, FLE_DWM_HZ ...) are read with getenv in both. */
#ifndef FLE_TEST_ENV_H
#define FLE_TEST_ENV_H
#include <stdlib.h>
#ifdef __EMSCRIPTEN__
#define FLE_TEST_GETENV(name) ((const char *)0)
#else
#define FLE_TEST_GETENV(name) getenv(name)
#endif
#endif
