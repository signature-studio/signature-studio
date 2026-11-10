# f2xm1 / fsincos のソフト実装の実行記録

日付: 2026-10-21。決定は `docs/decisions/0043-x87-trans.md`。出力は `lift/out-x87c/`（`tmp/` は TMPDIR、`target/` は CARGO_TARGET_DIR、`test/` はテストの生成物、`mut/` は変異体）。「実測」はツールの出力、「推測」は未検証の解釈。ホストは AMD Ryzen 9 9950X（Intel 実機は未確認）。

## 1. 実装

- `lift/runtime/fle_x87.c`: `fle_x87_f2xm1`、`fle_x87_fsincos`（+427 行、末尾に追記）。`fle_x87.h` に宣言 2 つ。
- `lift/crates/fe-lift/src/x87.rs`: `F2xm1`、`Fsincos` の生成 2 行、PROTOS 2 行、冒頭コメント。`lib.rs` は触っていない。
- 256 bit の仮数（32 bit の肢 8 個）で `exp(y)−1`（y = x·ln2、ln2 は 256 bit に切り捨てた定数）と sin/cos のテイラー級数を計算し、真値が入る ±2^16 単位の範囲の両端が同じ丸めになることを確認する。ln2 の肢は `out-x87c/ln2const.py`（整数の atanh 級数と `decimal.ln` の 2 通りで一致を確認）。
- fsincos の縮約は SDM の Pi（`0x3243F6A8885A308D3`）の厳密な 128 bit 整数の除算。|x| < 2^−2 は縮約なし。|x| ≥ 2^63 は C2=1、ST(0) 不変、push なし。
- 事前の実測（この作業で行った）: `probe_host.c`（ホストの NaN/∞/非正規化/範囲外/スタック満杯での出力）、`exp_model*.c`（ホストが「Pi66 で縮約した sin/cos」のどれと一致するか。`exp_model4` が計測用、`exp_model` から `exp_model3` は x87 の clobber を付けていなかったため全件一致という誤った出力が出た。最初の 3 つの出力は使っていない）。

## 2. テスト（`crates/fe-lift/tests/x87_trans.rs`、新規 9 件）

`cargo test --offline -p fe-lift --test x87_trans`（既定）: **9 件通過、約 40 秒**。

| テスト | 内容 | 実測 |
|---|---|---|
| `matches_the_correctly_rounded_reference` | `gen_ref.py`（Python の decimal 240 桁と fractions だけ、mpmath・libm なし。ネットワークなし）で作った表と、ソフト実装の全状態（SW、TW、物理レジスタ 8 個）を比較 | **19,308 行、不一致 0**。f2xm1 7,848 行、fsincos 11,460 行。RC ごと f2xm1 各 1,962 行、fsincos 各 2,865 行。±0 16、±1.0 108、非正規化 1,328、極小 1,340（指数 < 2^-70）、∞ 8、NaN 48、未サポート形式 40、|x| ≥ 2^63 は 44（C2=1）。オペランドの下に積んだ値の数（fill）は 0、1、2、3、5、6、7 の 7 通りで、実行後の TOP も 7 通り。C0/C2/C3 の事前値は 6 通り |
| `differs_from_the_host_by_less_than_the_sdm_error_bound` | ホストの f2xm1/fsincos と比較（参考値）。要求: 最近偶数で 1 刻み以内、他の RC で 2 刻み以内、タグ語と TOP が同じ | 14 区分×20,000 = **280,000 回**。結果が違うのは **4,254 件**、**最大 1 刻み**（最近偶数でも 1）。C1 違い 1,890 件。PE/UE/DE/IE/C2 の違い 0、タグ語の違い 0、TOP の違い 0 |
| `is_monotonic` | f2xm1 の増加、sin の増加と cos の減少（0〜π/2）、境界 15 区間 | 226,920 点、違反 0 |
| `256_bit_intermediates_match_python_decimal` | ランタイムの静的関数を直接呼び、ln2 定数と級数の値（expm1、sin、cos）を Python の decimal と比較（相対誤差 2^-248 以内） | 49 件一致 |
| `rounding_window_stops_a_value_too_close_to_a_boundary` | 丸め境界の ±2^16 単位以内の値で abort、2^20 単位離れれば丸める | 通過 |
| `f2xm1_outside_minus_one_to_one_stops_with_a_named_abort` | |x| > 1、±∞ の 8 入力で abort | 通過 |
| `stack_faults_and_unmasked_exceptions_stop` | 空スタック、fsincos のスタック満杯（|x| ≥ 2^63 も）、PE/IE/DE 未マスク、保留フラグ | 通過（12 ケース） |
| `out_of_range_fsincos_with_unmasked_pe_still_runs` | |x| ≥ 2^63 は PE 未マスクでも止まらず、SW=0x3c00、TW=0x3fff、ST(0) 不変 | 通過 |
| `lifted_text` | 生成 C の文字列 | 通過 |

