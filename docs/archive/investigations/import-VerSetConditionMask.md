# import: kernel32!VerSetConditionMask

作業者による調査。推測と実測を分けて書く。判断は `docs/decisions/0011-verSetConditionMask.md`。

## API の名前と DLL（実測）

- `lift/out-imports/imports.txt` の 400 行: `0x1971658 0x1571658 normal .idata kernel32.dll name VerSetConditionMask 0`。
- DLL は kernel32.dll。対応表の DLL 名も kernel32.dll に合わせた。
- imports.txt の 2 列目は RVA（0x1571658）、1 列目は VA（0x1971658）。harness のイメージベースは 0x400000 で、VA = RVA + 0x400000 と一致する。`jmp [IAT]` のアドレスは VA（0x1971658）を使う。依頼文の 0x1571658 は RVA 側の表記。

## 仕様の出典（公式）

- https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask （2026-10-20 取得）
  - Syntax: `NTSYSAPI ULONGLONG VerSetConditionMask([in] ULONGLONG ConditionMask, [in] DWORD TypeMask, [in] BYTE Condition);`
  - ConditionMask: VerifyVersionInfo の dwlConditionMask に渡す値。初回は 0。
  - TypeMask: 0x1 VER_MINORVERSION、0x2 VER_MAJORVERSION、0x4 VER_BUILDNUMBER、0x8 VER_PLATFORMID、0x10 VER_SERVICEPACKMINOR、0x20 VER_SERVICEPACKMAJOR、0x40 VER_SUITENAME、0x80 VER_PRODUCT_TYPE。
  - Condition: VER_EQUAL 1、VER_GREATER 2、VER_GREATER_EQUAL 3、VER_LESS 4、VER_LESS_EQUAL 5（VER_SUITENAME のときは VER_AND 6、VER_OR 7）。
  - Return value: "The function returns the condition mask value."
  - Remarks: "Call this function once for each bit set in the dwTypeMask parameter of the VerifyVersionInfo function."
  - Requirements: Kernel32.dll（api_location に ntdll.dll も挙がる）。Minimum client: Windows 2000 Professional。
- https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-ver_set_condition （マクロの同じ説明）
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfoa （dwlConditionMask の組み立て方）
- https://learn.microsoft.com/en-us/windows/win32/sysinfo/verifying-the-system-version （使用例）

## 確認できなかった点（公式では）

- dwlConditionMask の中で、各 TypeMask が使うビット位置（3 bit ずつ、どの位置か）。公式ページは書いていない。
- 複数フラグを 1 回に渡したときの挙動。Remarks は「フラグごとに 1 回」としか書いていない。
- Condition の値が 1〜5（VER_SUITENAME は 6、7）以外のときの挙動。
- Windows 本体の実装の中身（公式ページには無い）。

## 補助（非公式）

- Wine dlls/ntdll/version.c の VerSetConditionMask: `condition << k*3`（k は TypeMask のフラグの位置、VER_PRODUCT_TYPE は k=7）。
- `/usr/x86_64-w64-mingw32/include/winnt.h`（mingw-w64）: VER_AND 6、VER_OR 7、VER_CONDITION_MASK 7、VER_NUM_BITS_PER_CONDITION_MASK 3。

## 呼び出し元（実測、objdump）

f_409b30 は 0x409b67、0x409b77、0x409b87 で 3 回呼ぶ（TypeMask は 2、1、0x20、Condition は 3）。戻り値 rax は次の ConditionMask に渡し、最後の値は VerifyVersionInfoW（0x407370、IAT 0x1971620、imports.txt の 393 行）の r8 に入る。詳細は decisions/0011 の表。

## 呼び出し元の一覧（lift/out-usage/import-callers-v3.tsv、読み取り）

- 0x409b30（VerifyVersionInfoW と VerSetConditionMask の両方）
- 0x426310（VerifyVersionInfoW、VerSetConditionMask、LoadLibraryW）
- 0x454030（VerifyVersionInfoW、VerSetConditionMask）
- 閉包の中で jmp [IAT] として直接呼ばれる箇所（対応前の一覧、iat-jmp-unsupported-all-v3.tsv）: 0x407360 と 0x423cd0。0x423cd0 は objdump で `jmp [0x1971658]` を確認したが、今回の実行では通っていないため、対応後の動作は未確認。
