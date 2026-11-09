# 表 0x13768b0 の閉包外 895 件を extra seed にした計測（bulk-table-895）

対象: `binaries/win64/FLEngine_x64.dll`。作業場所: `lift/out-bulk/`（新規）。
本番の既定の生成 C、既定の seed、既存の API、`out-final/`、`out-seed-final/`、`out-seed-both/`、`out-idx/` は変えていない。
本ドキュメントの数字は、すべて `lift/out-bulk/` の出力からの実測。推測は「推測」と明記する。

## 1. 材料

- 表: 構造体 0x13768b0（件数 942、表の先頭 0x13768e0）。`lift/out-idx/table-classify.txt` から、`region=text in_lifted_table=0` の 895 件を取り出した。
- 一覧: `lift/out-bulk/seeds-table-895.txt`（895 行、重複 0、0x41d050 は含まない）。
- 既存の extra seed: 0x41d050（表には無い）、0x41e7e0（表の添字 0。895 件に含まれる）。
- 比較の基準（既存）: `lift/out-seed-both/`（seed 2 件）。閉包 6,972 関数 / 460,495 命令、`fle_unsupported` 1,561 件。
- 対象 seed 集合は 2 通り。
  - (A) 895 件のみ。
  - (B) 895 件 + 0x41d050 = 896 件（既存の seed 2 件を含む和集合）。既存の基準と比べるための本命はこちら。

## 2. 閉包の数字（実測。`lift/out-bulk/tool`、`bulk-table closure`）

| 項目 | 既存（seed 2） | (A) 895 のみ | (B) 896（895 + 0x41d050） |
|---|---|---|---|
| 関数 | 6,972 | 8,425 | 8,427 |
| 命令 | 460,495 | 497,354 | 497,367 |
| 増分（対 既存） | — | 関数 +1,453 / 命令 +36,859（※0x41d050 を含まないので、既存の上位集合ではない） | 関数 +1,455 / 命令 +36,872 |
| デコード失敗 | 0 | 0 | 0 |

- (B) の増分は、既存の 6,972 関数と 460,495 命令を含む和集合に対する差。
- 注: (A) は既存の seed のうち 0x41d050 を含まないため、既存の閉包の上位集合ではない。(A) の数字は参考。

## 3. 生成の件数（実測。`bulk-table lift`、出力は `lift/out-bulk/out/`）

| 項目 | 既存（out-seed-both） | (A) 895 のみ | (B) 896 |
|---|---|---|---|
| closure_instructions | 460,495 | 497,354 | 497,367 |
| functions | 6,972 | 8,425 | 8,427 |
| emitted_instructions | 460,575 | 497,514 | 497,527 |
| bad_nodes | 0 | 0 | 0 |
| gotos | 42,816 | 45,808 | 45,809 |
| c_lines | 502,421 | 548,078 | 548,103 |
| c_bytes | 39,177,903 | 42,414,253 | 42,415,522 |
| unsupported_total（fle_unsupported の件数） | 1,561 | 1,622 | 1,622 |
| 増分（fle_unsupported、対 既存） | — | +61 | +61 |
| call（handled） | 38,953 | 43,021 | 43,024 |
| call_ind:dispatch | 6,478 | 6,887 | 6,887 |
| call_ind:rip_table | 12 | 13 | 13 |
| call_ind:reg_table | 0 | 1 | 1 |

- fle_unsupported の増分 +61 の総数は実測（1,561 → 1,622）。内訳は次のとおり（`gen-bulk-table-896.txt` と `gen-report.txt` の一覧から読んだ値）。
  - 実測で確認した行: `Jmp: indirect through IAT slot` 400→435（+35）、`Cpuid` 0→5（+5）、`Jmp: indirect target unresolved` 16→20（+4）、`Imul` 129→132（+3）、`Cmpxchg` 27→30（+3）。
  - 未検証: この他の行の増減（`Div`、`Idiv`、`Cqo`、`Movsx`、`Bts`、`Fninit`、`Fnstcw`、`Fnstsw`、`Xgetbv` など）は、比較用の並べ替えに失敗して結合結果が不正確だったため、本計測では確定していない。上の 5 行の合計は +50 で、残りの +11 の内訳は未確認。
- 応答の呼び出し（call_ind:dispatch）は +409 件（6,478 → 6,887）。

## 4. clang とリンク（実測）

- ビルド: `lift/out-bulk/build.sh`（既存の `lift/out-seed-both/build.sh` と同じフラグ）。
  - `sh build.sh after-bulk-table-896`: 成功（終了コード 0）。生成物 `flengine-after-bulk-table-896`（44,755,856 B）、`after-bulk-table-896.o`（96,338,424 B）。ログ `build-896.txt`。
  - `sh build.sh after-bulk-table`（895）: 成功（終了コード 0）。生成物 `flengine-after-bulk-table`（44,755,008 B）、`after-bulk-table.o`（96,335,240 B）。ログ `build-895.txt`。
