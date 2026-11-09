# 0x40a3b0 の thunk と呼び出し元（IAT ではない）

作業者（PM）の調査。objdump（`binaries/win64/FLEngine_x64.dll`、読み取りのみ）で確認。

## thunk 0x40a3b0

```
40a3b0: 6a 00           push   0x0
40a3b2: 0f ae 1c 24     stmxcsr DWORD PTR [rsp]
40a3b6: 58              pop    rax
40a3b7: c3              ret
```

- IAT ではない（indirect `jmp [slot]` ではない）。
- 中身の読み取り: 0 を push し、そのスタック位置に MXCSR を書き、pop で値を rax に取り出して戻る。`rax` は戻り値の受け渡しに使われる。
- 推測: MXCSR の現在値を rax で返す関数（`_mm_getcsr` 相当）。これは中身から読んだ推測で、呼び出し先の意図の確認はしていない。

## 呼び出し元 0x412281（f_412160 の中）

```
41227a: c6 05 03 3e 1d 01 01   mov BYTE PTR [rip+0x11d3e03], 0x1     # 0x15e6084
412281: e8 2a 81 ff ff         call 0x40a3b0
412286: 89 05 c8 2d f9 00      mov DWORD PTR [rip+0xf92dc8], eax       # 0x13a5054
41228c: 83 bd 70 01 00 00 01   cmp DWORD PTR [rbp+0x170], 0x1
```

- 412270: `cmp qword [rip+0x11d3e00], 0`（0x15e6078）。0 でなければ 41228c へ飛ぶ。0 なら 41227a 以降の処理（フラグ 0x15e6084 を 1 にし、0x40a3b0 を呼ぶ）。
- 呼び出しの戻り値 eax を global 0x13a5054 に保存する。
- 同じ global 0x13a5054 は、objdump の別の箇所で `ldmxcsr [rip+...]`（0x40a36c、0x40f530）の読み込み元、`mov [rip+...], ecx`（0x40a366）の書き込み先として出てくる（`lift/objdump-text.txt` 14361・14369・14370 行、20960 行、24762 行）。
- 読み取りの範囲での推測: 0x13a5054 は MXCSR の保存値で、0x412286 の時点で現在値を保存し、0x40a36c などで復元するために使われる。確認していない（0x40a366 の書き込み、0x40a36c の読み込みは、この文書の範囲では全体の流れを読んでいない）。
- 0x412241 から 0x412281 の前後: 0x412241 の `repz cmp` と 0x412258 の `call [rbp+0x160]` は、この調査の範囲では外。

## 実行での確認

- 停止点は 0x40a3b2（`lift/out-tls2/run2.txt`）。
- 0x40a3b2 を実装した後（`lift/out-mxcsr/run1.txt`）、次の停止点は 0x41e7e0 への間接 call（後述）。
- run1（stmxcsr 対応後）では 0x40a3b0 は 1 回だけ呼ばれた（entry 15）。

## 次の停止点: 0x41e7e0（内部、IAT ではない）

- 呼び出しの命令: 0x412126 の `call QWORD PTR [rbp+0x38]`（`f_4120c0` の中）。
- 宛先 0x41e7e0 の命令（objdump）: `sub DWORD PTR [rip+0x11cdae1], 1`（0x15ec2c8）、`cmp DWORD PTR [rip+0x11cdada], -1`、`ret`。
- 直前は 0x41e7dd の `ret`、0x41e7de と 0x41e7df の `int3` の padding。よって 0x41e7e0 は関数の先頭候補（内部の関数）。
- IAT（`lift/out-imports/imports.txt`、798 行）には 0x41e7e0 のエントリはない（grep で 0 件）。
- 性質: 内部。`fle_dispatch` の表（関数の入口の集合）に 0x41e7e0 が無いため、dispatch miss で止まった（harness の abort kind は `fle_dispatch_miss`）。
- 推測: 0x412126 の間接 call の関数ポインタの値が 0x41e7e0 を指す。この関数が入口として解析の対象に入っていない理由は、この調査では確認していない（解析の入口集合の作り方の確認が要る）。
