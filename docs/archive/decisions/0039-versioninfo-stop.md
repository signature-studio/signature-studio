# 0039: VerifyVersionInfoW の停止（dwOSVersionInfoSize = 0）は直さず、名前付き abort のままにする

状態: 作業者（M1 作業者）が判断。依頼者（PM）の判断は含まない。
変更: ランタイム、lifter は変更なし。回帰テストの追加のみ（`lift/crates/fe-lift/tests/versioninfo.rs` 新規 3 件、`tests/osver.rs` の `undefined_cases_abort` に 1 行追記）。実行記録は `docs/investigations/versioninfo-stop-run.md`。

## 文脈

W1（0036）の後、all-pdata の harness は 0x4072ff を通過し、`fle_osver.c` の `fle: VerifyVersionInfoW: dwOSVersionInfoSize is not sizeof(OSVERSIONINFOEXW) (not defined by the MSDN page; not implemented)` で止まる。停止時の呼び出し鎖は `f_423cc0`（IAT 0x1971620 への thunk）← `f_454030` ← `f_454090` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← `f_1376820`（入口）。0014 が想定した呼び出し元 f_409b30 とは別の呼び出しである。

## 実測（gdb と DLL のバイト、`lift/out-ijmp/bin/flengine-ijmp-pdata`）

- この実行で `fle_imp_VerifyVersionInfoW` は 2 回呼ばれる。
  - 1 回目: f_409b30（f_409bd0 ← f_409bf0 ← f_41ce10 経由）。R1 = 構造体、先頭 `dwOSVersionInfoSize` = **0x11c**、dwMajorVersion 6、dwMinorVersion 1（先頭 16 バイトを読んだ。SP major は読んでいない）。R2 = 0x23、R8 = 0x1801b。成功して実行は先へ進んだ。
  - 2 回目（停止）: f_454030 の 0x45406e の call。R1 = 構造体（`lea rcx,[rbp+0x24]`）、**R2 = 0x80**（VER_PRODUCT_TYPE）、**R8 = 0x200000**（= 1 << 21、product type の条件 3 bit に 1 = VER_EQUAL）。構造体 284 バイトのうち、**0 でないバイトは +282（wProductType）= 1 の 1 つだけ**。`dwOSVersionInfoSize` は **0**。
- f_454030 の命令（`objdump -d`、0x454030〜0x454080）: `lea rcx,[rbp+0x24]; mov edx,0x11c; xor r8,r8; call 0x40d210`（memset(構造体, 0x11c, 0) 相当。0x40d210 は 0036 の記録のとおり memset）→ `mov BYTE PTR [rbp+0x13e],1`（= 構造体 + 282）→ `xor ecx,ecx; mov edx,0x80; mov r8b,1; call 0x423cd0`（VerSetConditionMask(0, 0x80, 1)）→ `lea rcx,[rbp+0x24]; mov edx,0x80; mov r8,rax; call 0x423cc0`（VerifyVersionInfoW）→ `test eax,eax; sete al`。**`dwOSVersionInfoSize` に書く命令は関数内にない**（DLL のバイトを直接読んだ。関数 0x51 バイトの中の `1c 01 00 00` は 0x454040 の `mov edx,0x11c`（memset の個数）の 1 か所だけ）。
- 対照: 同じ DLL の f_426310（TypeMask 0x23 の別のバージョン確認）は 0x42634f に `mov DWORD PTR [rbp+0x44],0x11c` を持つ。f_426310 は今回の実行では呼ばれていない（1 回目の呼び出しの 0x11c は f_409b30 側のもの）。この対照は DLL のバイトの比較だけで、実行の比較ではない。
- lift 側の値の確認: R1 = 構造体のアドレス、R2 = 0x80、R8 = 0x200000 は、上の命令列が計算する値と一致する。memset の後の構造体は、0x45404c の 1 バイトを除いて 0（gdb で 284 バイトを読んだ。構造体の直前のフレームを 0xAB で埋めてから実行しても、構造体の中は 0 のまま）。

## 分類

- (a) lift 側の初期化の欠陥: **該当しない**。ゲストの命令列に size を書く命令がなく、lift はその命令列のとおりに実行している。memset の経路（0x40d210 の switch、W2）は、0xAB で埋めたフレームの上でも構造体を 0 にした。
- (b) ABI（R1/R2/R8）の読み違い: **該当しない**。R1 = lpVersionInformation、R2 = dwTypeMask、R8 = dwlConditionMask は、Microsoft Learn のシグネチャ（`LPOSVERSIONINFOEXW, DWORD, DWORDLONG`）と一致し、値も命令列から計算した値と一致した。1 回目の呼び出し（size 0x11c、0x23、0x1801b）は同じ読み方で成功している。
- (c) ゲストが別のサイズを渡す: **SDK 版の違いによる別のサイズではない**。渡す値は **0** で、ゲストの f_454030 が size を設定していない（上記の命令列）。分類は (a)(b)(c) のどれにも当てはまらず、ゲスト側の入力が仕様の「しなければならない」を満たしていないケース。

