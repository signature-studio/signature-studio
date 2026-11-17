# tls-run: TlsSetValue、TlsGetValue、GetLastError の応答を加えた実行結果

作業者（PM）の実行。実行は 2 回（run1、run2）。run2 は run1 の後にコメントだけ直した最終版で、再ビルドして実行した。

## 生成とリンク

- 入力（読み取りのみ）: `lift/out-localalloc/out/plus41d-ll.c`
- 複製: `lift/out-tls2/out/plus41d-tls2.c`（`lift/out-tls2/build.sh` の sed で作成）
- 置換の回数（grep -c の差）:
  - `fle_unsupported(` の行: 1574 → 1571（差 3）
  - `fle_imp_` の行: 6（LocalAlloc、TlsAlloc、GetCurrentThreadId の既存 3 件と、TlsGetValue、TlsSetValue、GetLastError の新規 3 件）
  - build.sh 内の test で、3 件それぞれが 1 行であることを確認
- ビルド: 成功。`lift/out-tls2/flengine_tls2`。警告 0。
- 生成 C の置換行（抜粋）:
  - `L_41cf60: { /* 0x41cf60 jmp [0x1971778] kernel32!TlsGetValue */ fle_imp_TlsGetValue(); return; }`
  - `L_41cf70: { /* 0x41cf70 jmp [0x19717a8] kernel32!TlsSetValue */ fle_imp_TlsSetValue(); return; }`
  - `L_41cf80: { /* 0x41cf80 jmp [0x19713c0] kernel32!GetLastError */ fle_imp_GetLastError(); return; }`
- 注: この build は `$CC -pie` の出力。`--wrap=fprintf` と `--wrap=fputs` は既存の build と同じ。

## 実行 run2（最終版）

出力: `lift/out-tls2/run2.txt`（run1.txt は前の版）。

- 終了コード: 134（SIGABRT）
- 停止点: 0x40a3b2（`stmxcsr (%rsp)`、thunk 0x40a3b0 の中）
- abort の種類: `fle_unsupported`（実行時の runtime message: `fle: unsupported instruction at 0x40a3b2`）
- 関数実行数（distinct）: 16
- 関数の入場回数（calls）: 16
- 呼び出し深さ（停止時）: 4

呼び出し深さ（停止時）:

```
#0 0x40a3b0 f_40a3b0 guest-return=0x412286
#1 0x412160 f_412160 guest-return=0x41d989
#2 0x41d8f0 f_41d8f0 guest-return=0x1376875
#3 0x1376820 f_1376820 guest-return=0xdead0000
```

入場ログ（抜粋。entry 8 以降が今回の変更範囲）:

```
entry 8  callee=0x41cf40 f_41cf40  guest-return=0x41d066   (TlsAlloc の thunk。実装は前回の版で、今回の変更ではない)
entry 9  callee=0x41d000 f_41d000  guest-return=0x41d071
entry 10 callee=0x41cff0 f_41cff0  guest-return=0x41d00a   (LocalAlloc の引数用定数)
entry 11 callee=0x41cfd0 f_41cfd0  guest-return=0x41d02a
entry 12 callee=0x41cf20 f_41cf20  guest-return=0x41cfe2   (LocalAlloc)
entry 13 callee=0x41cf70 f_41cf70  guest-return=0x41d049   (TlsSetValue)
entry 14 callee=0x41cf60 f_41cf60  guest-return=0x41d07c   (TlsGetValue)
entry 15 callee=0x40a3b0 f_40a3b0  guest-return=0x412286   (停止)
```

実測と推論の区別:

- 実測: 入場ログ（entry 13 と 14 の callee と戻り番地）、停止点の VA、abort の種類、関数数、呼び出し深さ。
- 推論: entry 14 の戻り番地が 0x41d07c（0x41d077 の call の次）なので、0x41d000 は戻っており、そこで TlsSetValue も戻っている。戻りを直接記録したわけではない。
- 推論: TlsAlloc の結果（索引 0）は -1 と不一致。0x412a60 は呼ばれていない（entry log に無い）。

## -1 比較（0x41d010）

- 実測: 0x412a60 は呼ばれていない。索引は 0 で、-1 ではない。
- 仕様との照合: `docs/decisions/0004-tls-and-lasterror.md` の「-1 比較の判定」を参照。

## 停止点の命令（次の停止点）

- VA: 0x40a3b2
- 命令: `stmxcsr (%rsp)`（objdump: `40a3b0: push $0x0`、`40a3b2: stmxcsr (%rsp)`、`40a3b6: pop %rax`、`40a3b7: ret`）
- IAT の名前: なし。0x40a3b0 は `jmp [IAT]` ではなく、内部の小さな関数（push、stmxcsr、pop、ret）。IAT には対応しない。
- 呼び出し元: 0x412281（f_412160 の中、`call 0x40a3b0`）。戻り番地 0x412286 は `mov %eax,[0x13a5054]`。
- 性質（推測）: MXCSR を読んで eax に入れ、グローバルに保存する。SSE の状態を読む処理。SSE 命令の対応は別の作業（M1 の未着手項目）。

## 次に進んだか

- 停止点は import（0x41cf70 TlsSetValue、0x41cf60 TlsGetValue）を越えた。
- 次の停止点は import ではなく、SSE 命令 `stmxcsr`（0x40a3b2）。繰り返しはしていない。
- GetLastError（0x41cf80）は、この経路では呼ばれていない。よって未検証。

## 残りの作業量

- 停止点 0x40a3b2 の `stmxcsr` は、SSE の対応が要る（未着手。別の作業単位）。
- GetLastError の応答は、0x41db26 や 0x41dc2b など別の経路で通ると思われる（推測）。
- 0x412a60 の中身（0x4128b0、0x412770、0x409660 の分岐。未読）。
- 全体の残件数（未分類 1,571 件の `fle_unsupported`、生成 C 上）。import 以外も含むので、import の件数はこの実行では数えていない。

## 失敗・拒否された操作

- なし。権限拒否は出ていない。rm、git、/tmp、sleep、timeout は使っていない。
- 既存ファイルの上書きは無し。新規は `lift/out-tls2/` 配下、`lift/runtime/fle_imports.c` と `.h` への追記（既存の関数は変更していない）、docs の新規 5 件。