- 参照表の独立性: 表の生成は C ランタイムのコードを使わない（Python のみ）。丸めの境界との距離は、v·(1±1e-200) が同じ丸めになることを行ごとにアサート（tiny の引数は 1e-100 未満で別扱い。真値が `v·(1−ε)` で厳密値の直下にある場合）。縮約は Fraction の正確な値、`sin²+cos²=1` を 1e-230 で確認、Pi の上位 66 bit を Machin の公式の整数計算で確認（`check_pi66`）。
- 既存テスト: `x87.rs`（14 件、`fsincos`、`f2xm1` を「未対応のまま」の一覧から外し、閉包の形の一覧に足した）、`x87ctl.rs`（7 件、`f2xm1` の行を `fpatan` に差し替え）は通過。

## 3. 変異テスト

`out-x87c/mutate_trans.py`（ランタイムの 39 箇所を 1 つずつ壊した .c を `FLE_X87_C` でテストバイナリに渡す。結果は `mutation-trans.txt` から `-4.txt`）。**39 変異すべてを最終的に検出**（4 つのファイルを順に読んで集計した結果: 検出 39、生存 0、未実行 0）。

- 1 回目（`mutation-trans.txt`、テスト 7 件の時点）: 38 変異を実行（`quadrant-3-sin-sign` はパターン不一致で未実行）。**6 件が生き残った**: `ln2-low-limb`（ln2 の最下位 limb を +1）、`expm1-stops-early`（級数の打ち切りを 2^-100 に）、`window-not-checked`（窓の両端の一致確認を外す）、`bump-too-small`（窓の幅を 0 に）、`ln2-limb5-plus-1`、`expm1-stops-at-2^-80`。生き残った理由は、256 bit の下位の誤差が 64 bit への丸めには見えないこと、窓の検査が丸め境界のすぐ近くの入力でしか効かないこと（推測。個別には確認していない）。
- 対処: テストに「256 bit の中間値を decimal と比較」と「丸め窓の境界」の 2 件を足し（上の表 4、5）、生き残った 6 件を再実行して**すべて検出**（`mutation-trans-4.txt`。ほかに追加の変異 `ln2-limb1-plus-1` も検出）。`quadrant-3-sin-sign` はパターンを直して検出（`-2.txt`）。`mutation-trans-3.txt` は追加の変異（`ln2-limb6-plus-1`、`ln2-limb5-plus-1`、`expm1-stops-at-2^-60`、`expm1-stops-at-2^-80`）を足した段階の結果で、そのとき `ln2-limb5-plus-1` と `expm1-stops-at-2^-80` はまだ生き残っていた（テストを足す前）。
- 足したテストは、生き残った変異を見てから作ったもの（後付け）。
- 検出した変異の種類: ln2（5 limb）、級数の打ち切り（4）、Pi66（2）、象限（4）、符号、最近整数への縮約、C2（2）、2^63 の閾値、ゼロの sin、スタック検査、結果の入れ替え、f2xm1 の ±1、非正規化フラグ（2）、極小の sin/cos の直下判定（2）、RC の無視、128 bit 乗算の桁上がり、減算の借り、cos 級数の分母、sin 級数の符号、範囲外 abort（2）、C1（2）、∞ の結果、窓（2）。（種類の数は目視の分類で、表と一致するか個別には数え直していない。）

