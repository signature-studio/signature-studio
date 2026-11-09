# VerifyVersionInfoW に dwOSVersionInfoSize = 0 を渡したときの挙動: 仕様・参考実装・ゲスト側の影響

日付: 2026-10-21（取得は 2026-10-20〜21）。読み取り専用の調査。ランタイム、lifter、テストは変更していない。
前提文書: `docs/decisions/0014-emulated-os.md`、`0039-versioninfo-stop.md`、`docs/investigations/versioninfo-stop-run.md`。方針は ADR-0007/0009/0010（仕様で確認できない挙動に既定値を入れて進めない）。

表記: 【事実】= 取得したテキストまたはツール出力で確認した。【推測】= 解釈。【未確認】= 確認できていない。
取得した原本は `lift/out-vi2/src/` に保存した（ファイル名は §8）。

## 0. 結論（要約）

1. Microsoft Learn の 5 ページ（VerifyVersionInfoW、RtlVerifyVersionInfo、OSVERSIONINFOEXW、VerSetConditionMask、IsWindowsServer）と補助 2 ページ（Verifying the System Version、Version Helper functions）を全文取得して確認した。**dwOSVersionInfoSize が 0 または不正のときの戻り値、GetLastError、ERROR_INVALID_PARAMETER への言及は、どのページにもない**【事実】。書かれているのは「set しなければならない」だけ。
2. 非公式の実装（ReactOS、Wine、NT5 系の流出ソースの抜粋）は、`RtlVerifyVersionInfo` / `VerifyVersionInfoW` の入力構造体の `dwOSVersionInfoSize` を**読まない**。Wine の単体テストは「size = 0 でも VerifyVersionInfoA は成功する」を期待値にしている。ただし各材料に弱点がある（§2.5）。**実機 Windows 10 の結果を示す一次資料は得られていない**【事実】。
3. ゲスト側: `f_454030` の戻り値（= `VerifyVersionInfoW` の戻り値が 0 のとき 1）は、`f_454090`（Delphi の `TOSVersion` 初期化に相当、推測）で **OS 名の文字列（FName）を 1 つ選ぶ分岐にだけ**使われる。TRUE なら "Windows 10"、FALSE なら "Windows Server 2019"（build 19045 の場合）。ほかの条件分岐は `f_454090` 内にない【事実】。この文字列がさらにどこまで影響するかは途中までしか追えていない（§3.5）。
4. 推奨（決定は PM）: **B を size == 0 だけに限定して実装し、決定として記録する**。併せて実機 Windows での 1 回の測定（D）を依頼し、結果が出たら ADR を更新する。理由と弱点は §4。

## 1. Microsoft Learn（全文取得して確認）

取得は `ocean_webfetch`（markdown）と `curl`（HTML 全文を `lift/out-vi2/src/` に保存）の両方。全ページ取得に成功（HTTP エラーなし）。

| ページ | URL | `dwOSVersionInfoSize` の出現 | `ERROR_INVALID_PARAMETER` の出現 | size 不正時の記述 |
|---|---|---|---|---|
| VerifyVersionInfoW | https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow | 1 回 | 0 | なし |
| RtlVerifyVersionInfo | https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlverifyversioninfo | 0 | 0（`STATUS_INVALID_PARAMETER` は 1 回） | なし |
| OSVERSIONINFOEXW | https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexw | 2 回（メンバー名と説明） | 0 | なし |
| VerSetConditionMask | https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask | 0 | 0 | なし |
| IsWindowsServer | https://learn.microsoft.com/en-us/windows/win32/api/versionhelpers/nf-versionhelpers-iswindowsserver | 0 | 0 | なし（本文 86 語） |
| Verifying the System Version | https://learn.microsoft.com/en-us/windows/win32/sysinfo/verifying-the-system-version | 2 回（例のコード） | 0 | なし |
| Version Helper functions | https://learn.microsoft.com/en-us/windows/win32/sysinfo/version-helper-apis | （size への言及なし） | 0 | なし |

出現回数は `grep -o` の件数（`curl` で保存した HTML に対して）。

逐語の引用【事実】:

- VerifyVersionInfoW, Parameters, `lpVersionInformation`:
  > You must set the **dwOSVersionInfoSize** member of this structure to `sizeof(OSVERSIONINFOEX)`. You must also specify valid data for the members indicated by *dwTypeMask*. The function ignores structure members for which the corresponding *dwTypeMask* bit is not set.
