# V1: 独立検証 — 6 モジュール構成（フォント配置あり）の a4 完走（決定 0113）は再現するか

作業日: 2026-10-28。検証対象: HEAD **32848f0**（ブランチ main）の `git archive HEAD lift docs` を `/tmp/opencode/v1/src/` に展開したもの（コミットされた木そのもの。c1/src は使っていない）。検証者は worker ではない別のセッション。**測定のみ・ソース変更なし・commit なし**。

凡例: **実測**＝コマンドの出力を自分で見た事実／**読み**＝そこからの解釈（コード・逆アセンブルを読んだ範囲）／**未確認**。

## 0. 結論（先に）

| 項目 | 結果 |
|---|---|
| 再現（2 回、別 OUT で一からビルド、コミット済みの `a4-multi.sh` をそのまま使用） | **2 回とも** a4 `init_count=942 of 942`、`all_units_done=yes`、`runnable_started=895 completed=895 in_progress=0 of 895`、`== stop: returned from entry`（`runtime message: (none)`）、harness 終了コード **0**、functions 5,306、entries 1,007,942。native の SHA-256 は 2 回とも **`9c76bc6a9270bd99e874cfd2633ca99334ba1a276d2337db235ecf7a25a23785`＝worker の値と一致**（実測）。TAG `t2331d00e8ebb`、各モジュールの生成 C の SHA、FLEngine 註釈ありの生成 C の SHA（`e047beeb…`）も worker と一致 |
| 独立の確認（gdb。a4.c／fe-pe／worker の道具を使わない） | **InitCount の連続増加: 有**（ウォッチポイント 943 回の発火 = 1, 2, …, 942 が +1 ずつ、続いて 0。bt を取った 1, 2, 3, 941, 942 回目の書き手はいずれも `f_4120c0`＝InitUnits）。**unit proc の呼び出し: 895 / 895**（runnable の proc ごとに breakpoint。全 proc が**ちょうど 1 回**、`InitCount == index+1` のとき、`fle_dispatch ← f_4120c0 ← f_412160` から、unit の index 順に呼ばれた）。**0 戻り**: 942 の直後に `f_4128b0` が InitCount を 0 にし（FinalizeUnits 相当の `f_412020` を呼ぶ**前**）、`f_412770` が InitContext+0 を 0 にし、`f_4126f0`（ゲストの longjmp）で DllMain の setjmp へ戻る。**ただし worker の説明は 2 点不正確（§8）**: 「FreePascal」ではなく Embarcadero Delphi の RTL、「ユニットを finalize した」ではなく finalizer は 1 つも呼ばれていない |
| 回帰（コミットされた木） | **単独 a4: `init_count=145 of 942`、停止 `LoadLibraryW "dsp_ippv2_x64.dll"` のまま**（functions 822、entries 15,686、depth 10。`docs/progress.md` の 822 / 15,686 と一致）。**M0（フォント配置なしの 6 モジュール）: 246/942 のまま `StretchDIBits`（1x1 DDB）で停止**（functions 3,411、entries 134,085、depth 32、終了コード 134＝worker の値と同一） |
| 完走の性質 | 仮の決定の数 **N = 46**（README の索引表の行のうち「仮」列が `**仮**`）。実在モジュール 5（dsp_ippv2、Blend2D、QFC、freetype、ILTools）＋ FLEngine ＝ 6、VFS 種付け 2 ファイル。基底・種・註釈はすべて**測定用の値**。実行中に呼ばれた応答関数 `fle_imp_*` は **171 種・13,572 回**（名前付き停止は 0 回）。**実機 Windows との照合は 0 件。wasm は未確認** |

肯定的な結果だが、過大評価しない点: (1) 決定 0101 により、これは「A4 完走（仮の応答 46 件を含む）」という**別表記**の値で、決定 0046 の A4（外部 API の応答は仕様確認済みのみ）を文字どおりは満たさない。(2) 基底・種・註釈は測定用の値。(3) wasm 未測定（0046 の判定は native と wasm の両方）。(4) a4 が報告する `init_count=942` は、ゲストが完走後に InitCount を 0 に戻すため、**関数入口でサンプルした最大値**（`a4_sample()`）であって、報告時点のゲストメモリの値ではない（§5、§8）。

## 1. 環境と手順の制約