## 4. ホストとの差の測定（参考値）

- `diff_host2.c`（`./diff_host2 100000`、`diff_host2-out.txt`）: クラス 7（通常、小、極小、大、非正規化、疑似非正規化の組み合わせなど）× 100,000、PC と RC は乱数、事前の C0/C2/C3 乱数。f2xm1: 差 13,698 件（1.96%）、fsincos: 6,219 件（0.89%）。最大差はどちらも 1 刻み。非正規化・極小のクラスでは fsincos は差 0、f2xm1 は 非正規化クラス 801 件と 極小 1,600 件。C0/C3 の差 0、タグ語の差 0。
- 誤差の測定（`errulp2.c`、`errulp-out.txt`。真値は libquadmath の `expm1q`、`cosq`、`sinq` で、測定の物差しとしてのみ使用。ランタイムは使っていない）: f2xm1（2^-60<|x|<1）: ホスト最大 0.62 ulp（最近偶数）、1.10〜1.14（他の RC）。ソフト最大 0.5000、1.0000。fsincos（2^-8≤|x|≤1.4）: ホスト最大 0.65、1.03〜1.16。ソフト 0.646、1.021〜1.145。縮約なしの領域（|x|<Pi66/4）ではソフトは 0.5000／1.0000。SDM の上限（最近偶数で 1 ulp 未満、他 1.5 ulp 未満）の内側。
- x = ±1.0 の f2xm1: ホストは PE を立てる（`pm1-out.txt`）。ソフトは厳密なので立てない。差として記録（決定 0043）。

## 5. wasm32

`out-x87c/wasm-inc/`（stdio.h、stdlib.h、string.h のスタブ）、`wasm_driver.c`（128 bit の補助関数 `__ashlti3`、`__lshrti3`、`__multi3`、`__udivti3`、`__umodti3` の簡易実装を含む）、`wasm_check.mjs`。

`clang --target=wasm32 -ffreestanding -O2` で `fle_x87.c` と `fle_x87ctl.c` をコンパイル、`wasm-ld` でリンクして（19 KB）node v24.19.0 で参照表 19,308 行を実行: **不一致 0、トラップ 0**（`wasm-check.txt`）。emcc は環境に無く未実施。実機の compiler-rt ではなく自前のスタブで検証した。速度は未測定。

## 6. all-pdata の再生成（実測）

`out-x87c/target/release/fe-lift lift FLEngine_x64.dll out-x87c/out/after-x87c.c --pdata`（`gen-x87c.txt`、2.83 秒、最大 RSS 320 MB）。生成 C 215,539,555 バイト（2 GB 以内）。

| 項目 | x87-run.md（前） | 今回 |
|---|---|---|
| 閉包の命令 | 2,526,826 | 2,526,826 |
| 関数 | 58,695 | 58,695 |
| `unsupported_total` | 667 | **601** |
| F で始まる未対応（F2xm1、Fsincos） | 2 | **0**（生成 C の `/* 0x… F… */` の `fle_unsupported` 行数。`grep` で 0） |

