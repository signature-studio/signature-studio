# VerifyVersionInfoW の停止（W1 の後）の原因調査

日付: 2026-10-20。判断は `docs/decisions/0039-versioninfo-stop.md`。成果物は `lift/out-vi/`（`tmp/` は TMPDIR、cargo の target は `out-vi/target`）。手順は `pdata-seeds-run.md`、`indirect-jmp-run.md` のとおり（split ビルドの `lift/out-ijmp/bin/flengine-ijmp-pdata` を使う。再生成・再ビルドはしていない: ランタイム、lifter を変更していないため）。

「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 結果（要約）

- 原因の分類: (a) lift 側の初期化の欠陥 = 該当しない、(b) ABI の読み違い = 該当しない、(c) ゲストが別のサイズを渡す = サイズの違いではなく **dwOSVersionInfoSize が 0**（ゲストの f_454030 が size を書く命令を持たない）。
- 修正は入れていない（ランタイム、lifter は変更なし）。仕様から結果が決まらないため、名前付き abort のまま。回帰テストを追加した。
- 停止点は変わらない: **VerifyVersionInfoW（f_423cc0 ← f_454030）**、関数実行数（distinct）**147**、関数入口 **736**、深さ 7、`unsupported_total` **698**（`out-ijmp/gen-pdata.txt`。再生成していないので不変）。
- 参考（修正ではない実験、§4）: gdb で size を 0x11c に書き換えると、次は `GetLocaleInfoW`（0x423560）で止まる（LCType 0x100b）。

## 2. 停止時のゲスト状態（実測、gdb。`gdb1.txt`〜`gdb8.txt`、`gdb*.cmds`）

停止（SIGABRT）の直前の呼び出し鎖（`gdb2.txt` の bt。ゲスト VA は `run.txt` の `#n` と一致）:

| # | 関数 | guest-return |
|---|---|---|
| 0 | f_423cc0（0x423cc0 `jmp [0x1971620]`、VerifyVersionInfoW の thunk） | 0x454073 |
| 1 | f_454030 | 0x45456a |
| 2 | f_454090 | 0x412129（`fle_dispatch` 経由） |
| 3 | f_4120c0 | 0x41229a |
| 4 | f_412160 | 0x41d989 |
| 5 | f_41d8f0 | 0x1376875 |
| 6 | f_1376820（入口） | 0xdead0000 |

`fle_imp_VerifyVersionInfoW` は実行中に **2 回**呼ばれる:

| 回 | 呼び出し元 | R1（構造体） | dwOSVersionInfoSize | R2（dwTypeMask） | R8（dwlConditionMask） | 結果 |
|---|---|---|---|---|---|---|
| 1 | f_409b30（f_409bd0 ← f_409bf0 ← f_41ce10） | 0x7ffff7bffa44 | **0x11c（284）** | 0x23 | 0x1801b | 成功（先へ進む） |
| 2 | f_454030（0x45406e） | 0x7ffff7bff924 | **0** | **0x80**（VER_PRODUCT_TYPE） | **0x200000**（product type の条件 = 1 = VER_EQUAL） | abort |

- 1 回目の構造体（実測、`gdb1.txt`、先頭 16 バイトのみ読んだ）: size 0x11c、dwMajorVersion 6、dwMinorVersion 1、dwBuildNumber 0。SP major（+276）は読んでいない。TypeMask 0x23 は major、minor、SP major。1 回目の戻り値は直接見ていない（実行がその後も続き、2 回目の呼び出しに到達したことから、abort していないとだけ言える）。
- 2 回目の構造体（実測、`gdb3.txt`、`gdb8.txt`）: **284 バイトのうち 0 でないバイトは +282（wProductType）= 1 の 1 つだけ**。size（+0）、major、minor、build、platform（+4〜+19）、szCSDVersion、SP、suite、wReserved はすべて 0。構造体の直前 1 バイトは 0xAB（構造体の手前のスタックを 0xAB で埋めてから走らせた。memset が構造体の 284 バイトだけを 0 にし、外へはみ出さないことを確認）。直後の 1 バイト（+284）は 0x50（隣のフレームの値）。
- 構造体は f_454030 のフレーム上（入口の rsp − 0x124）。