- 作業は `/tmp/opencode/v1/`（tmpfs）。`/` は空き約 51 MB で、成果物のテキストだけを書く。作業ツリー（`/home/ubuntu/work/signature-studio-develop`）は読み取りと `git archive`／`log`／`status`／`show`／`diff` だけ。作業の前後で `git rev-parse HEAD` = `32848f0478bfd9c93eccac6b03ae9551055e8ab4`、`git status --short` は同じ 5 件の未追跡（`docs/investigations/.tmp-readme-kari`、`README-0101-line.txt`、`lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`。私が作ったものではない）。
- ディスク（tmpfs quota）: `/tmp/opencode` の使用量（`du -sm`）の観測値: 着手時 868 MB、o1 のビルド中 1,124／1,512／1,540／1,913 MB、o1 完了＋M0＋gdb 後 1,956 MB、o2 完了時 **2,081 MB（観測した最大）**、o3（単独）完了時 1,744 MB。2.3 GB は超えていない（連続監視はしていない。サンプルした時点の値）。quota 超過・`No space left` は 0 回。
- 測定の道具: **コミットされた `lift/harness/a4-multi.sh`（32848f0）を一切変更せずに**使った。変えたのは PATH だけ（`tools/wrap/clang`、`tools/wrap/python3`。ディスク対策）:
  - `clang` ラッパー: `-g` を外す。コンパイルに成功した分割ソース（`/tmp/opencode/v1/o*/{src,v-*/src}/<label>-NN.c`）を消す（worker の `tools/wrap/clang` と同じ考え方）。
  - `python3` ラッパー: `split_mod.py` が成功した後に、その入力（モジュールの生成 C 全体）を消す。
  - worker の複製 `a4-multi-c1.sh` との差（`diff` で確認、実測）: `HERE` の固定、既定 `EXPECT_FLE_SHA`（複製は旧値 `508b76fb…` のまま、コミット版は `f6d3a971…`。どちらも測定では環境変数で上書き）、`RELINK`／`SHARE_FLE` の節、生成 C の削除（TRIM）。新しい OUT から一からビルドする限り、RELINK／SHARE_FLE は働かず、TRIM は削除だけ。
  - `-g` が `.text`／`.rodata` を変えないこと: ランタイム 2 ファイル（`fle_sys.c`、`fle_gdi.c`）を `-g` あり・なしでコンパイルし、`objcopy -O binary -j .text/.data/.rodata` の SHA が一致（`logs/g-flag-check.txt`、実測。**サンプル 2 ファイルのみ**）。決定的な裏づけは、**native の SHA-256 が worker（同じく `-g` なし）の値と一致**したこと。
- 注意: 6 モジュール構成に FLEngine の註釈（`FLE_HOOKS`）を付けると生成 C は註釈なしの値（コミット版の既定 `EXPECT_FLE_SHA` = `f6d3a971…`）と異なるので、`EXPECT_FLE_SHA=e047beeb44f16aa406a6ec254cc6603be192044206195c62ea2d459ec88bfdd2` を渡した（worker と同じ値。gen の検査は「(expected)」で通った）。**既定のままでは 6 モジュールの測定は gen で止まる**（スクリプトの先頭コメントどおりの仕様）。

## 2. コミットされた木のビルド（実測）

- `cd /tmp/opencode/v1/src/lift && CARGO_TARGET_DIR=/tmp/opencode/v1/target CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --offline`（dev）: 成功、**警告 0・エラー 0**（2.5 秒。`logs/cargo-build-dev.txt`）。
- release: `CARGO_TARGET_DIR=/tmp/opencode/v1/target-rel CARGO_INCREMENTAL=0 cargo build --release --offline -p fe-pe -p fe-lift`: 成功、警告 0・エラー 0（3.4 秒。`logs/cargo-build-rel.txt`）。release の `fe-lift` の SHA-256 `3a6ff3c8ac068807c6b79ca9a76ccd7a2bc8218ee9f0c2e9ad25e344782467f6` は worker の `c1/target-rel/release/fe-lift` と**同一**（実測。ビルドが再現可能）。run 2 では OUT 内で `fe-pe`／`fe-lift` を release で作り直し（`a4-multi.sh` の既定動作）、その `fe-lift` も同じ SHA だった。
- どちらを使ったか: FLEngine の生成 C の SHA は **dev と release の両方**で取った。a4 の 2 回は release の `fe-lift`／`fe-pe`（1 回目は `target-rel`、2 回目は OUT 内で一からビルド）。
- テスト（`cargo test`）は**実行していない**（依頼の範囲外。worker の「1,575 passed」は未確認）。

## 3. FLEngine の生成 C（註釈なし）の SHA（実測）

`fe-lift lift FLEngine_x64.dll OUT.c --pdata`（`FLEngine_x64.dll` SHA-256 `b2c64da4…f9496d`）:

| fe-lift | ファイルの SHA-256 | functions | bad_nodes | unsupported_total | c_bytes |
|---|---|--:|--:|--:|--:|
| release | `f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c` | 59,380 | 0 | 390 | 215,773,171 |
| dev | 同じ（`cmp` で同一。stdout の統計も同一） | 同じ | 同じ | 同じ | 同じ |

