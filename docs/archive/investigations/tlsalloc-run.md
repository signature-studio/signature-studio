# TlsAlloc の計測実行（0x41cf40）

作業者による実行記録。推測と実測を分けて書く。

## 1. 追記した関数（実測ではなく実装の内容）

`lift/runtime/fle_imports.c` と `fle_imports.h` に `fle_imp_TlsAlloc` を追記した。既存の `fle_imp_GetCurrentThreadId` は変えていない。

- 判断記録 `docs/decisions/0002-tlsalloc.md` の案 A どおり。
  - 呼ばれるたびに単調増加の索引を返す（`__atomic_fetch_add`、開始値 0）。
  - 上限 1,088 個。超過時は `0xFFFFFFFF` を `R[0]` に返し、加算を巻き戻す。
  - 引数は読まない。
- 上限 1,088 と `0xFFFFFFFF` は判断記録の値。`0xFFFFFFFF` の出典は未確認（判断記録どおり仮定）。
- TlsFree は未実装。解放された索引の再利用はない。

## 2. 生成とリンク（実測）

- 入力: `lift/out-seed41d/out/plus41d.c`（読み取りのみ）。
- 書き換え: `L_41cf40` を `fle_imp_TlsAlloc()` に置換（sed 1 行）。`L_406f90` の GetCurrentThreadId 置換は従来どおり。
- 出力: `lift/out-tlsalloc/out/plus41d-tls.c`（新規）、ビルド手順 `lift/out-tlsalloc/build.sh`。
- ビルド: 成功。`lift/out-tlsalloc/flengine_tls` を生成。
- sed 置換の検証（grep -c が 1）も通過。

## 3. 実行の停止点（実測。`lift/out-tlsalloc/run1.txt`）

- 停止点 VA: `0x41cf20`
- 関数: `f_41cf20`
- 命令: `jmp [0x19717c8]`（objdump で確認）
- IAT の名前: `kernel32.dll!LocalAlloc`（`lift/out-imports/imports.txt` の 0x19717c8）
- abort の種類: `fle_unsupported`（SIGABRT、終了コード 134）
- 関数実行数: 13（distinct 13、entries 13）
- 呼び出し深さ: 7

呼び出しスタック（実測）:

```
#0 0x41cf20 f_41cf20 guest-return=0x41cfe2
#1 0x41cfd0 f_41cfd0 guest-return=0x41d02a
#2 0x41d000 f_41d000 guest-return=0x41d071
#3 0x41d050 f_41d050 guest-return=0x412241
#4 0x412160 f_412160 guest-return=0x41d989
#5 0x41d8f0 f_41d8f0 guest-return=0x1376875
#6 0x1376820 f_1376820 guest-return=0xdead0000
```

## 4. 0x41cf40 を越えたか（実測）

越えた。trace の entry log で以下が確認できた。

- entry 8: `f_41cf40`（guest-return 0x41d066）。TlsAlloc 相当の stub が返った。
- entry 9: `f_41d000`（guest-return 0x41d071）。0x41d066 の後の経路（`mov [0x13a9388], eax` の後の `call 0x41d000`）に進んだ。
- entry 10: `f_41cff0`、entry 11: `f_41cfd0`、entry 12: `f_41cf20`。

0x41cf40 の stub の戻りは 0x41d066 で、run1 の前回（seed41d）と同じ戻り番地。

## 5. 次の停止点（実測と objdump）

- 0x41cf20 の `jmp [0x19717c8]`。IAT 名は `kernel32.dll!LocalAlloc`。
- 呼び出し元は 0x41cfd0（`call 0x41cf20`）。0x41cfd0 は ecx=ebx（=0x32a8、0x41cff0 の戻り値）、edx=ebx、ecx=0x40 に整えてから 0x41cf20 を呼ぶ。つまり LocalAlloc(uFlags=0x40, uBytes=0x32a8) に見える（引数の順は objdump からの読み。推測を含む）。
- 0x41cfd0 は 0x41d000 の `call 0x41cfd0`（0x41d025）から呼ばれる。0x41d000 の処理（objdump）: 0x41cff0 の戻り値（`lea rax, [0x32a8]`、定数）を ebx に入れ、0 でなければ続行。`cmp [0x13a9388], -1` で -1 の場合は 0x412a60 を呼ぶ。その後 0x41cfd0 を呼び、戻り値（rax）が NULL なら 0x412a60（ecx=0xe2）を呼ぶ。非 NULL なら 0x41cf70 を呼ぶ（ecx=[0x13a9388]=TlsAlloc の索引、rdx=rax。TlsSetValue 相当に見える）。
- 推測: 0x41d000 は「TLS 索引に紐づく領域を確保して設定する」処理に見える。0xe2 はエラー記録の値に見える。0x412a60 の中身は未読のため未確認。

## 6. 残りの作業量（推測）

- 0x41cf20 LocalAlloc の応答（判断は PM）。
- 0x41cf70 TlsSetValue、0x41cf60 TlsGetValue、0x412a60 の中身（いずれも未対応・未読）。
- 0x41cf20 の LocalAlloc の応答（戻り値の値の範囲、失敗時の扱い）は仕様未確認のまま。作業者は繰り返していない（依頼どおり。次の判断は PM）。

## 7. 失敗・拒否された操作

- 権限拒否は無し。
- git 操作は行っていない。rm や既存ファイルの上書きも行っていない（新規ファイルのみ作成）。
- `/tmp` は使っていない。TMPDIR は `lift/out-tlsalloc/tmp` に向けた。
- sleep/timeout は使っていない。

## 8. 検証の範囲

- ビルドと実行は実測。
- 0x41cfd0 が LocalAlloc の呼び出しという読みは、IAT の jmp 先（0x41cf20 → LocalAlloc）と objdump からの推測。
- TlsAlloc の戻り値が以後どう使われたか（0x41cff0 の値が索引として正しいか）は未確認。