- VerifyVersionInfoW, Return value:
  > If the currently running operating system satisfies the specified requirements, the return value is a nonzero value.
  > If the current system does not satisfy the requirements, the return value is zero and GetLastError returns ERROR_OLD_WIN_VERSION.
  > If the function fails, the return value is zero and GetLastError returns an error code other than ERROR_OLD_WIN_VERSION.
  size との関係は書かれていない。「fails」がどの条件を指すかも書かれていない。
- OSVERSIONINFOEXW, Members:
  > The size of this data structure, in bytes. Set this member to `sizeof(OSVERSIONINFOEX)`.
- RtlVerifyVersionInfo, Return value（表）: `STATUS_SUCCESS`「The specified version matches the currently running version of the operating system.」、`STATUS_INVALID_PARAMETER`「The input parameters are not valid.」、`STATUS_REVISION_MISMATCH`「The specified version does not match the currently running version of the operating system.」。どの入力が「not valid」かは書かれていない。
- IsWindowsServer: 「Indicates if the current OS is a Windows Server release.」「True if the current OS is a Windows Server version; otherwise, false.」
- Windows 10 の manifest 挙動（VerifyVersionInfoW の Remarks）:「**VerifyVersionInfo** returns false when called by applications that do not have a compatibility manifest for Windows 8.1 or Windows 10 if the *lpVersionInfo* parameter is set so that it specifies Windows 8.1 or Windows 10」。今回の呼び出しは VER_PRODUCT_TYPE だけで、バージョン番号を指定しないので、この節の対象外のはず【推測】。

補足（検索結果の抜粋のみ。ページ全文は取得していない）: RtlGetVersion のページ（WDK）には「A caller specifies which input structure is used by setting the dwOSVersionInfoSize member of the structure」とある。WDK の OSVERSIONINFOEXW ページ（取得済み）は「This member must be set before the structure is used with RtlGetVersion.」。いずれも RtlGetVersion の話で、VerifyVersionInfo の size 不正時の結果ではない。

結論【事実】: 公式ドキュメントは「size = 0 のとき TRUE か FALSE か、どの last error か」を定義していない。

## 2. 非公式だが一次に近い資料（非公式）

### 2.1 ReactOS（非公式。Windows の挙動の保証ではない）

- `sdk/lib/rtl/version.c` の `RtlVerifyVersionInfo`（https://raw.githubusercontent.com/reactos/reactos/master/sdk/lib/rtl/version.c ）【事実】:
  - 入力 `VersionInfo` の `dwOSVersionInfoSize` を**読まない**（ファイル内の `dwOSVersionInfoSize` 2 回は、ローカル変数 `Version.dwOSVersionInfoSize = sizeof(Version);` と FIXME の文字列）。
  - 冒頭の FIXME（逐語）:
    ```
    /* FIXME:
        - Check the following special case on Windows (various versions):
          o lp->wSuiteMask == 0 and ver.wSuiteMask != 0 and VER_AND/VER_OR
          o lp->dwOSVersionInfoSize != sizeof(OSVERSIONINFOEXW)
        - MSDN talks about some tests being impossible. Check what really happens.
     */
    ```
    → Windows 側で `dwOSVersionInfoSize != sizeof(OSVERSIONINFOEXW)` のときに特別な挙動があるかを、ReactOS の作者が**確認していない**（確認すべき項目として残している）。
  - 戻り値: `!TypeMask || !ConditionMask` → `STATUS_INVALID_PARAMETER`、不一致 → `STATUS_REVISION_MISMATCH`、全部一致 → `STATUS_SUCCESS`。
- `dll/win32/kernel32/client/version.c` の `VerifyVersionInfoW`（https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/kernel32/client/version.c ）【事実】: `RtlVerifyVersionInfo` を呼ぶだけ。`STATUS_INVALID_PARAMETER` → `SetLastError(ERROR_BAD_ARGUMENTS); return FALSE;`、`STATUS_REVISION_MISMATCH` → `SetLastError(ERROR_OLD_WIN_VERSION); return FALSE;`、それ以外 → `TRUE`。size の検査なし。
- ReactOS の `versionhelpers.h` の `IsWindowsServer`（https://raw.githubusercontent.com/reactos/reactos/master/sdk/include/psdk/versionhelpers.h ）: `OSVERSIONINFOEXW osvi = { sizeof(osvi), 0, 0, 0, 0, {0}, 0, 0, 0, VER_NT_WORKSTATION };` … `return VerifyVersionInfoW(&osvi, VER_PRODUCT_TYPE, dwlConditionMask) == FALSE;`【事実】。f_454030 と同じ形のテストだが size を設定する。

### 2.2 Wine（非公式）