期待値（`f6d3a971…`、390、59,380、0）と**一致**。215 MB の C ファイルは検査後すぐ削除した。註釈あり（`--hook-points flengine-hook-points.txt`）の SHA `e047beeb…` は a4-multi の gen 段階で 2 回とも検査を通った（`logs/r1/gen.txt`、`logs/r2/gen.txt`）。

## 4. 6 モジュール構成の a4（再現 2 回）

コマンド（`tools/run6.sh`。コミット済みの `a4-multi.sh` を呼ぶ）:

```
FEBIN= bash tools/run6.sh /tmp/opencode/v1/o2      # run 2: OUT 内で fe-lift/fe-pe も一から作る
bash tools/run6.sh /tmp/opencode/v1/o1             # run 1: 別 OUT（FEBIN=target-rel/release）
# どちらも: FLE_HOOKS=<tree>/lift/harness/hooks/flengine-hook-points.txt  EXPECT_FLE_SHA=e047beeb…
#           VFS_SEED=<tree>/lift/harness/seeds/vfs-seed-fonts.tsv  VFS_SEED_ROOT=/home/ubuntu/work/signature-studio-develop  JOBS=8
# モジュール（NAME:DLL:BASE[:SEEDS[:OPTS[:HOOKS]]]）:
#   dsp_ippv2_x64.dll:lift/out-ext/dsp_ippv2_x64.dll:0x2000000:lift/out-three/three/dll/seeds-idx0-plus-initterm2.txt
#   blend2d.dll:lift/out-blend2d/Blend2D.dll:0x4700000:lift/out-seed58/seeds/seeds58-4700000.txt:pdata
#   quickfontcache_x64.dll:lift/out-qfc/quickfontcache_x64.dll:0x1d00000::pdata:harness/hooks/qfc-hook-points.txt
#   freetype_x64.dll:lift/out-wallQ/freetype/freetype_x64.dll:0x4b00000:harness/seeds/ft-seeds-4b00000-c1.txt:pdata
#   iltools_x64.dll:lift/out-inventory/extracted/ILTools_x64.dll:0x6000000:harness/seeds/iltools-6000000-seeds400.txt:pdata:harness/hooks/iltools-hook-points.txt
```

| | run 1（OUT o1） | run 2（OUT o2） | worker（`c1-results/logs/final/`） |
|---|---|---|---|
| a4 の行（`a4:` で始まる全 7 行） | 下 | **run 1 と完全に同一**（`diff` 空） | 同一（`diff` 空） |
| stop | `== stop: returned from entry`、`runtime message: (none)`、`abort kind: (not a runtime message)`、`call depth at stop: 12` | 同じ | 同じ |
| functions / entries | 5,306 / 1,007,942 | 同じ | 同じ |
| harness 終了コード / a4-multi.sh の終了コード | 0 / 0 | 0 / 0 | 0 |
| native の SHA-256 | `9c76bc6a…a23785` | `9c76bc6a…a23785` | `9c76bc6a…a23785`（o5、o6） |
| TAG | `t2331d00e8ebb` | 同じ | 同じ（入力ファイル・種・註釈のバイト列が一致している印） |
| 生成 C の SHA（先頭 16。m1〜m5） | `6e9f5fdb…`、`733a6a50…`、`eaf6eacf…`、`b800faab…`、`33554184…` | 同じ | 同じ |
| ステージ時間 | gen 13 s、build 37 s | gen 13 s、build 39 s | gen 14 s、build 39 s |
| stderr | `native.err` 0 バイト、`survey.err` 0 バイト。a4-multi の stderr は `stage gen`／`stage build` の 2 行（run 2 は先に fe-pe/fe-lift のビルドの 1 行） | 同様 | — |

a4 の 7 行（実測。run 1、run 2、worker で同一）:

```
a4: note: InitContext+0 is 0 now (the program's exit path finalised the units); the table 0x13768b0 and init_count=942 are the last non-zero header and the largest InitCount sampled at lifted function entries
a4: init_count=942 of 942, last_unit_proc=0x13751d0
a4: last_unit index=940 run_rank=895 of 895 proc=0x13751d0 state=returned
a4: runnable_started=895 completed=895 in_progress=0 of 895 runnable
a4: unit[init_count-1] proc=0x0; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=yes functions_distinct=5306 function_entries=1007942
```

