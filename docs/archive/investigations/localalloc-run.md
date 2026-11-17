# localalloc-run: LocalAlloc 応答を入れた実行の結果

作業者（PM）の実行。出力は `lift/out-localalloc/run1.txt`。

## 生成とリンク

- 入力: `lift/out-tlsalloc/out/plus41d-tls.c`（読み取りのみ）。
- 置換: `sed` で `L_41cf20` の行を `fle_imp_LocalAlloc()` に置換。`grep -c 'fle_imp_LocalAlloc'` = 1 を確認（build.sh の test）。
- 出力: `lift/out-localalloc/out/plus41d-ll.c`、`lift/out-localalloc/flengine_ll`。
- 結果: 成功（警告なし）。TMPDIR は `lift/out-localalloc/tmp`。

## 実行結果（実測。`lift/out-localalloc/run1.txt`）

- 終了コード: 134（SIGABRT）
- 停止点: 0x41cf70（`jmp [0x19717a8]`）
- 関数名: f_41cf70。IAT 0x19717a8 = kernel32!TlsSetValue（`imports.txt` 442 行）
- abort の種類: `fle_unsupported`（`fle: unsupported instruction at 0x41cf70`）
- 関数実行数（distinct）: 14
- 関数実行回数（calls）: 14
- 呼び出し深さ（stop 時）: 6
- 呼び出し経路（shadow stack）:
  - #0 0x41cf70 f_41cf70（戻り番地 0x41d049）
  - #1 0x41d000 f_41d000（戻り番地 0x41d071）
  - #2 0x41d050 f_41d050（戻り番地 0x412241）
  - #3 0x412160（戻り番地 0x41d989）
  - #4 0x41d8f0（戻り番地 0x1376875）
  - #5 0x1376820（戻り番地 0xdead0000）
- entry log の 13 番目が 0x41cf20（戻り番地 0x41cfe2）、14 番目が 0x41cf70（戻り番地 0x41d049）。

## 判定

- 0x41cf20（LocalAlloc）を越えた。LocalAlloc の戻り値は 0x41d02a の `test %rax,%rax` を通り、0x41d03b の TlsSetValue 呼び出しに到達した。
- 0x41d000 の経路は 0x41d049 に戻る予定（TlsSetValue 呼び出し後の終了）。今回は TlsSetValue の jmp で止まった。

## 次の停止点（objdump で確認）

- VA: 0x41cf70
- 命令: `41cf70: jmp *0x1554832(%rip) # 0x19717a8`
- IAT の名前: kernel32!TlsSetValue（`lift/out-imports/imports.txt` 442 行）

## 残りの作業量

- この実行で確認できたのは、LocalAlloc の 1 件の応答と、その次の停止点（TlsSetValue）まで。
- 同じ経路の残り: TlsSetValue（0x41cf70）、TlsGetValue（0x41cf60）、GetLastError（0x41cf80）は未対応。依頼に従い、次の停止点は繰り返さない（TlsSetValue は未実装）。
- 本件の実行で LocalAlloc が NULL を返したかは、ログに出力がないため確認していない（0x41d02a の分岐は通らなかったと推定。TlsSetValue に進んでいるので rax != 0 は実行で確認済み）。
- 全体の残件数は見積もれない（既存メモの 1,575 件は未分類。import 以外の fle_unsupported も含む）。

## 推測と実測の区別

- 実測: 停止点の VA、関数名、abort の種類、関数実行数（14）、呼び出し深さ（6）、TlsSetValue への到達。
- 推測: 0x41d000 の -1 比較の意味、0x412a60 の役割（未読）。
