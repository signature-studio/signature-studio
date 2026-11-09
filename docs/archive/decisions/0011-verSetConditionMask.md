# 0011: kernel32!VerSetConditionMask

状態: 作業者（M1 作業者）が決定。依頼者（PM）の判断は含まない。実装は `lift/runtime/fle_imports.c` と `fle_imports.h`（追記のみ、既存の関数は変更なし）。対応表は `lift/crates/fe-lift/src/imports_table.rs` に 1 件追加。テストは `lift/crates/fe-lift/tests/ver.rs`（新規）。

## 文脈

0x407360 の `jmp [IAT 0x1971658]` が停止点だった。`lift/out-imports/imports.txt` の 400 行に `0x1971658 ... kernel32.dll name VerSetConditionMask` がある。呼び出し元は f_409b30（0x409b67、0x409b77、0x409b87 の 3 回）と f_409bd0 から。呼び出し元の戻り値は VerifyVersionInfoW（0x407370、imports.txt の 393 行）へ渡されている（後述）。

## 仕様の出典

以下は Microsoft Learn（2026-10-20 取得）。

- VerSetConditionMask: https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask
  - 読んだ箇所: Syntax（`ULONGLONG VerSetConditionMask(ULONGLONG ConditionMask, DWORD TypeMask, BYTE Condition)`）、Parameters（TypeMask の値 0x1 から 0x80、Condition の値 1 から 5、VER_SUITENAME のときは 6 と 7）、Return value（"The function returns the condition mask value."）、Remarks（"Call this function once for each bit set in the dwTypeMask parameter of the VerifyVersionInfo function."）。
  - DLL は Kernel32.dll（ページの Requirements。api_location には ntdll.dll も挙がる）。
- VER_SET_CONDITION マクロ: https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-ver_set_condition （同じ引数の説明）。
- VerifyVersionInfoA/W: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfoa （dwlConditionMask は「VerSetConditionMask 関数、または VER_SET_CONDITION マクロで、比較する要素ごとに 1 回ずつ組み立てる」）。
- Verifying the System Version: https://learn.microsoft.com/en-us/windows/win32/sysinfo/verifying-the-system-version （使い方の例。VER_SET_CONDITION を要素ごとに呼ぶ）。
- 補助（非公式、実装の根拠の候補）: Wine の dlls/ntdll/version.c（`VerSetConditionMask`）。条件を `condition << k*3` で OR する。k は TypeMask の各フラグのビット位置（VER_MINORVERSION 0x1 は k=0、VER_PRODUCT_TYPE 0x80 は k=7）。mingw-w64 の `/usr/x86_64-w64-mingw32/include/winnt.h`（行 9437 から 9465）で、`VER_AND` が 6、`VER_OR` が 7、`VER_CONDITION_MASK` が 7、`VER_NUM_BITS_PER_CONDITION_MASK` が 3、`VER_SET_CONDITION` が `VerSetConditionMask` の呼び出しであることを確認。

## 確認できなかった点

- Microsoft の公式ページには、dwlConditionMask の中で各 TypeMask がどのビット位置を使うか（3 bit ずつ、どの k に置くか）が書かれていない。公式ページ（2 つ）は「戻り値は条件マスク」としか言っていない。したがって、ビット配置は公式には確認できていない。
- 上の配置は Wine の実装と mingw-w64 の定数から取った。Windows 本体の実装で一致するかは確認していない（ホストは Linux で、Windows の挙動は実行できない）。
- 複数のフラグを 1 回に渡した場合（TypeMask に 2 bit 以上）の挙動は、公式ページでは確認できない（Remarks は「フラグごとに 1 回」）。
- Condition の 3 bit 外（6、7 以外の 8 以上）の扱いは、公式ページでは確認できない。
- 戻り値が返す値の、VER_SUITENAME（TypeMask 0x40）のときの 6、7 の扱い、0 のときの扱いは、公式ページでは確認できない。

## 呼び出し元の戻り値の使われ方（objdump の根拠）

`binaries/win64/FLEngine_x64.dll` の f_409b30（0x409b30）:

