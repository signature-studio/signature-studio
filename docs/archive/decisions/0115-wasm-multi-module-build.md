# 0115: 多モジュール構成の wasm ビルドの方針（setjmp の着地、測定ドライバー、locals 超過関数）

状態: **仮の決定**（ビルドと測定の方針。応答の値は決めない）。PM の決定を 2026-10-28 に記録した。根拠の測定は `docs/investigations/a4-six-module-wasm/wasm6-report.md`（測定担当の報告。PM は a4 の行とログの diff を読んだ。wasm のビルドの独立再現は未実施）。

## 背景

6 モジュール構成（FLEngine + dsp_ippv2、Blend2D、QFC、freetype、ILTools。測定用の基底、フォント 2 ファイルを配置）の native の a4 は `init_count=942 of 942` で完走する（決定 0113、独立再現 `a4-six-module-completion-verification.md`）。決定 0046 の A4 の判定は native と wasm の両方を挙げる（0046 の 98 行。A3 は M1 完了の条件ではない: 方針 8）。

## 決定

| # | 項目 | 決定 |
|---|---|---|
| 1 | wasm の setjmp の着地 | 多モジュール構成の wasm は `-DFLE_LANDING_FORCE`（emscripten 既定の setjmp／longjmp。追加のフラグは不要）でビルドする。既定の `FLE_LANDING_NOJUMP`（決定 0100 項 5）だと QFC の DllMain の setjmp の着地で名前付きで止まり、159/942 になる（実測）。`build_nmod_wasm.sh` の `VAR=force`／`VAR=nojump` で選ぶ |
| 2 | a4 の報告 | wasm の `trace.c` も `a4_sample()`（weak）を呼ぶ（決定 0113 が `trace_native.c` にだけ入れていた）。無いと完走しても a4 の行が `init_count=0 of ?` になる |
| 3 | 測定ドライバー | 標準は**未改造**のドライバー（`emscripten_return_address` が `-finstrument-functions` の各フックで JS のスタック解析をする。完走に 32 分 54 秒）。`FLE_WASM_FAST_RETADDR=1` で `emscripten_return_address` を `return 0` に差し替える**高速版**（2.9〜3.2 秒）を選べる。両者の stdout は同一（`rsp=` の行を除く）。報告には使ったドライバーを書く |
| 4 | locals が 50,000 を超える関数 | 19 個（dsp_ippv2 の既知の 15 個 + Blend2D の 4 個: `f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100`）を、1 関数ずつ別の翻訳単位にして `-O1` でコンパイルする（既存の流儀）。この 19 関数は native の完走で 1 回も入られない（gdb で 0 回）ので、`-O1` 版の計算が native と一致するかは**未測定** |
| 5 | wasm の A4 の読み | 6 モジュールの wasm で a4 が native と同一（init_count 942、895/895、`returned from entry`、終了コード 0）になった。0046 の A4 の wasm 側の判定として**使える**。ただし A4 の表記は決定 0101 の別表記（A4 完走・仮の応答 N 件の決定を含む）のままで、ブラウザは未検証、ページ保護は fail-open（決定 0045）、比べたのは a4 の行・functions・entries・深さ・先頭 256 件の entry log・rax まで |

## 測定の要点（出典: `a4-six-module-wasm/`）

- ビルド: 生成 C 約 581 MB（60 ファイルに分割）、`flengine6.wasm` 389,439,333 B（V8 が受理）、emcc 約 8 分（JOBS=2）、リンク 2.2 秒、emcc のピーク RSS 8.83 GB（dsp の単位 1 本ずつ）。
- 実行（FORCE）: a4 の 7 行が native と同一、functions 5,306、entries 1,007,942、終了コード 0。stdout の差は `calling entry … rsp=` の 1 行だけ。stderr は両方 0 バイト。3 回ともバイト同一。
- 実行（NOJUMP、既定）: 159/942、停止 `fle: jump to 0x1e9c210, the landing of a setjmp of the guest: …`。同じ NOJUMP の native と一致。

## 範囲外

ブラウザでの実行。wasm のビルドの再現性（2 回ビルドして SHA が一致するか）。ページ保護が働いたかの検査。レジスタ・メモリ値の比較。19 関数の `-O1` 版の正しさ。

## 撤回条件

(a) 決定 0100 の着地の方式が変わったとき。(b) emscripten の setjmp／longjmp が native の `_setjmp`／`_longjmp` と違う振る舞いをすると分かったとき。(c) 依頼者の別指示。

## 実装した場所

`lift/harness/two/build_nmod_wasm.sh`（新規）、`run_node_nmod.cjs`（新規）、`lift/harness/vfs_seed.h`（`__EMSCRIPTEN__` のスタブを撤去）、`lift/harness/wasm/trace.c`（`a4_sample()`）。差分は `lift/out-git/wasm6-results/wasm6.diff`（HEAD の archive に `patch -p1` で当たる）。

## 訂正（2026-10-30、`docs/investigations/jspi-real-wasm.md` による）

項 1 の「`-DFLE_LANDING_FORCE`（emscripten 既定の setjmp／longjmp。追加のフラグは不要）」は、**JSPI（`-sJSPI`）を使う構成では成り立たない**。emscripten 既定の setjmp／longjmp は JS の `invoke_*` ラッパーを通り、JS フレームは JSPI で中断できないため、中断を 1 回も要求しなくても `SuspendError` で止まる（実測）。JSPI 構成は `-sSUPPORT_LONGJMP=wasm`（wasm 例外処理。`build_nmod_wasm.sh` の `SJLJ=wasm`／`SJLJ=wasm-exnref`）が要る。JSPI を使わない既存の構成（本決定の測定、独立再現）は変わらない。決定 0100 の landing の C コードは変えていない。