## 3. ゲスト側の呼び出し元（`objdump -d -M intel`、`dis-all.txt` と DLL のバイト、`rawcheck.py`）

f_454030（0x454030〜0x454080、0x51 バイト）:

```
454030 push rbp
454031 sub rsp,0x140
454038 mov rbp,rsp
45403b lea rcx,[rbp+0x24]       ; 構造体
45403f mov edx,0x11c            ; memset の個数
454044 xor r8,r8                ; 値 0
454047 call 0x40d210            ; memset
45404c mov BYTE PTR [rbp+0x13e],1   ; = 構造体 + 282 (wProductType)
454053 xor ecx,ecx
454055 mov edx,0x80
45405a mov r8b,1
45405d call 0x423cd0            ; VerSetConditionMask(0, 0x80, 1)
454062 lea rcx,[rbp+0x24]
454066 mov edx,0x80
45406b mov r8,rax
45406e call 0x423cc0            ; VerifyVersionInfoW
454073 test eax,eax
454075 sete al
```

- `dwOSVersionInfoSize` に書く命令はない。関数 0x51 バイトの中の `1c 01 00 00` は 0x454040 の `mov edx,0x11c`（memset の個数）の 1 か所だけ（`rawcheck.py`）。lift した C（`out-ijmp/out/ijmp-pdata-part-00.c` の `f_454030`）も同じ命令列を 1 対 1 で出している（`L_45403f` から `L_45406e`）。
- 呼び出し元の f_454030 は 0x454565 と 0x4546e5 の 2 か所から呼ばれる（`test al,al` の結果で分岐する）。
- 対照: f_426310（TypeMask 0x23 のバージョン確認）は 0x42634f に `mov DWORD PTR [rbp+0x44],0x11c` を持ち、size を設定する（`rawcheck2.py`）。f_454030 には対応する命令がない。これは DLL のバイトの比較であり、f_426310 は今回の実行では呼ばれていない。
- f_409b30（1 回目）は 0x409b56 の `movl $0x11c,0x24(%rbp)` で size を設定する（0014 の記録）。

## 4. 実験（修正ではない。製品の変更なし）

gdb（`gdb6.cmds`、`gdb7.cmds`）で 2 回目の呼び出しの R1 の先の size を 0x11c に書き換えて続行した（`out-ijmp/bin/flengine-ijmp-pdata` のメモリ上の一時的な書き換え。ランタイムの再ビルドはしていない）:

- VerifyVersionInfoW は通過し、次の停止は `fle: GetLocaleInfoW: LCType value for en-US is not on the MSDN pages`（`fle_locale.c` の 283 行、名前付き abort）。関数実行数 **189**、関数入口 **957**、深さ 10。呼び出し鎖: f_423560 ← f_446370 ← f_447860 ← f_446400 ← f_44bb70 ← f_455790 ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376820。引数（gdb、`gdb7.txt`）: Locale 0x409、LCType **0x100b**、バッファ 0x7ffff7bff620、cchData 0x100。
- この数値は「size が 0x11c だったとしたら」の仮定の実験で、仕様から決まった挙動ではない。size が 0x11c のときの `fle_osver_check` の結果（wProductType の VER_EQUAL 比較）は、実装の規則どおりに product_type 1 == 1 で成功を返した、と読んでいる（gdb で戻り値は直接見ていない。推測を含む）。
- 実験は 0007（fail-soft の続行は進捗と認めない）に従い、進捗として数えない。

## 5. 仕様の確認（Microsoft Learn と参考実装、詳細は 0039）

- VerifyVersionInfoW、OSVERSIONINFOEXW とも「size を sizeof(OSVERSIONINFOEX) にしなければならない」とのみ書き、他の値のときの結果は書かない。引数の順序は `LPOSVERSIONINFOEXW, DWORD, DWORDLONG` で、R1/R2/R8 の読み方と一致。
- 参考: ReactOS（FIXME で Windows の挙動を未確認とする）と Wine の `RtlVerifyVersionInfo` は size を読まない。Windows の挙動の根拠としては使っていない。