- `a4: note:` の行が出ている＝**`a4_sample()` が実際に使われた**（実測）。ゲストが完走後に InitContext+0 と InitCount を 0 に戻すため、報告時点のメモリは 0 で、a4 はサンプルした最後の非 0 の表（0x13768b0）と最大の InitCount（942）を使っている。
- native のログは worker の最終ログと **1 行だけ**違う（`calling entry … rsp=` のスタック番地。ASLR）。units.tsv（fe-pe の表）も worker と同一。
- 追加の決定性の確認: 同じ native バイナリ（o1）に対し `a4-survey.sh` を 3 回（`tools/survey-direct.sh`、種付けあり）走らせ、a4 の行・stop・functions・entries・終了コードの md5 が 3 回とも同一（`logs/det/`）。
- ホストの実ファイルをゲストに見せていないこと: VFS 種付けは `vfs-seed-fonts.tsv` の 2 ファイルだけ（`vfs seed: done, 2 files, 6 directories created`。ホスト側の SHA-256 `588c4bf4…`／`578b46df…` を私が別に `sha256sum` して一致を確認）。**未確認**: この 2 ファイルが実インストーラのペイロードと一致するか（インストーラはこの環境に無く、再抽出していない。リストの SHA と一致することだけ確認）。

## 5. 独立の確認（gdb。実測）

方法（`gdb/v1watch.py`。worker の `a4.c`／`fe-pe`／`tools/` を使わない）:

- unit 表は**自分で書いた PE パーサ**（`tools/pe_units.py`）で `FLEngine_x64.dll` から直接読んだ: 表のヘッダ 0x13768b0、unit 数 **942**、units 0x13768e0、runnable（`proc != 0` かつ proc の先頭 8 バイトが非 0。`f_4120c0` の 0x412115〜0x412124 を逆アセンブルで確認した条件）**895**、empty 46、noproc 1。fe-pe の表（worker の `units.tsv`）と**全 942 行が一致**（proc、finalizer、種別）。
- ゲストのメモリは native でもゲストの VA にそのまま置かれる（`image: base=0x400000`）ので、InitCount（0x15EAC98）と InitContext+0（0x15EAC90）にはそのままハードウェアウォッチポイントを置ける。ホスト側のコードの番地だけが ASLR で動く（`f_<va>` のシンボル名で指定）。
- `a4_set_image` で止めてから: ウォッチポイント 2 つ、runnable な 895 個の proc（`*f_<va>`）に breakpoint（止まらず記録）、finalizer（各 unit の +8）896 個に breakpoint、ゲストの setjmp `f_412680`／longjmp `f_4126f0`（＋QFC の `f_1d0f9c0`、ILTools の `f_600e090`）、`trace_report`。

結果（`logs/gdb1/analysis.txt`、`result.json`。バイナリは run 1 のもの。run 2 と SHA 同一）:

