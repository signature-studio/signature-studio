/* A wasm TEST build only (lift/harness/parity/make-test-tree.sh puts this file in the place of lift/runtime/fle_test_env.h in a COPY of the tree; the product tree never has it): the test entries read the environment of
 * the Emscripten module, so the node rows can give the harness its test inputs (FLE_G3_SCRIPT, FLE_FILEDLG_PICK ...). */
#ifndef FLE_TEST_ENV_H
#define FLE_TEST_ENV_H
#include <stdlib.h>
#define FLE_TEST_GETENV(name) getenv(name)
#endif