- 注: `build.sh` に実行権限が無かった（`./build.sh` は Permission denied、終了コード 126）。権限は変えず、`sh build.sh` で実行した。
- 2 GB 級の中間ファイルは作っていない（最大 96 MB の .o）。

## 5. 実行と停止点（実測。引数に DLL のパスを渡す）

### 5.1 (B) 896 の run（`run-896.txt`）

- 停止: SIGABRT（`fle_unsupported`）。停止 VA `0x40f47b`。関数 `f_40f470`（`cpuid`。`lift/out-bulk/out/after-bulk-table-896.c` の L_40f47b は `fle_unsupported(0x40f47b)` で、`Cpuid: mnemonic outside minimal set`）。
- 関数実行数: 22（distinct）、関数入口 22 回。
- 呼び出し深さ: 8。
  - #0 f_40f470 ← #1 f_40f490 ← #2 f_40f4b0 ← #3 f_41ce10 ← #4 f_4120c0 ← #5 f_412160 ← #6 f_41d8f0 ← #7 f_1376820。
- 表の要素の進み（実測）: 添字 0（0x41e7e0。閉包外の候補）の入口は入口ログ 17 番目。添字 1（0x41ce10。同じく閉包外の候補）の入口は 18 番目で、その戻り先が 0x412129（ループの継続点）。このことから、0x41e7e0 は返ったと見られる（推測。入口ログの順序からの推定で、戻りは直接は記録していない）。添字 1 の中で停止したので、添字 2 以降には届いていない。
- 閉包外の関数を越えたか: 元の閉包（seed 2）では 0x41e7e0 と 0x41ce10 は閉包外だった。(B) では両者を seed に入れたため閉包内になっている。この run は、両方の関数に入り、その子（0x40f4b0、0x40f490、0x40f470）まで進んで、cpuid で止まった。すなわち、閉包外の関数を越えて進んだ（ただし、閉包を広げた上での話）。
- dispatch miss は 0 件。停止は閉包内の関数で、未対応命令（cpuid）による。

### 5.2 (A) 895 の run（`run-895.txt`）

- 停止: SIGABRT（`fle_dispatch_miss`）。停止 VA は呼び出し先 `0x41d050`（間接 call）。
- 関数実行数: 7（distinct）、入口 7 回。呼び出し深さ 3（f_412160 ← f_41d8f0 ← f_1376820）。
- 停止の経路（実測）: 0x41d050 は表の要素ではなく、呼び出し元は f_412160（表のループ 0x412126 ではない）。入口ログでは、0x41d050 の入口は (B) の run にだけ見え、(A) では間接 call の宛先として dispatch miss になった。
- 解釈（実測の範囲）: 895 件だけでは、既存の 2 seed の閉包（0x41d050 を含む）を覆えない。(A) は既存の閉包の代わりにはならない。

### 5.3 閉包外の関数の実行（問い 5 への回答）

- (B) では、閉包外の候補のうち添字 0（0x41e7e0）と添字 1（0x41ce10）を越えて、その子まで進んだ（5.1 のとおり）。添字 2 以降には届いていない。
- (A) では、0x41d050 の dispatch miss で止まり、表の要素（0x412126）には届いていない。
- 閉包外の要素の添字 2 以降（0x420d80、0x421910 など）は、どちらの run でも呼ばれていない（実測）。

## 6. 推測と未確認（明記）

- 推測: 表の残りの要素のうち、0x40f47b の `cpuid` を含む関数群が、本番の DLL の実行で実際に呼ばれるかは、この計測では判断できない。
- 未確認: 添字 2〜941 の各要素の実行結果。表の 47 件（data 宛 46、0 の 1）は seed に入れていない。0x412126 で data 宛・0 の宛先が呼ばれたときの挙動は、この計測では未確認（既存の `index-0x412104.md` の注と同じ）。
- 未確認: `cpuid` の実装（生成器は未対応のまま）。`Jmp: indirect through IAT slot (not lifted)` が 435 件あり、表の要素から先の実行で、これが止まる要因になり得る（推測）。
- 未確認: 0x41d050 の表 0x13a9410 との関係は、本件の対象外。

## 7. 残り（作業量）

- 表の残り 941 − 2 = 939 要素のうち、添字 2 以降は未実行。
- 停止点 `cpuid`（0x40f47b）の扱いと、未対応命令の増分 61 件は、本計測の範囲外（次の判断は PM）。
- 本計測では、閉包の定義・既定の seed・既存の生成 C は変えていない。

## 8. 操作の記録

- 新規作成: `lift/out-bulk/`（`Cargo.toml`、`tool/`、`build.sh`、`seeds-table-895.txt`、`closure-*.txt`、`gen-bulk-table*.txt`、`out/`、`run-895.txt`、`run-896.txt`、`build-*.txt`、`tmp/`、`target/`）と、本ファイル。
- 既存のファイルの変更: なし。`rm`、上書き、git、/tmp、sleep や timeout は使っていない（`TMPDIR` は `lift/out-bulk/tmp`）。
- 権限拒否: なし。`build.sh` の実行権限が無かったのは、拒否ではなく作成時の状態。`sh build.sh` で実行した。