```
409b64: mov $0x3,%r8b          ; Condition = 3 (VER_GREATER_EQUAL)
409b67: call 0x407360          ; VerSetConditionMask(ConditionMask=rcx=0, TypeMask=rdx=2 (VER_MAJORVERSION))
409b6c: mov %rax,%rcx          ; 戻り値を次の ConditionMask に
409b6f: mov $0x1,%edx          ; TypeMask = 1 (VER_MINORVERSION)
409b74: mov $0x3,%r8b
409b77: call 0x407360
409b7c: mov %rax,%rcx
409b7f: mov $0x20,%edx         ; TypeMask = 0x20 (VER_SERVICEPACKMAJOR)
409b84: mov $0x3,%r8b
409b87: call 0x407360
409b8c: ...                    ; OSVERSIONINFOEX を組み立てる（dwMajorVersion など）
409b8c-409b98: mov %ebx,0x28(%rbp) ...
409b9f: lea 0x24(%rbp),%rcx    ; lpVersionInfo
409ba3: mov $0x23,%edx         ; dwTypeMask = 0x23 (MAJOR|MINOR|SPMAJOR)
409ba8: mov %rax,%r8           ; dwlConditionMask = 直前の戻り値
409bab: call 0x407370          ; VerifyVersionInfoW
409bb0: test %eax,%eax
409bb2: setne %al              ; 戻り値 BOOL を 0/1 にする
```

- 戻り値 rax は、次の VerSetConditionMask の rcx（ConditionMask）に入る。最後の戻り値 rax は VerifyVersionInfoW の r8（dwlConditionMask）に入る。
- 3 回の TypeMask は 2、1、0x20。Condition は 3（VER_GREATER_EQUAL）。
- 0x409bd0 は `mov $0x6,%cx; mov $0x1,%dx; mov $0x1,%r8w; call 0x409b30`。つまり f_409b30 の引数は、dwMajorVersion（cx=6）、dwMinorVersion（dx=1）、wServicePackMajor（r8w=1）に見える（推測。関数の引数名は未確認）。
- 0x409bf0 は `call 0x409bd0` の後で `test %al,%al` とその結果を見る。VerifyVersionInfoW の戻り値（BOOL）の 0/1 を、この関数の戻り値として使っている（呼び出し元の判断は未確認）。
- 0x409b30 は VerifyVersionInfoW の結果（setne）を返すだけで、戻り値 rax の値を別の所で使ってはいない。

## 判断（方針）

1. 応答は「条件マスクの計算だけ」を行う。OS の状態は見ない（仕様どおり。VerSetConditionMask は OS を見ず、ビットを組み立てるだけ）。
2. 規則: `ConditionMask | (Condition & 7) << (3 * k)`。k は TypeMask の最上位のフラグのビット位置。TypeMask が 0 や対応しないときは ConditionMask をそのまま返す。Wine の条件分岐（VER_PRODUCT_TYPE から VER_MINORVERSION の順に、最初に一致したもの）と同じ。
3. 仕様が確認できない点は、上の「確認できなかった点」のとおり。この規則はその中で最も近い実装（Wine）に合わせたもので、Windows 本体との一致は未確認。
4. 戻り値は R[0]（rax）に入れる。引数は rcx（R[1]）、rdx（R[2]）、r8（R[8]）を読む。Condition は r8 の下位 8 bit（BYTE）として読み、& 7 する。
5. VerifyVersionInfoW（0x407370）は実装しない。依頼により、この作業の範囲外。

## テスト

`lift/crates/fe-lift/tests/ver.rs`:

- `table_thunk_calls_runtime_and_others_stay_unsupported`: 対応表の thunk が `fle_imp_VerSetConditionMask(); return;` を呼ぶこと。VerifyVersionInfoW は未対応のまま（`fle_unsupported`）。
- `runtime_mask_matches_hand_computed_layout`: 12 件。期待値は `Condition << 3k` を手で計算した値を書いた（例: VER_MAJORVERSION（k=1）、条件 3 → 0x18。VER_SERVICEPACKMAJOR（k=5）、条件 3 → 0x18000）。Condition の 3 bit 化（0x0f → 7）、2 つの呼び出しの連結、TypeMask が複数フラグのときの最上位（Wine 順）も含む。
- 結果: `cargo test --offline --workspace` は全件通過（EXIT=0）。ログは `lift/out-ver/test-workspace-1.txt`。`ver` は 2 件通過。

## 実測として言えないこと

- Windows 本体の VerSetConditionMask の戻り値との一致は、ホストが Linux のため未確認。
- 期待値は仕様の「規則」（Wine の実装から取った配置）を手で式にしたもの。公式ページの文言との一致は、ビット配置については確認していない（上のとおり）。