## 6. テスト

- 追加: `fe-lift/tests/versioninfo.rs` 3 件（実 DLL のバイトを `iced-x86` で復号して、f_454030 に size を書く命令がないこと、対照の f_426310 には書く命令があること、thunk の IAT スロットを確認）。`fe-lift/tests/osver.rs` の `undefined_cases_abort` に 1 行（`size-0-as-f_454030`）。単独実行: osver 6 件 pass、versioninfo 3 件 pass（`cargo test --offline -p fe-lift --test versioninfo --test osver`）。
- ワークスペース全体（`out-vi/test-ws.txt`、`test-ws.exit` は EXIT=101、`cargo test --offline --workspace --no-fail-fast`）: 62 バイナリ、**549 件 passed、11 件 failed**、0 ignored（`test result` 行の集計）。
  - 失敗 11 件の内訳:
    - **x87（他者の作業）8 件**: `tests/x87.rs` 7 件（`x87_arith_memory_forms`、`x87_arith_register_forms`、`x87_compares`、`x87_integer_stores`、`x87_loads`、`x87_misc`、`x87_stores`。`clang: error: no such file or directory: runtime/fle_x87.c`）、`tests/x87ctl.rs` の `x87_arithmetic_stays_unsupported` 1 件（`lift_body(&[0xd9, 0x07], 0x1000).is_err()` が成り立たない）。
    - **oleaut_ole 3 件（x87 ではない、私の変更でもない）**: `tests/oleaut_ole.rs` の `sys_alloc_string_len_layout`（C プログラムが SIGSEGV）、`sys_realloc_string_len`（期待値との不一致）、`real_dll_handles_the_rows`（`SysAllocStringLen is a handled thunk`）。`oleaut_ole.rs`、`runtime/fle_ole.c`、`fle_oleaut.c` ほかは作業ツリーの未追跡ファイルで、更新時刻は 23:15〜23:17（私のテスト実行の少し前〜途中。実行中に他の担当が編集していた可能性があるが、確認していない）。私はこれらを触っていない。
  - 前回（indirect-jmp-run.md §4.3）は 531 passed、8 failed（x87 のみ）。差 +18 passed、+3 failed の増加分のうち、私の分は versioninfo.rs の 3 件（osver.rs への 1 行は既存テストの中の行）。残りは他担当の `oleaut_ole.rs` など（推測を含む。内訳は未確認）。

## 7. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き（新規ファイルへのリダイレクトのみ）、git 操作、`/tmp`、sleep/timeout は使っていない。TMPDIR は `lift/out-vi/tmp`。権限拒否はなし。外部通信は Microsoft Learn、GitHub 上のソース（Wine、ReactOS、mingw-w64）の取得と検索。
- 2 GB 級の中間ファイルなし。`lift/out-vi/` は約 2.0 GB（cargo の target が大半、`dis-all.txt` は 266 MB。`out-pdata` の大きなファイルは複製していない）。
- 後始末できていないもの（削除禁止）: `out-vi/dis-all.txt`（0x401000〜0x1370000 の逆アセンブル、266 MB）、`out-vi/gdb*`、`rawcheck*.py`。

## 8. 触ったファイル

- 新規: `docs/decisions/0039-versioninfo-stop.md`、`docs/investigations/versioninfo-stop-run.md`、`lift/crates/fe-lift/tests/versioninfo.rs`、`lift/out-vi/`（作業物一式）。
- 変更: `lift/crates/fe-lift/tests/osver.rs`（`undefined_cases_abort` に 1 行とコメント 3 行を追記）。
- 触っていない: x87 関連（`x87.rs`、`tests/x87.rs`、`fle_x87.h`、`lib.rs`、`fle_mmx.h`）、ランタイム、lifter、`imports_table.rs`。
