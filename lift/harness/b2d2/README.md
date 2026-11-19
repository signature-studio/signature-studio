# lift/harness/b2d2: 構成 E(D + Blend2D 置換)の生成と wasm での検証(決定 0167、仮)

opt-in。通常のテスト・ビルドには関わらない。決定 0160(`lift/harness/b2d1/`)のアダプタ・種・置換入口を、決定 0161 のバンドル(`app/scripts/build-engine-bundle.sh`)に載せる。

| ファイル | 内容 |
|---|---|
| `gen-direct.sh` | 生成段階(`B2D_SWAP=1` または `HLE_DSP=1` のとき `build-engine-bundle.sh` が呼ぶ)。`a4-multi.sh` のモジュール欄は `--swap-entries`・`--hle-entries` を表せず、その native のビルドは上流 Blend2D のアダプタを要るので、`gen_two.sh rebase gen split merge` と realmod 表だけを作る |
| `run-g3.sh` | wasm のエンジンを node で走らせ、g3 のスクリプト(既定 16 スナップ)の g3-snap / surface / wintable / progress の行を出す |
| `g3cmp.py` | 2 つの g3 行ファイルの比較(`entries N` と g3-layered は比べない) |
| `cpuhook.cjs` | `run-g3.sh` が `--require` する、スナップごとのメインスレッド CPU 秒の出力 |
| `scan-compare.py` | Agg2D 走査(決定 0120/0160)の `RESULT`/`CHECK`/`VALUE` の行を、wasm の走行ディレクトリと native の基準ディレクトリで、テストごとに比べる |
| `expected/` | 期待値(82 行): `g3-lines-D-default.txt` = 構成 D(lift した Blend2D)の既定の progress の数え方。`g3-lines-E-default.txt` = 構成 E の既定の数え方(**新しい基準**)。`g3-lines-split.txt` = Blend2D の単位だけ progress を別に数える検証構成(D も E も同じ 82 行) |

## 検証の手順(画素の等価の証明は 3 の `split`)

1. 作り方: `app/scripts/build-engine-bundle.sh`(`HLE_DSP=1 B2D_SWAP=1 B2D_UPSTREAM_LIB=… B2D_UPSTREAM_SRC=…` と決定 0161 の D のオプション。`docs/investigations/b2d2/MERGE-NOTES.md`)の `gen resplit wasm` までで objects ができる。**node の確認用リンク**は `lift/harness/two/build_nmod_wasm.sh link` を `OBJ=<objects>` で(`LINK_EXTRA="<b2d_swap.o> <libblend2d.a>"`、`LINK_OPT=O0`、node + NODERAWFS が既定)。`wasm-opt -Oz`(`build-engine-bundle.sh` の `wasmopt` 段階と同じ引数)をかけた wasm は、ブラウザ用の `weblink` の wasm と**バイト同一**(リンクの JS だけが違う: 決定 0167 で確認)ので、node で走らせるのは配布する wasm そのもの。
2. 既定の数え方(`expected/g3-lines-E-default.txt`): `WASMDIR=<E の node リンク + wasm-opt> DLLDIR=<WORK>/onat/v-*/dll REPO=<binaries のある checkout> bash lift/harness/b2d2/run-g3.sh` → `python3 lift/harness/b2d2/g3cmp.py lift/harness/b2d2/expected/g3-lines-E-default.txt g3-run/g3-lines.txt` が `IDENTICAL (82 lines)`。D(置換なし)は `expected/g3-lines-D-default.txt`。**E と D は 82 行中 48 行(snap 16、progress 16、surface 16)が違う**(Blend2D の内部の間接呼び出しが `fle_progress_count` に入らないため。決定 0160 §6.3)。
3. 分離構成(`split`): `B2D_PROGRESS_SPLIT=1` で `wasm` 段階をやり直す(`m2-*` の単位だけが `-Dfle_progress_count=fle_progress_count_b2d -Dfle_progress_limit=fle_progress_limit_b2d` 付きで再コンパイルされる。他の単位は同じ)。D も E もこの構成の `run-g3.sh` の出力が `expected/g3-lines-split.txt` と一致する(34 面の SHA を含む 82 行)=**Blend2D を置換しても画素・クロック以外の状態は変わらない**。`B2D1_CHECK=1` を付けると置換版のアダプタが各呼び出しの後に鏡を検査する。
4. 走査(Agg2D の 130 スロット、`lift/harness/b2d1/scan_all.sh` の wasm 版): 走査用の node リンクは、FRUITY の h-fruity の `main_two.o` を `-DFLE_B2D_SCAN=1` でコンパイルした物と、`b2d_scan.c`(置換版は `-DB2D1_SWAP=1`)に差し替える(`R`・`X` が固定アドレスなので、コンパイルにも `-DFLE_LOWREGS -DFLE_LOWREGS_X` が要る)。1 テスト 1 プロセス: `FLE_B2D_SCAN=<test> FLE_B2D1_RET=1 [B2D1_CHECK=1 B2D2_STRICT=0] node --experimental-wasm-jspi lift/harness/two/run_node_fruity.cjs <FLEngine> <5 modules>`。比較: `python3 scan-compare.py <wasm の出力ディレクトリ> <native の基準>`(決定 0160 の `n-full-base`〔142 本〕と `fz-base`〔fuzz 41〜300〕)。`B2D2_STRICT=0` は走査用(DLL も同じエラーを黙って返すので、同じ『描かない』結果を比べる)。省くと JIT 専用の合成演算で名前つきで止まる(rc=5。`blendmode:N` の 29 本のうち 20 本)。

## 既知の差(許容ではなく説明)

- `dtor`(走査): 破棄の後にテスト側が解放済みの Agg2D オブジェクトの状態を読む行。native では `state none`、wasm では D も E も解放済みメモリの値を読む(wasm のヒープは解放した領域をそのまま残す)。画素(`hash=0x5ebb39d31700c325`)と他の行は一致。
