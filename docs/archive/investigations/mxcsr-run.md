# MXCSR 対応後の実行（out-mxcsr）

作業者（PM）の計測。出力は `lift/out-mxcsr/run1.txt`。

## 生成と実行

- 生成 C: `lift/out-mxcsr/out/plus41d-mxcsr.c`。`lift/out-tls2/out/plus41d-tls2.c`（読み取りのみ）の 1 行を置換して作った。置換は `build.sh` の sed で、grep で 1 回を確認（`fle_mxcsr_store` の出現 1 回、`fle_unsupported(UINT64_C(0x40a3b2))` の残りは 0 回）。
- 置換後の行: `L_40a3b2: { /* 0x40a3b2 Stmxcsr */ { void fle_mxcsr_store(uint64_t); fle_mxcsr_store((R[4])); }}`
- 生成器の出力と一致するかは、テスト `stmxcsr_rsp_body_text` で確認（生成器の出力は `{ void fle_mxcsr_store(uint64_t); fle_mxcsr_store((R[4])); }`）。
- リンク: `lift/out-mxcsr/build.sh` で成功（`flengine_mxcsr`）。`fle_mxcsr.c` を追加してリンク。
- 実行: `flengine_mxcsr` に `binaries/win64/FLEngine_x64.dll` を渡した（読み取りのみ）。exit 134（SIGABRT）。

## 停止点（実数）

| 項目 | 値 |
| --- | --- |
| VA | 0x41e7e0 |
| 命令（objdump） | `sub DWORD PTR [rip+0x11cdae1], 1`（0x41e7e0 の先頭） |
| 関数名 | f_41e7e0（内部。IAT ではない） |
| 呼び出し元 | 0x412126 の `call QWORD PTR [rbp+0x38]`（f_4120c0 の中） |
| abort の種類 | fle_dispatch_miss（runtime message: `fle: indirect call to 0x41e7e0 is not a known function entry`） |
| 関数実行数（distinct） | 17 |
| 関数の入口の回数（calls） | 17 |
| 呼び出し深さ（stop 時） | 4 |

コールスタック（stop 時）:
- #0 0x4120c0 f_4120c0（guest-return 0x41229a）
- #1 0x412160 f_412160（guest-return 0x41d989）
- #2 0x41d8f0 f_41d8f0（guest-return 0x1376875）
- #3 0x1376820 f_1376820（guest-return 0xdead0000）

前回（out-tls2）は 16 関数・深さ 4 で 0x40a3b2 で止まった。今回は 17 関数、同じ深さで、0x41e7e0 への dispatch miss で止まった。

## 残りの作業量

- 0x41e7e0 の扱い（内部の関数の入口として解析に入れるか、関数の境界の解析の見直しが要るか）は未着手。
- 停止点の繰り返しは、この作業の指示（次の停止点を報告して止める）に従い、やっていない。