| 確認 | 結果 |
|---|---|
| InitCount の変化 | **943 回**。値は 1, 2, …, 942 と +1 ずつ連続（飛びなし・戻りなし）、最後に 0。worker の `gdb-wc-final.txt`（`COUNT` 行 943 個）と同じ |
| 書き手 | C スタック（bt）を取ったのは 1, 2, 3, 941, 942 回目と最後の 0 の 6 か所。1〜942 の 5 か所はいずれも `fle_st` ← `f_4120c0`（InitUnits）← `f_412160` ← `f_41d8f0` ← `f_1376840` ← `f_1376820` ← `main`。他の回の書き手は bt を取っていない（ただし各 unit の proc の最初の呼び出し時の InitCount が全 895 件で index+1 であること（下）は、直前に `f_4120c0` が増やしたことと整合）。最後の 0 への書き込みは **`f_4128b0`** の中（`f_4120c0` ではない） |
| InitContext+0 | 2 回だけ変化: 0 → **0x13768b0**（`f_412160`〔`f_41d8f0` から呼ばれる〕の中。`f_4120c0` が始まる前）、**0x13768b0 → 0**（`f_412770 ← f_4128b0 ← f_1376840 ← f_1376820 ← main`）。worker の記録と同じスタック |
| unit proc の呼び出し | runnable **895 個すべて**が呼ばれた（**895 / 895**）。**各 proc ちょうど 1 回**（合計 895 回）。最初の呼び出し時の InitCount は**全 895 件で index+1**。呼び出し元は全 895 件で `fle_dispatch ← f_4120c0 ← f_412160`。呼び出し順は unit の index 順 |
| 戻ったか | `f_4120c0` は 1 回だけ呼ばれ、InitCount が +1 されるのは直前の unit の呼び出しから戻った後（逐次ループ。FLEngine の setjmp 着地は終了時の 1 回だけ。QFC と ILTools の longjmp は unit 158／245 の中の `LoadLibraryW` が呼ぶ各 DllMain の内側で完結し、`f_4120c0` のフレームを飛び越えない＝読み）。よって 895 の proc は**すべて戻った**（読み＋実測の組合せ。proc ごとの return は直接は観測していない） |
| finalizer | 896 個（各 unit の +8 のうちコードのもの。残り 1 つは 46 unit が共有するデータ上のゼロブロック 0x13a9390 でシンボルが無い）に breakpoint → **0 回**。ユニットの finalizer は 1 つも呼ばれていない |
| 終了処理の順序（seq はイベント通し番号） | `f_4128b0` 入り（InitCount=942）→ **InitCount = 0**（`f_4128b0` 内）→ `f_412020`（入口が `InitCount` を下げながら FInit を呼ぶループ＝FinalizeUnits 相当。呼ばれた時の InitCount は 0 なのでループは回らない）→ `f_412770`（InitContext を外側の文脈に戻す。InitContext+0 = 0）→ `f_4126f0`（ゲストの longjmp）→ 着地（`f_1376840` の 2 回目）→ 戻る |
| longjmp | 3 回: QFC の `f_1d0f9c0`（InitCount=159）、ILTools の `f_600e090`（InitCount=246）、FLEngine の `f_4126f0`（InitCount=0）。**worker の「3 回、QFC・ILTools・FLEngine」と一致**。各々 DllMain 相当の setjmp（`f_1e9c1f0`、`f_62b35f0`、`f_1376820`）へ戻る。setjmp（`f_412680`、FLEngine）は 1 回 |
| entry の戻り値 | `trace_report("returned from entry")` の時点の `R[0]`（rax）= **0x1**（gdb が `R` のメモリを nm のオフセット経由で直接読んだ値。ハーネスの出力 `rax=0x1` とも一致）＝ DllMain が TRUE を返した。プロセスの終了コード 0 |
| 呼び出し深さ 12 | 実測は harness の報告のみ。内訳は読み: 3 回の longjmp が捨てた lifted 関数のフレームが各 4（QFC: `f_1d0f9c0`、`f_1d0fa40`、`f_1d0fb80`、`f_1e9c210`／ILTools: `f_600e090`、`f_600e110`、`f_600e250`、`f_62b3610`／FLEngine: `f_4126f0`、`f_412770`、`f_4128b0`、`f_1376840` の 1 回目）で 4+4+4 = 12 と一致（worker の「longjmp で捨てられたフレームの残り」と整合）。深さそのものを実行時に読む確認はしていない |

**(c)「Halt0 が InitContext を 0 に戻す」の確認**: 0 に戻したのは確かに終了処理（`f_4128b0` ＝ Halt0 相当、`f_412770`）で、a4 の 942 は実際に起きた値（`f_4120c0` が書いた）。ただし**説明は 2 点正確でない**（§8）。

追加の観測（実測・読み）: `FLEngine_x64.dll` の文字列に `Embarcadero Delphi for Win64 compiler version 36.0 (29.0.53982.0329)` がある（`strings -a`）。`f_4120c0` は `InitContext.InitTable`／`InitCount` を使う Delphi RTL の InitUnits と同じ形（except 節に FinalizeUnits が続く）。バイナリに `FPC` を含む文字列は 1 つだけ（`FPC does not support monolithic mode with FLAC compression…`。何の略かは未確認で、コンパイラの証拠ではない）。**読み**: FLEngine は Embarcadero Delphi 製で、worker が書く「FreePascal」ではない。FinalizeUnits 相当 `f_412020` は InitCount を 1 ずつ下げて `FInit` を呼ぶ形だが、呼ばれた時点で InitCount は 0（実測）。

## 6. 回帰（実測）

| 構成 | 結果 | 比較 |
|---|---|---|
| **FLEngine 単独**（`a4-multi.sh` を引数なし＝0 モジュールで。註釈なし。`EXPECT_FLE_SHA` は既定の `f6d3a971…` で検査を通った） | **`init_count=145 of 942`**、last unit index=144 proc=0x65c540 `in_progress`、started=132 completed=131、停止 `fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; …)`、**functions 822、entries 15,686**、call depth 10、終了コード 134 | 決定 0101 の 145、`docs/progress.md` の「822 / 15,686（統合前は 821 / 15,685）、depth 10」と**一致**。functions／entries は E1 などで +1 ずつ動いた値 |
| **M0**（6 モジュール、VFS 種付けなし。run 1 と同じバイナリを `tools/survey-direct.sh` で） | **`init_count=246 of 942`**、last unit index=245 proc=0x733090 `in_progress`、started=209 completed=208、停止 `fle: StretchDIBits: pixel transfer from a DIB (StretchDIBits rop 0xcc0020, iUsage 0, 1 x 1 to (0, 0) from 1 x 1 at (0, 0)): the bitmap of hdc is a device-dependent bitmap (or the 1-by-1 placeholder): it keeps no pixels (decision 0106) (decision 0111)`、functions 3,411、entries 134,085、depth 32、終了コード 134（SIGABRT） | worker の M0 と**完全に一致**。完走は VFS 種付け（決定 0110）に依存する |