- `dlls/ntdll/version.c` の `RtlVerifyVersionInfo`（https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/ntdll/version.c ）【事実】: 入力 `info` の `dwOSVersionInfoSize` を読まない（`dwOSVersionInfoSize` の 2 回はどちらも内部の `ver.dwOSVersionInfoSize = sizeof(ver)` と `RtlGetVersion` 内の比較）。`!(dwTypeMask && dwlConditionMask)` → `STATUS_INVALID_PARAMETER`。product type は `version_compare_values(ver.wProductType, info->wProductType, dwlConditionMask >> 7*3 & 0x07)`。
- `dlls/kernel32/version.c` の `VerifyVersionInfoW`（https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/kernel32/version.c ）【事実】:
  ```
  ver.dwOSVersionInfoSize = sizeof(ver);
  if (!GetVersionExW((OSVERSIONINFOW*)&ver)) return FALSE;
  if (!dwTypeMask || !dwlConditionMask) { SetLastError(ERROR_BAD_ARGUMENTS); return FALSE; }
  ```
  入力 `info->dwOSVersionInfoSize` は読まない。不一致は `ERROR_OLD_WIN_VERSION`。`ERROR_INVALID_PARAMETER` を使う箇所はない（`ERROR_BAD_ARGUMENTS`）。
  なお、`dlls/kernelbase/version.c`（全 1857 行のうち 1539 行まで取得、残りは保存ファイルで確認）に `VerifyVersionInfoW` はない（grep 0 件）。`dlls/kernelbase/sysinfo.c` は 404。
- Wine の単体テスト `dlls/kernel32/tests/version.c`（https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/kernel32/tests/version.c ）【事実】:
  ```
  /* test bad dwOSVersionInfoSize */
  info.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEXA);
  GetVersionExA((OSVERSIONINFOA *)&info);
  info.dwOSVersionInfoSize = 0;
  ret = VerifyVersionInfoA(&info, VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR | VER_SERVICEPACKMINOR,
      VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL));
  ok(ret, "VerifyVersionInfoA failed with error %ld\n", GetLastError());
  ```
  - 履歴（GitHub API で取得）: コミット 60a5ee6560（2018-02-25）の時点では `ok(ret || broken(!ret) /* some win2k */, ...)` だったものを、同日の e019097556「Remove workarounds for older versions」と 60a5ee6560 の差分で `ok(ret, ...)` に変えた（旧版では 2000 の一部で FALSE になる環境があった、という注記）。
  - Wine のテストは実機 Windows でも走らせる前提で書かれており（`broken()` は「実機 Windows の既知の異常」を表す）、`ok(ret, ...)` は「実機でも size = 0 で TRUE」を期待している、と読める【推測】。
  - `https://test.winehq.org/data/tests/kernel32:version.html` を取得できた（直近ビルドの Win10/Win11 の列がすべて 0。意味は「失敗数」と読んだ【推測】。列の定義は確認していない）。個別ビルドのページ（`/data/840a939ee526/`）は 404。
  - 弱点: このテストは **VerifyVersionInfoA** を呼ぶ。Wine の `VerifyVersionInfoA` は内部で `verW.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEXW)` を設定してから W を呼ぶ。実機 Windows の A 版が同じ動きをするか（= W 版に 0 が渡るか）は**確認できていない**【未確認】。ReactOS の `VerifyVersionInfoA`（`versionansi.c`）も `viex.dwOSVersionInfoSize = sizeof(viex)` を設定してから W を呼ぶ【事実】。テストの題材は VER_MAJORVERSION 系で、VER_PRODUCT_TYPE ではない。

### 2.3 Wine と ReactOS の合意点と限界

Wine（2018 年の履歴を持つテストで実機を意識）、ReactOS（FIXME で未確認と自認）とも、「size を読まない」実装。Wine はテストで実機の結果を確かめている可能性がある（A 版経由）。いずれも Windows 10 の `kernelbase!VerifyVersionInfoW` のコードそのものではない【事実】。

### 2.4 Windows SDK の versionhelpers.h（実物は未取得）

