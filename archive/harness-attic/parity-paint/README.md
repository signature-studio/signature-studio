# parity-paint — 決定 0226 §7(戻らないイベント)の記録手段

- `parity_traceall.c`: 全 `fle_imp_*` の呼び出し(名前、RCX、RDX、戻り番地、RAX)を `parity_trace_all_on` の間 stderr に出す(746 個。`--wrap` で包む)。native の試験ビルド専用。wasm・既定のビルドには入らない。
- `traceall-wrap-flags.txt` / `customtrace-wrap-flags.txt`: そのリンクフラグ(`parity_wrap.c` が既に包む API を除く 746 個、と `parity_wrap.c` の戻り値つきの 9 個)。
- `native-link-trace.sh`: N1 と同じ構成(`lift/harness/parity/native-build.sh` が作る .o)に `TRACEALL=1` で上の記録を付けてリンクする。
- `repro/`: G3 スクリプト(ハーネスの模擬入力。ユーザー操作ではない)。`hang6_6.native.g3` = 5 歩 + 戻らない 6 歩目、`hang2.native.g3` / `hang2.wasm.g3` = 長い形(wasm は node)。
- 説明: `docs/investigations/parity1-paint/PAINT-NOTES.md`。