M0 は M1 と同じバイナリを使った（種付けは実行時の入力で、ビルドの一部ではない。`a4-multi.sh` の仕様）。M0 だけを別 OUT で一からビルドしてはいない。

## 7. この完走の性質（決定 0101 の表記のため）

(a) **仮の応答の数 N = 46**。式（索引の表の行だけを数える）:

```
grep -E '^\| 0[0-9][0-9][0-9] \|' docs/README.md | grep -c '\*\*仮'        # = 46   （索引の行は 115）
```

最後の列（「仮」）が `**仮**` で始まる行を awk で数えても 46。`grep -c '\*\*仮' docs/README.md`（ファイル全体）は 51 で、索引以外の 5 行を含むため使わない。該当する決定: 0040 0041 0042 0048 0053 0054 0055 0056 0057 0059 0061 0062 0063 0065 0066 0067 0069 0070 0071 0072 0078 0081 0082 0083 0084 0085 0087 0088 0092 0096 0098 0099 0100 0102 0103 0104 0105 0106 0107 0108 0109 0110 0111 0112 0113 0114（0112〜0114 を含む）。N は**決定の数**で、応答の数でも、この完走で実際に使われた仮の決定の数でもない（どの仮の応答が使われたかの対応づけは**していない**）。

(b) **実際に呼ばれた応答関数**（gdb で全 `fle_imp_*` 435 シンボルにカウント用 breakpoint。`gdb/v1apis.py`、`logs/gdb-apis-table.txt`）: **171 種・13,572 回**。名前付き停止に当たる呼び出しは無い（走り切ったため）。回数の多い順の上位: `WideCharToMultiByte` 3,011、`GetCurrentThreadId` 1,684、`GetProcAddress` 1,612、`EnterCriticalSection`／`LeaveCriticalSection` 各 681、`MultiByteToWideChar` 578、`TlsGetValue` 513、`SelectObject` 408、`GetLastError` 270、`GetLocaleInfoW` 245、`RegOpenKeyExW` 186、`InitializeCriticalSectionAndSpinCount` 179、`LoadStringW` 162。決定 0113 の壁の API: `HeapReAlloc` 9、`GetObjectW` 44、`StretchDIBits` 34、`SelectPalette` 34、`DestroyCursor` 30、`LoadLibraryW` 34。E1（決定 0112）: `GetComputerNameExW` 2、`GetComputerNameW`／`GetFileVersionInfoW`／`VerQueryValueW`／`LocaleNameToLCID`／`GetUserNameW` は 0（worker の記録と一致）。ファイル系: `CreateFileW` 3、`WriteFile` 2（種付けの 2 ファイル）、`ReadFile` 3、`SetFilePointer` 6、`FindFirstFileW` 12、`GetFileAttributesW` 136。**どの呼び出しが仮の決定に属するかの対応づけはしていない**。