- 実物の SDK ヘッダーはこのホストにない。代わりに mingw-w64 版（https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/versionhelpers.h ）を取得した【事実】: `IsWindowsServer` は `OSVERSIONINFOEXW vi = {sizeof(vi),0,0,0,0,{0},0,0,0,VER_NT_WORKSTATION}; return !VerifyVersionInfoW(&vi, VER_PRODUCT_TYPE, VerSetConditionMask(0, VER_PRODUCT_TYPE, VER_EQUAL));`。
- Windows 8.1 SDK の `VersionHelpers.h` は、個人ブログ（https://greenfishblog.tistory.com/208 、第三者の引用で、SDK 原本ではない）が全文を載せていた。`IsWindowsServer` は `OSVERSIONINFOEXW osvi = { sizeof(osvi), 0, 0, 0, 0, {0}, 0, 0, 0, VER_NT_WORKSTATION }; ... return !VerifyVersionInfoW(&osvi, VER_PRODUCT_TYPE, dwlConditionMask);`。
- 3 実装とも**必ず size を設定する**。つまり公式ヘッダーは size を設定する前提で書かれており、size = 0 の呼び出しは SDK の通常経路に出てこない【事実】。ゲストの `f_454030` は SDK の `IsWindowsServer` と同じ形（`wProductType = VER_NT_WORKSTATION`、VER_PRODUCT_TYPE、VER_EQUAL、結果の否定）で、size の設定だけが欠けている【事実: 命令列は §3】。

### 2.5 NT5 系の流出ソース（検索結果の抜粋のみ。非公式）

- 出典: `https://dl.malwarewatch.org/software/features/ntvdmx64/build/nt5docs/d0/d6/version_8c.html`（ntvdmx64 の nt5 ドキュメント。`RtlVerifyVersionInfo` のソース抜粋がある。Windows 2000 / XP 世代の ntos/rtl/version.c と思われる【推測】）。
- **このページは直接取得できなかった**（`ocean_webfetch` が Transport error。`_source.html` も同じ）。内容は検索エンジンの抜粋に出た 219〜394 行（一部の行は抜粋で欠落または文字化け）。
- 抜粋で読めた範囲【事実（抜粋に対して）】: Return Value は「STATUS_INVALID_PARAMETER if the parameters are not valid. STATUS_REVISION_MISMATCH if the versions don't match. STATUS_SUCCESS if the versions match.」。関数の先頭は `if (TypeMask == 0) { return STATUS_INVALID_PARAMETER; }` の後に `RtlZeroMemory(&CurrVersion, ...); CurrVersion.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEXW); Status = RtlGetVersion(...)`、続いて `VersionInfo->wSuiteMask != 0` の処理。入口付近（243〜254 行）に `VersionInfo->dwOSVersionInfoSize` の検査は**見えない**。255〜286 行は抜粋が欠けて読めない【未確認】。
- Windows 10 の ntdll とは世代が違う。参考にとどめる。

## 3. ゲスト側の静的解析（`objdump -d -M intel`、`binaries/win64/FLEngine_x64.dll`）

### 3.1 f_454030（0x454030〜0x454080）【事実。再確認した】

```
454030 push rbp
454031 sub rsp,0x140
454038 mov rbp,rsp
45403b lea rcx,[rbp+0x24]          ; 構造体
45403f mov edx,0x11c               ; memset の個数
454044 xor r8,r8
454047 call 0x40d210               ; memset(構造体, 0x11c, 0)
45404c mov BYTE PTR [rbp+0x13e],1  ; 構造体+282 = wProductType = VER_NT_WORKSTATION
454053 xor ecx,ecx
454055 mov edx,0x80
45405a mov r8b,1
45405d call 0x423cd0               ; VerSetConditionMask(0, VER_PRODUCT_TYPE, VER_EQUAL)
454062 lea rcx,[rbp+0x24]
454066 mov edx,0x80
45406b mov r8,rax
45406e call 0x423cc0               ; VerifyVersionInfoW
454073 test eax,eax
454075 sete al                     ; 戻り値 = (VerifyVersionInfoW の戻り値 == 0)
...
454080 ret
```

- 0x423cc0 = `jmp [0x1971620]`、0x423cd0 = `jmp [0x1971658]`。PE の import テーブルを読むと、0x1971620 = kernel32.dll!VerifyVersionInfoW、0x1971658 = kernel32.dll!VerSetConditionMask【事実】（同じ import 走査で 0x1971618 = GetVersionExW も確認）。
- 構造体の size（+0）に書く命令はない（0039 で確認済み）。
- 意味: **戻り値 1 = 「VerifyVersionInfoW が 0 を返した」 = 「ワークステーションではない = サーバー」**（SDK の `IsWindowsServer` と同じ向き）。

### 3.2 呼び出し元 f_454090（0x454090〜0x4547cd）

f_454030 の呼び出しは 2 か所、0x454565 と 0x4546e5【事実】（`dis-all.txt` で `call 0x454030` を検索して 2 件）。

f_454090 の前半【事実。命令を読んだ】:

- `[rbp+0x54]` を 0x11c バイト 0 にして `[rbp+0x54] = 0x11c` を書き、`call 0x423800`（`jmp [0x1971618]` = GetVersionExW）。結果の `dwMajorVersion`、`dwMinorVersion`、`dwBuildNumber`、`wServicePackMajor`、`wServicePackMinor` を、グローバル 0x15ec584（major）、0x15ec588（minor）、0x15ec580（build）、0x15ec59c、0x15ec5a0 に保存する。
- `f_454980(5, 1)`（major・minor が 5.1 以上か。内部の cmp と `jg`/`jge` の読み）が真なら 0x4236f0 を呼び、`wProcessorArchitecture == 9`（AMD64）を 0x15ec57c に保存する（`[0x13adf30 + cl]` の 1 バイト表を引く）。0x4236f0 の IAT スロット（0x1976390）は、このスクリプトの import 走査では名前を解決できなかった【未確認。0039/0014 以前の記録では `f_4236f0` として出てくる】。
- 6.1 より新しい OS（major > 6、または major == 6 && minor > 1。0x454164〜0x454181）は、`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` の `CurrentMajorVersionNumber`、`CurrentMinorVersionNumber`、`CurrentBuildNumber` を読んで 0x15ec584/588/580 を上書きする（文字列は 0x45485c 以降の UTF-16 を目視で確認。`RegOpenKeyExW`/`RegQueryValueExW`/`RegCloseKey` は IAT で確認）。`RegOpenKeyExW` が失敗（非 0）ならスキップ（0x4541ba の `jne 0x454516`）。

分岐の本体（0x454516〜0x4547c0）【事実】: `eax = major - 5` を 0（major 5）、1（major 6）、4（major 10）で切り分ける。

- **major 10（0x454550）**: minor が 0 のとき（`[0x15ec588] - 1 == 0xffffffff`）だけ 0x454565 で `f_454030` を呼ぶ。`test al,al`:
  - al ≠ 0（f_454030 が 1 = サーバー）→ 0x45456e: build（0x15ec580）が `0x4f7c`（20348）以上 → `[0x15e0368]`、`0x4563`（17763）以上 → `[0x15e4268]`、それ未満 → `[0x15e40a0]` を FName（0x15ec590）に代入（`call 0x41c820`、Delphi の UnicodeString 代入）。
  - al = 0（f_454030 が 0 = ワークステーション）→ 0x4545ce: build が `0x55f0`（22000）以上 → `[0x15e20d0]`、それ未満 → `[0x15e3bc0]`。
- **major 6 minor 3**（0x4546e2〜0x454719）: 0x4546e5 で `f_454030`。al ≠ 0 → `[0x15e43e8]`、al = 0 → `[0x15e41a8]`。
- 他の major/minor の分岐は f_454030 を呼ばない。

文字列定数の解読【事実】: 各スロット（`[0x15e....]`）は Delphi の「リソース文字列」の記述子（`98 c2 5e 01 ... <id 2 バイト> ff ...`）を指す。id は `.rsrc` の RT_STRING テーブルを引いて解読した（`ff00` 〜 `ff1f` の範囲。同じスクリプトで `.rsrc` を走査）:

| スロット | リソース id | 文字列 | 使われる条件 |
|---|---|---|---|
| 0x15e0368 | 0xff02 | Windows Server 2022 | major 10、f_454030 = 1、build ≥ 20348 |
| 0x15e4268 | 0xff01 | Windows Server 2019 | major 10、f_454030 = 1、17763 ≤ build < 20348 |
| 0x15e40a0 | 0xff00 | Windows Server 2016 | major 10、f_454030 = 1、build < 17763 |
| 0x15e20d0 | 0xff06 | Windows 11 | major 10、f_454030 = 0、build ≥ 22000 |
| 0x15e3bc0 | 0xff05 | Windows 10 | major 10、f_454030 = 0、build < 22000 |
| 0x15e43e8 | 0xff1f | Windows Server 2012 R2 | major 6.3、f_454030 = 1 |
| 0x15e41a8 | 0xff04 | Windows 8.1 | major 6.3、f_454030 = 0 |

### 3.3 VerifyVersionInfoW の戻り値ごとの結果（lift 環境、エミュレート OS 10.0.19045、major 10 minor 0）

| VerifyVersionInfoW | f_454030 | f_454090 が選ぶ FName | 備考 |
|---|---|---|---|
| 非 0（TRUE） | 0 | **"Windows 10"**（build 19045 < 22000） | エミュレート OS（`wProductType` = 1、0014）と整合する |
| 0（FALSE） | 1 | **"Windows Server 2019"**（17763 ≤ 19045 < 20348） | エミュレート OS の定義と食い違う名前になる |