## 仕様

Microsoft Learn（2026-10-20 取得）:
- VerifyVersionInfoW: `lpVersionInformation` は `OSVERSIONINFOEX` へのポインタで、"You must set the dwOSVersionInfoSize member of this structure to sizeof(OSVERSIONINFOEX)"。size が違うときの戻り値、last error は書かれていない。戻り値の節は、条件を満たさなければ 0 と ERROR_OLD_WIN_VERSION、"If the function fails, the return value is zero and GetLastError returns an error code other than ERROR_OLD_WIN_VERSION" とだけ書く。
- OSVERSIONINFOEXW: "Set this member to sizeof(OSVERSIONINFOEX)"。
- RtlVerifyVersionInfo（WDK）: 戻り値は STATUS_INVALID_PARAMETER（"The input parameters are not valid"）など 3 種類。size への言及はない。

公式ページに、size が 0 または 284 以外のときの結果の定義はない。

参考（Windows の仕様ではない。実装の挙動の事実のみ。`ocean_webfetch` で取得したソース）:
- ReactOS の `RtlVerifyVersionInfo`（sdk/lib/rtl/version.c）は `VersionInfo->dwOSVersionInfoSize` を読まない。冒頭の FIXME に、"Check the following special case on Windows (various versions): ... lp->dwOSVersionInfoSize != sizeof(OSVERSIONINFOEXW)" とあり、Windows での結果は ReactOS 側でも未確認と読める。
- Wine の `RtlVerifyVersionInfo`（dlls/ntdll/version.c）も size を読まない。
- mingw-w64 の `IsWindowsServer`（versionhelpers.h）は `{sizeof(vi),0,0,0,0,{0},0,0,0,VER_NT_WORKSTATION}` で size を設定し、同じ TypeMask（VER_PRODUCT_TYPE）、同じ VER_EQUAL で呼ぶ。f_454030 と同じ形のテストだが、size を設定する点が違う。

## 判断

- (b) 実装しない: `dwOSVersionInfoSize` が 0 のときの結果（成功、失敗、どの last error か）は、公式の仕様から決まらない。ReactOS と Wine が size を読まないことは、Windows の挙動の根拠にならない（ReactOS 自身が FIXME で未確認としている）。0007（fail-soft の続行は進捗と認めない）に従い、size を 0 のまま通す既定値、size を 0x11c とみなす補正は入れない。
- `fle_osver.c` の停止は、仕様が結果を定義しないケースに対する名前付き abort として正しいので、変更しない。0014 の「dwOSVersionInfoSize が 284 以外は abort」の規則を維持する。
- OSVERSIONINFOW 形（276）を VerifyVersionInfoW が受けるかは、公式ページが OSVERSIONINFOEX だけを書くので受けない（変更なし）。今回のゲストが渡すのは 276 でもなく 0 である。
- 次の判断に必要な事実（PM の判断待ち）: この呼び出しの結果が実際の Windows で何になるかは、実機の測定でしか確定できない（このマシンは Linux）。推測: f_454030 の元のソースは「構造体をゼロ初期化して wProductType だけ設定する」形で、size の設定を落としている。ゲストが自分のバグを持つ場合、実際の Windows でも結果が 0（呼び出し側の `sete al` で「Workstation」と判定される向き）になる可能性があるが、これは推測であり、仕様に基づくものではない。

## 回帰テスト

- `fe-lift/tests/versioninfo.rs`（新規、3 件、実 DLL のバイトを `iced-x86` で独立に復号）: `f_454030_never_stores_the_size_member`（関数 0x454030〜0x454081 の復号が境界で終わる、call 先 0x423cc0、構造体 `[rbp+0x24]`、memset の個数 0x11c、`[rbp+0x13e]` への 1 の書き込みが唯一の rbp 相対書き込みで、`[rbp+0x24..0x28]` への書き込みがない）、`f_426310_stores_the_size_member`（対照。同じ検出関数が f_426310 の `[rbp+0x44]` への 0x11c の書き込みを検出する）、`thunks_used_by_f_454030`（0x423cc0 が IAT 0x1971620、0x423cd0 が 0x1971658 への jmp）。DLL が既定のパスに無ければスキップ、`FLENGINE_PE` 指定でファイルが無ければ失敗。
- `fe-lift/tests/osver.rs` の `undefined_cases_abort` に行 `size-0-as-f_454030` を追記（size 0、wProductType 1、TypeMask 0x80、VER_EQUAL の呼び出しが SIGABRT で止まり、`fle:` の行を出し、戻らない）。
- 変異テスト（検出力の確認）は行っていない。

## 限界

- 実機の Windows との照合はできていない（ホストは Linux）。
- 1 回目の VerifyVersionInfoW（0x11c）の返り値の意味（0014 の「Windows 7 SP1 以上」の読み）は、0014 のまま。