- `0x6a2a53 F2xm1` と `0x6a39ad Fsincos` は `fle_x87_f2xm1();`、`fle_x87_fsincos();` に生成された（`out/after-x87c.c`）。
- 差 −66 の内訳（`gen-x87.txt` との比較）: F2xm1 −1、Fsincos −1、Mulpd −25、Addpd −13、Cvtdq2ps −9、Subpd −8、Prefetcht0 −5、Cvtpd2ps −2、Cvtps2pd −2（計 −66）。x87 以外の −64 は、前回の生成の後にワークツリーへ入った他の担当の変更（SSE2 の packed 命令の対応）によるものと**推測**する（個別には確認していない）。この作業の寄与は 2 件。
- 閉包の x87 以外の未対応: `Jmp: indirect through IAT slot` 557、Psubw 10、Pcmpgtd 8、Rcpps 8、Bswap 7、`Test: high-byte register` 7、`And: high-byte register` 1、Andnpd 1、`Jmp: indirect target unresolved` 1、Xgetbv 1（計 601）。
- ビルド・実行（harness）は指示により行っていない。生成 C を `clang` でリンクしての確認は未実施。ランタイムの `fle_x87.c` は単体で `clang -O0` と wasm32 でコンパイルでき、テストの C プログラムでリンク済み。

## 7. ワークスペース全体のテスト

`cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0、フォアグラウンド）: **64 バイナリ、609 passed、0 failed、0 ignored**。`x87.rs` 14、`x87_trans.rs` 9、`x87ctl.rs` 7 を含む。この数はワークツリーにある他の担当のテスト（ETXTBSY 修正後のもの、ロケール）を含む。x87-run.md 時点の 588 件・1 失敗（ETXTBSY）との差 +21 のうち、私の分は 9 件（`x87_trans`）、残りは他の担当の変更。

## 8. F 系 stub の残数

生成 C の F で始まる mnemonic の `fle_unsupported`: **0 件**。ランタイムで未実装のまま（閉包に出ない）: fsin、fcos、fptan、fpatan、fyl2x、fyl2xp1、fprem、fprem1、fxtract、fnstenv ほか（0033）。

## 9. 触ったファイル

- 新規: `docs/decisions/0043-x87-trans.md`、`docs/investigations/x87-trans-run.md`、`lift/crates/fe-lift/tests/x87_trans.rs`、`lift/crates/fe-lift/tests/x87_trans/gen_ref.py`、`lift/crates/fe-lift/tests/x87_trans/check_bf.py`、`lift/out-x87c/` 全体（測定プログラム `probe_host.c`、`exp_model*.c`、`diff_host*.c`、`errulp*.c`、`pm1.c`、`quick.c`、`dbg.c`、`ln2const.py`、`mutate_trans.py`、`wasm_*`、`wasm-inc/`、出力、`mut/`、`test/`、`target/`、`out/after-x87c.c`、`ref1.txt`、`ref2.txt`、`test-ws.txt`、`test-ws.exit`）。
- 編集（追記・局所変更）: `lift/runtime/fle_x87.c`（末尾に追記）、`lift/runtime/fle_x87.h`、`lift/crates/fe-lift/src/x87.rs`、`lift/crates/fe-lift/tests/x87.rs`（`x87_forms_not_implemented_stay_unsupported` の 2 項目の削除、`x87_every_closure_form_lifts` に 2 項目の追加）、`lift/crates/fe-lift/tests/x87ctl.rs`（1 つの assert の差し替えとコメント）。
- 触っていない: `lib.rs`、ETXTBSY の変更リストのテストファイル、`fle_locale.c`、`tests/locale.rs`。他の担当の未コミット変更は読んでいない（`git status` で存在を見ただけ）。
- 残置物（削除禁止のため）: `out-x87c/` 内の `exp_model`〜`exp_model3`（誤った出力を出した試作。`exp_model4` が正）、`diff_host`（`diff_host2` の前の版）、`errulp`（`errulp2` の前の版、`errulp-out.txt` は後の版の出力）、`ref1.txt`（`gen_ref.py` の最初の版の途中で失敗した実行の部分出力、7,856 行。使っていない。`ref2.txt` が完成版で 19,308 行）、`w.c`、`w.o`。`out-x87c/` は約 2.3 GB（`target/` 1.8 GB、`test/` 204 MB、`out/` 206 MB）。2 GB を超える単一ファイルはない。
- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない。ただし 1 回、`rm -f /dev/null` を誤って実行して権限エラーになった（影響なし。`/dev/null` は削除されていない）。