build は `GetVersionExW`（エミュレート値 19045）か、`fle_reg` がレジストリ値を返す場合はその値【推測: harness のレジストリ応答は未確認】。last error（ERROR_OLD_WIN_VERSION か別の値か）はゲストが読まない（f_454030 は `GetLastError` を呼ばず、f_454090 も呼ばない）【事実】。つまり **ゲストにとって「FALSE の理由が何か」は影響しない。TRUE か FALSE かだけが効く**。

### 3.4 ゲストのソースの推測【推測】

- f_454030 は Delphi の `IsWindowsServer` 相当（`FillChar` 相当で構造体を 0、`wProductType := VER_NT_WORKSTATION`、VER_PRODUCT_TYPE、VER_EQUAL、`not VerifyVersionInfo(...)`）に見える。`OSVERSIONINFOEXW` の size 設定を落としている。
- f_454090 は Delphi の `TOSVersion`（`System.SysUtils`）の初期化に相当する形（major/minor/build/SP の保存、64 ビット判定、レジストリ `CurrentVersion` の参照、`'Windows 10'`/`'Windows Server 2019'` などのリソース文字列の選択）。Delphi RTL の実ソースは確認していない。
- 実機でこの呼び出しが FALSE を返すなら、FL Studio は Windows 10 でも "Windows Server 2019" と名乗る不具合を常に出すことになる。製品として目立ちにくい（テレメトリ向け文字列、§3.5）ので、実機が TRUE という根拠にはならない。弱い傍証にとどまる。

### 3.5 FName の使われ方（途中まで）

- FName（0x15ec590）の参照は、`dis-all.txt`（0x401000〜0x1370000）と、この調査で作った `lift/out-vi2/tmp/dis-tail.txt`（0x1370000〜0x13a5000、.text の終端まで）で、rip 相対の参照を検索した: f_454090 の代入（0x454516〜0x4547b2）、ユニット終了処理（0x438714、UnicodeString の解放）、**f_4549a0 の読み出し 2 か所（0x454a1c、0x454af8）**のみ。アドレスをデータとして持つ表（絶対アドレス 8 バイト）には出てこない【事実】。
- f_4549a0（0x4549a0〜0x454bc4）は、FName と major/minor/build と 64 ビットかの値を Delphi の `Format`（0x43dd60）に TVarRec の配列（型 0x11 = UnicodeString、0x0 = Integer）で渡し、整形した文字列を返す関数【事実: 命令】。`TOSVersion.ToString` 相当の形（推測）。
- f_4549a0 の直接呼び出しは 1 か所（0x7a62fb、f_7a62d0 の中）。f_7a62d0 は、major 10 のとき文字列を固定名で上書きせずに id（0x13 または 0x14）を書いて終わる（0x7a632c〜0x7a6356 → 0x7a671b）【事実: 命令】。つまり major 10 では FName を含む整形文字列がそのまま残る【推測】。
- f_7a62d0 は f_7a6c80（0x7a6d01 で呼ぶ）から呼ばれ、結果を `[rax+8]` に格納する（0x7a6d15 の `call 0x4133f0`）。この f_7a6c80 が 0x15f6b38 のオブジェクトを初期化する（0x7a7380 の `call 0x7a6c80`、rcx = 0x15f6b38）。`[0x15e4728]` はこのオブジェクトを指すグローバル（0x15e4728 の値が 0x15f6b38 を指す）で、0x1250260 付近の関数と 0x11f083e、0x115e940 などが `[0x15e4728]+8`、`+0x28` を読む。0x1250260 付近で使われるキー文字列は UTF-16 で `flstudio_language`、`os_name`、`os_version`、`os_language`（0x12506a4〜0x1250714）【事実】。
- ここから先は未確認【未確認】: `[0x15e4728]+8` が `os_name` に入るのか、`+0x28` が `os_version` かの対応、これらが送信・保存されるか、FL Studio の動作（機能の有効・無効、ライセンス確認など）の分岐に使われるか。**ここまでの静的解析では、FName を使って機能の分岐をする箇所は見つかっていない**が、全参照を追い切っていない。

### 3.6 「この呼び出しの結果が FLEngine の挙動を実質的に分けるか」

- 制御フロー: f_454090 の中では **OS 名の文字列の選択だけ**を分ける【事実】。
- 観測できる差: OS 名の文字列が "Windows 10" か "Windows Server 2019" になる（build 19045 の場合）。この文字列が最終的にどの出力へ出るかは未確認【未確認】。
- したがって「結果に影響しない」とは言えない（文字列は変わる）。ただし機能分岐に使われるという証拠もない。

