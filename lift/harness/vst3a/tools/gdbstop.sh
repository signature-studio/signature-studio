#!/bin/bash
# gdbstop.sh BIN ARGS... : run BIN under gdb until fle_stop/vst3a_unanswered/abort and print the native backtrace (f_<VA> are lifted functions). usage: FLE_VFS_SEED=... gdbstop.sh BIN args
exec gdb -batch -nx -ex 'set pagination off' -ex 'break fle_stop' -ex 'break vst3a_unanswered' -ex 'break vst3a_stop' -ex 'break abort' -ex 'break fle_unsupported' -ex run -ex 'bt 40' -ex 'p/x *(unsigned long long*)R[4]' --args "$@" 2>&1 | grep -v "^\[" | tail -${TAILN:-60}
