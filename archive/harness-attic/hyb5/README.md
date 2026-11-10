# hyb5(決定 0227): 行カーネル(packed 整数命令)を直接 wasm にする混成

`fe-wasmgen` の opt-in 旗 `--sse-int` で、SSE2 の packed 整数・シャッフル・ビット移動の命令(PADDUS*/PSUBUS*/PADDS*/PSUBS*・PMULLW/PMULHW/PMULHUW・PMADDWD・PAVG*・PMIN/PMAX*・PSADBW・PINSRW・PSRLQ/PSLLQ・PSHUFD/LW/HW・PUNPCK*・PACK*・PCMP*・PSLL/PSRL/PSRA* ほか)を wasm の v128 命令 1 本ずつにする。MXCSR の窓(packed 浮動小数点)は `--mx-packed` のまま。旗なしの出力は従来と同一。

- `seeds/hyb-sse2-F2x-<m>.txt`: 混成の選択リスト(`0x<va>` 1 行 1 関数、fle 730・dsp 33・b2d 31・qfc 163・ft 45・ilt 7 = 1,009 関数)。現行 `seeds/hyb-p1000r-F1x-*.txt`(977 関数)の上位集合で、差分の 32 関数は『旗なしでは lower できず `--sse-int` では lower できる』かつ『計数ビルドか V8 プロファイルが実行を見た』物。作り方は `docs/decisions/0227-simd-lowering.md` §4。
- 使い方(`docs/investigations/ship1/tools/hyb.sh` の ship2 版と同じ段階。fe-wasmgen の引数だけが違う): `FEW_EXTRA="--static-calls --sync-saved --sse-int"` を環境変数で渡し、`LISTS` に上のリストを `lowered-<m>.txt` の名前で置いたディレクトリを指す(`lowered-all.txt` は全部の和集合)。旗を付けずにこのリストを使うと n0 が『listed but not emitted』を出す(32 関数は lower できない)。
- 差分テスト: `cargo test --release -p fe-wasmgen --test simd_int_forms`(94 形 × 100,000 入力 + mutation 94 形。`FE_SIMD_N=1000000` で 1e6)、`-- --ignored every_16bit`(16 ビットレーン 13 形の全 2^32 順序対の digest 比較。`FE_SIMD_ONLY=<形>`、`FE_SIMD_A=<from>..<to>` で絞る)。