(c) **完走に必要な実在モジュールの集合**: FLEngine ＋ 5（`dsp_ippv2_x64.dll`、`blend2d.dll`、`quickfontcache_x64.dll`、`freetype_x64.dll`、`iltools_x64.dll`）＝6 モジュール。VFS 種付け: `ILCursors.ilfont`（7,892 B）と `ILGlyphsEx.ilfont`（102,712 B）の 2 ファイルだけ（`C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\`）。**実測**: 単独は dsp で、M0 は種付けなしで 246 で止まる。**未確認**: 5 モジュールの**各々が個別に必要か**（1 つずつ外した測定はしていない）、他の同梱 DLL（決定 0101 の 10 名）が InitUnits の外か中か。dsp は全体の lift ではなく種 452 個の縮小版（生成 C は 8,479 関数）。

(d) **測定用の値**（すべて「決定ではない測定用の値」。コミットされた木の外にあるファイルは `lift/out-*`（gitignore）から読んだ）:

| モジュール | 基底 | 種 | 註釈（hook points） | その他 | 入力 DLL の SHA-256 |
|---|---|---|---|---|---|
| FLEngine | 0x400000（元のまま） | — | `harness/hooks/flengine-hook-points.txt`（13 行、`e9458b26…`） | `--pdata`。生成 C の SHA `e047beeb…`（註釈あり） | `b2c64da4…f9496d` |
| dsp_ippv2_x64.dll | **0x2000000** | `lift/out-three/three/dll/seeds-idx0-plus-initterm2.txt`（452 行、`cf664439…`） | — | `--pdata` なし | `497dd4f7…691c352` |
| blend2d.dll | **0x4700000** | `lift/out-seed58/seeds/seeds58-4700000.txt`（58 行、`ca6439b6…`） | — | `--pdata` | `66634d24…66066c7` |
| quickfontcache_x64.dll | **0x1d00000** | なし | `harness/hooks/qfc-hook-points.txt`（16 行、`2b97de2e…`） | `--pdata` | `afc5a0ed…1047736d2` |
| freetype_x64.dll | **0x4b00000** | `harness/seeds/ft-seeds-4b00000-c1.txt`（170 行、`d3487fd2…`。旧 160 ＋ 10） | — | `--pdata` | `6ea6c106…3837cb8` |
| iltools_x64.dll | **0x6000000** | `harness/seeds/iltools-6000000-seeds400.txt`（400 行、`b696d775…`） | `harness/hooks/iltools-hook-points.txt`（21 行、`fb21e57c…`） | `--pdata` | `20409dcb…061a0f95` |

VFS 種付けリスト `harness/seeds/vfs-seed-fonts.tsv`（SHA `127a4471…`）、`VFS_SEED_ROOT`＝リポジトリのルート。ほかに `JOBS=8`、`FLE_PER_PART=3700`、`REAL_PER_PART=800`（既定）。

## 8. worker の報告との一致／不一致

| 項目 | worker（決定 0113、MERGE-NOTES） | V1 | 判定 |
|---|---|---|---|
| a4 の行、stop、functions／entries、終了コード | 942/942、895/895、`returned from entry`、5,306／1,007,942、0 | 2 回とも同一 | **一致** |
| native の SHA-256（独立 2 回のビルド） | `9c76bc6a…` | 2 回とも同じ | **一致** |
| FLEngine の生成 C（註釈なし／あり）、390、59,380、0 | 一致 | 一致（dev・release） | **一致** |
| M0 | 246/942、StretchDIBits、3,411／134,085、depth 32、134 | 同じ | **一致** |
| 単独 | （回帰として 145/942） | 145/942、822／15,686 | **一致** |
| InitCount 1…942 → 0（gdb 943 回） | 一致 | 一致 | **一致** |
| longjmp 3 回（QFC、ILTools、FLEngine） | 一致 | 一致（InitCount 159、246、終了時） | **一致** |
| E1 の API の呼び出し | GetComputerNameExW 2、他 0 | 同じ | **一致** |
| N | HEAD で 43、+3 = 46 | 46 | **一致** |
| 「完走後に **FreePascal** の `Halt0` がユニットを **finalize** し、InitContext+0 と InitCount を 0 に戻す」（決定 0113 T1、`a4.c` のコメントと `a4: note:` の文言「the program's exit path finalised the units」） | — | **不一致（説明のみ。測定値には影響しない）**: (1) コンパイラは **Embarcadero Delphi**（バイナリ内の文字列）で FreePascal ではない（読み: 決定 0113 が「FreePascal」と書く根拠は示されていない）。(2) **ユニットは finalize されていない**: InitCount は 1 回の書き込みで 0 にされ（942 → 941 → … の下降は無い）、FinalizeUnits 相当は InitCount=0 で呼ばれてループが回らず、896 個の finalizer は 1 つも呼ばれない。InitContext+0 を 0 にするのは `f_412770` による外側の文脈の復元。DllMain の正常な戻り（rax=1）の一部と読める（Delphi RTL の library の DLL_PROCESS_ATTACH の終わり方。**Delphi の RTL ソースとは照合していない**） | 説明が不正確 |
| 「`a4` の `init_count=942`」 | a4 の報告 | 報告時点のゲストメモリの値は 0。942 は**サンプルした最大値**。gdb の独立観測と値が一致するので正しいが、a4 単体の読みではない | 注記 |
| 「全体テスト 115／1,575 passed／0 failed」「変異検査 12 件」 | — | **実行していない** | **未確認** |
| 「`a4-multi.sh` の複製で測定」 | 複製（`a4-multi-c1.sh`） | コミット済みの `a4-multi.sh` で同じ結果 | 差は結果に影響しない（§1） |

食い違いは説明の 2 点（FreePascal／finalize）だけで、完走の数値・SHA・回帰の値はすべて再現した。

## 9. 規則違反の自己申告

- **標準ライブラリと衝突する名前のファイルを作った**: 逆アセンブル用に `tools/dis.py` を作り、1 回実行したところ `capstone` が標準の `dis` を import する際に自分のファイルが読まれて ImportError のトレースバックが出た（逆アセンブルの出力自体は出た）。直ちに `v1dis.py` に改名して以降使った（成果物は `v1dis.py`）。
- **バックグラウンド実行を 1 回使った**: run 1（o1 のビルドと a4。約 1 分）を shell ツールの background で起動し、完了通知を待った。待つ間は読み取り・逆アセンブル・`du`／`stderr` の確認だけで、完了確認の繰り返しはしていない。それ以外はすべて前景。
- 存在しない `bash` ツールを 1 回呼んで失敗（副作用なし）。`FLENGINE_PE` を export し忘れて dev の `fe-lift` を 1 回空引数で走らせた（rc=1、出力ファイルなし）。`survey-direct.sh` の初版に `OUT` の代入順の誤りがあり 1 回失敗した（`o1/runs/m0run` を作り直した）。
- Python: ほとんどに `PYTHONDONTWRITEBYTECODE=1` を付けたが、確認用の `python3 -c "import capstone"`、`python3 -c "import pefile"`、算術の `python3 -c "print(hex(...))"` では付け忘れた。`find /tmp/opencode/v1 -name __pycache__` は 0 件（site-packages 側は確認していない）。gdb 内蔵の Python は `-x` のスクリプト実行で、pyc は作られていない（読み）。
- 削除は `/tmp/opencode/v1/` の自分のものだけ（cargo の dev target、生成 C、OUT の objects／native／target、gz の控え）。`/tmp/opencode` の他の作業者のディレクトリは削除していない（PM が許可した削除も不要だった）。
- gdb を native に付けた（`ulimit -c 0`、読み取りとブレークポイント・ウォッチポイントだけ）。ホストの実ファイル・レジストリをゲストに見せていない。依頼者の資格情報・ブラウザには触れていない。
- sleep／timeout／nohup／末尾の `&` は使っていない（shell ツールの `timeout` パラメータと bash の `time` は使用）。emcc／wasm は使っていない。`lift/out-cs/` を cwd にしていない。他の作業の `CARGO_TARGET_DIR` を指定していない。
- AGENTS.md の「作業は task agent を通じて」は、私がサブエージェントで task ツールを持たないため、自分で作業した。
- `/tmp/opencode` の使用量は最大 2,081 MB（観測）で、2.3 GB は超えていない。

## 10. 限界・確認できなかったこと

- **実機 Windows との照合は 0 件**。**wasm は未確認**（emcc 不使用。決定 0046 の A4 は native と wasm の両方を求める）。
- 完走は測定用の基底・種・註釈と仮の応答（N = 46 の決定）を含む構成での値。決定 0101 の表記では「A4 完走（仮の応答 46 件を含む）」であり、「A4 達成」ではない（判定は PM）。
- 完走した**後**（InitUnits の後のエンジンの動作）は測っていない。unit の中身が実機と同じ動作をしたかは言えない（動作の確認ではなく、停止せずに最後まで走ったことの確認）。
- テスト一式・変異検査は再実行していない。個々の応答（S1〜S13 の出典なしの選択）の正しさは検証していない。
- 5 モジュールが個別に必要かの切り分け、どの仮の決定が実行で使われたかの対応づけ、`depth 12` の実行時の読み（読みのみ）、Delphi RTL のソースとの照合、VFS 種付けの 2 ファイルとインストーラの一致（インストーラが無い）は**未確認**。
- 単独 a4 は註釈なしのみ（註釈あり・なしの両方を測る必要はないと判断。progress.md は両方同一と書く。未確認）。
- 入力 DLL・種の一部（`lift/out-*`）はコミットされていない（gitignore）ので、コミットだけでは再現できない。このセッションでは worker と同じファイル（TAG が一致）が作業ツリーにあった。

## 11. 成果物

`lift/out-git/v1-results/`（コピー。`/tmp/opencode/v1/final/` に同じもの）:

- `verification.md`（本書）
- `logs/`: `cargo-build-*.txt`、`fle-lift-*.out|err`、`fle-*.sha`、`r1/`・`r2/`（a4-multi の stdout／stderr、gen、build、survey、native のログ、native の SHA）、`m0/`、`single/`、`det/`、`pe-units.tsv`、`gdb1/`（`analysis.txt`、`result.json`、stderr）、`gdb1.txt`、`gdb-apis/`、`gdb-apis-table.txt`、`g-flag-check.txt`
- `tools/`: `run6.sh`、`survey-direct.sh`、`wrap/clang`、`wrap/python3`、`pe_units.py`、`v1dis.py`、`analyze_gdb.py`
- `gdb/`: `v1watch.py`、`v1apis.py`

巨大なもの（生成 C、バイナリ、オブジェクト）は置いていない。