## 4. 選択肢の比較

前提（共通）: エミュレート OS は 0014 の決定どおり `wProductType = 1`（ワークステーション）。

| 案 | 内容 | 根拠の強さ | ゲストの観測結果 | ADR-0007/0009/0010 との関係 | リスク |
|---|---|---|---|---|---|
| **A** | 現状維持。size ≠ 284 は名前付き abort | 仕様（Learn）が定義しないので、abort は「仕様が決めないケースで止まる」方針どおり | harness は 0x45406e で止まり続け、これ以降の実行は得られない | 最も忠実 | 進捗が止まる。止まった先の次の停止（GetLocaleInfoW など）も調べられない。実機が TRUE を返すなら、止めているのは実機にない障害 |
| **B** | 非公式の実装（ReactOS / Wine / NT5 抜粋）に従い、size を読まずに比較を進める。決定として記録 | 3 実装が一致（§2）。Wine のテストが size = 0 で TRUE を期待（A 版経由、実機走行が前提と読める）。弱点: ReactOS 自身が FIXME で未確認、Wine のテストは A 版（Wine は A→W で size を補う）、NT5 抜粋は Windows 2000 世代で一部欠落、Windows 10 の kernelbase の実装そのものではない | VerifyVersionInfoW = TRUE → f_454030 = 0 → FName = "Windows 10"（エミュレート OS と整合） | **仕様にない挙動への既定値**に当たる。ADR-0007 の「fail-soft の続行は進捗と認めない」の精神に触れる。決定として明文化し、根拠の弱点と限定範囲を書くことで、ADR 上は「出典付きの決定」に格上げできる | 実機が FALSE（ERROR_INVALID_PARAMETER など）を返す場合、FName が誤る（"Windows 10" と出るべきでない状況で出る）。ただし影響は §3 のとおり文字列のみ（機能分岐は未発見） |
| **B-narrow**（B を限定） | size == 0 のときだけ「size を検査しない」と扱う。それ以外の不正値（276 など）は従来どおり abort | B と同じ。さらに「ゲストが実際に出している値だけ」に限定するので、仕様外の既定値の範囲が最小 | B と同じ | B より ADR の趣旨に近い（未観測の入力はこれまでどおり止まる） | 他の不正値が出たとき再び止まる（それは意図どおり） |
| **C** | この結果は挙動に影響しないとして、結果を問わず進める | 弱い。§3.6 のとおり「影響しない」とは言えない（文字列が変わる）。文字列の行き先も途中までしか追えていない | 不定（TRUE/FALSE のどちらにするかを結局決める必要がある） | 「影響しない」の証明が必要。今はできていない | 影響がテレメトリ以外に及ぶ場合を見逃す |
| **D** | 実機 Windows で測定してから決める（A のまま待つ） | 最強（一次の事実） | 測定後に決まる | 最も忠実 | 実機（Windows 10 以降、できれば manifest なしの exe）が必要。このホストは Linux |

### 推奨（決定は PM）

**B-narrow ＋ D の依頼**。

1. ADR を 1 本追加する（例: 0040）: 「size == 0 の VerifyVersionInfoW は size を検査せず、通常どおり比較する。出典は ReactOS / Wine の実装と Wine のテストで、Windows 実機の仕様ではない。Learn は結果を定義しない。影響範囲は f_454030 → f_454090 の OS 名文字列だけ（FLEngine の機能分岐は未発見）。実機測定で否定されたら撤回する」。
2. `fle_osver.c` の size 検査を「0 のときは検査しない、それ以外の 284 以外は abort」に狭める。size 0 のテスト（0039 が追加した `size-0-as-f_454030` は「abort する」期待値なので、期待値を「戻り値 1（TRUE）」に変える必要がある）。この変更は本調査では行っていない。
3. D: 実機で次の 10 行程度のプログラムを走らせ、戻り値と `GetLastError` を 1 回記録する（dwOSVersionInfoSize = 0、wProductType = 1、TypeMask = 0x80、`VerSetConditionMask(0, 0x80, 1)`。manifest なし、Windows 10 以降）。結果が FALSE なら B-narrow は撤回し、その結果（戻り値、last error）に合わせる。

理由: (a) 3 つの独立した実装がそろって size を読まず、Wine のテストは実機での TRUE を期待している（A 版経由という弱点つき）。(b) TRUE がエミュレート OS の定義（ワークステーション）と整合し、FALSE だと「Windows Server 2019」という矛盾した名前が出る。(c) A のままだと、実機にない障害で以降の解析が止まる。弱点は (1) 実機の結果を直接見ていないこと、(2) 影響が文字列だけという主張が途中までの静的解析に基づくことで、どちらも ADR に書く。

A を選ぶなら、停止の次に進めないので、D を先に済ませるのが最短。

## 5. 取得に失敗・不完全だったもの（記録）

- `https://gitlab.winehq.org/search?...`: Anubis のボット確認ページが返り、検索不可。
- `https://dl.malwarewatch.org/.../version_8c.html` と `version_8c_source.html`: Transport error（2 回）。§2.5 は検索エンジンの抜粋のみ。
- `https://stackoverflow.com/questions/34235151/get-windows-version`: 403。
- `https://api.github.com/search/code?...`: 401（認証なしは不可）。`search/commits` は結果 0 件。
- `https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/kernelbase/sysinfo.c`: 404。`dlls/kernelbase/tests/version.c`: 404（本文 14 バイト）。
- `https://test.winehq.org/data/840a939ee526/`: 404（一覧ページ `kernel32:version.html` は取得できた）。
- Windows SDK の実物の `versionhelpers.h`（`C:\Program Files (x86)\Windows Kits\...`）: このホストにない。mingw-w64 版と、第三者ブログが引用した 8.1 版で代替。
- Windows Research Kernel の ntos/rtl/version.c: 取得できていない（WRK は Windows Server 2003 SP1 世代で、公開リポジトリの URL を確認できなかった）。
- 実機 Windows での結果: **取得不可**（ホストは Linux）。

## 6. 参照した既存文書

- `docs/decisions/0039-versioninfo-stop.md`、`docs/investigations/versioninfo-stop-run.md`: 停止の事実（size = 0、R2 = 0x80、R8 = 0x200000）は再確認で一致。
- `docs/decisions/0014-emulated-os.md`: エミュレート OS（10.0.19045、`wProductType` = 1）。
- `lift/runtime/fle_osver.c` の `fle_osver_check`（93〜94 行の size 検査）を読んだ。変更していない。
- 0039 の「ReactOS は FIXME で未確認」「Wine も size を読まない」は、今回の取得で再確認。新しい情報は、(1) Wine のテストが size = 0 で TRUE を期待していること（履歴つき）、(2) f_454030 の戻り値の使われ方、(3) NT5 抜粋。

## 7. 用いたコマンド（再現用）

- 逆アセンブル: `objdump -d -M intel --start-address=<a> --stop-address=<b> binaries/win64/FLEngine_x64.dll`（0x454030、0x454090、0x454980、0x4549a0、0x7a62d0、0x7a6c80、0x44ad10 など）。
- 参照の検索: `lift/out-vi/dis-all.txt`（既存。0x401000〜0x1370000）と `lift/out-vi2/tmp/dis-tail.txt`（今回作成。0x1370000〜0x13a5000）に対する `grep -n "# 0x15ec590$"` など。
- PE の解析: python3 で import テーブル（IAT スロットの名前）と `.rsrc` の RT_STRING テーブル（リソース id の文字列）を読んだ。標準ライブラリのみ。
- 取得: `curl -sS -L --max-time 60`（`lift/out-vi2/src/` へ保存）と `ocean_webfetch`。

## 8. 保存した取得物（`lift/out-vi2/src/`）

`learn-verifyversioninfow.md`、`learn-rtlverifyversioninfo.md`、`learn-osversioninfoexw.md`、`learn-versetconditionmask.md`、`learn-iswindowsserver.md`、`learn-verifying-system-version.md`（拡張子は `.md` だが中身は HTML）、`reactos-rtl-version.c`、`reactos-k32-version.c`、`reactos-k32-versionansi.c`、`reactos-versionhelpers.h`、`reactos-winetests-kernel32-version.c`、`wine-ntdll-version.c`、`wine-kernel32-version.c`、`wine-kernel32-tests-version.c`、`mingw-versionhelpers.h`。ほかに探索で取得した `wine-ntdll-rtl.c`、`wine-ntdll-info.c`、`wine-ntdll-om.c`、`wine-kernelbase-version.c`（404 の 14 バイト）、`wine-kernel32-sync.c`（`RtlVerifyVersionInfo`、`VerifyVersionInfo` を含まないことの確認用）。

## 9. 制約の遵守

`rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない。`lift/crates` と `lift/runtime` は変更していない。2 GB 級の中間ファイルなし（`dis-tail.txt` は約 74907 行）。権限拒否はなかった。
