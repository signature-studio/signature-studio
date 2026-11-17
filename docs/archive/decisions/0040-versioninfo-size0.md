# 0040: VerifyVersionInfoW の dwOSVersionInfoSize = 0 は検査せず比較を進める（B-narrow、仮の決定）

状態: PM が決定（案 B-narrow）。実装は作業者。**仮の決定**: 根拠は非公式の出典（ReactOS / Wine の実装と Wine のテスト）だけで、**実機の Windows の結果は未確認**。実機で FALSE（または別の結果）と分かった場合は撤回する（撤回条件を参照）。
関連: `0039-versioninfo-stop.md`（停止の分析。0039 には追記していない。この決定が 0039 の「名前付き abort のままにする」判断のうち、size 0 の部分だけを置き換える）、`0014-emulated-os.md`（エミュレート OS）、`docs/investigations/versioninfo-size0-spec.md`（調査）、`docs/investigations/versioninfo-size0-run.md`（実行記録）。

## 決定

- `fle_osver.c` の `VerifyVersionInfoW` は、`dwOSVersionInfoSize` が **0 のときに限り** size を検査せず、通常どおり比較を進める。
- 0 でも 284（`sizeof(OSVERSIONINFOEXW)`）でもない size（例: 1、0x114 = OSVERSIONINFOW、0x11d）は、従来どおり名前付き abort（`fle: VerifyVersionInfoW: dwOSVersionInfoSize is neither 0 nor sizeof(OSVERSIONINFOEXW) ...`）。284 は従来どおり比較に進む。
- 比較に使う構造体メンバは、size 0 でも `dwTypeMask` が示すものだけを読む（`dwTypeMask` のビットが立っていないメンバーは読まない。構造体は 284 バイトあるものとして扱う。ゲストの f_454030 は 0x11c バイトを memset で確保・0 埋めしている。0039）。
- VER_PRODUCT_TYPE（0x80）+ VER_EQUAL の比較は、エミュレート OS（0014: Windows 10 22H2、ProductType = VER_NT_WORKSTATION = 1）に対して行う。結果（TRUE/FALSE）はコードの比較で決まり、固定していない。ゲストが渡す wProductType = 1 なので TRUE（戻り値 1）になる。

## 根拠（すべて非公式の出典）

- Microsoft Learn（VerifyVersionInfoW、RtlVerifyVersionInfo、OSVERSIONINFOEXW ほか）は、size が 0 または不正のときの戻り値、GetLastError を**定義していない**（`versioninfo-size0-spec.md` §1）。
- ReactOS `sdk/lib/rtl/version.c` の `RtlVerifyVersionInfo`: 入力の `dwOSVersionInfoSize` を読まない。冒頭の FIXME は、size が `sizeof(OSVERSIONINFOEXW)` と違うときの Windows の挙動を未確認として残している。https://raw.githubusercontent.com/reactos/reactos/master/sdk/lib/rtl/version.c 、`dll/win32/kernel32/client/version.c`: https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/kernel32/client/version.c
- Wine `dlls/ntdll/version.c`、`dlls/kernel32/version.c`: 入力の size を読まない。https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/ntdll/version.c 、https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/kernel32/version.c
- Wine の単体テスト `dlls/kernel32/tests/version.c`（"test bad dwOSVersionInfoSize"）は、size = 0 で `VerifyVersionInfoA` が成功することを `ok(ret, ...)` で期待する。https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/kernel32/tests/version.c
- 結果が TRUE ならゲスト（f_454030 → f_454090）は OS 名に "Windows 10" を選ぶ。エミュレート OS（ワークステーション）と整合する。FALSE なら "Windows Server 2019" を選び、エミュレート OS の定義と食い違う（`versioninfo-size0-spec.md` §3.3）。

## 非公式であること、実機未確認であること（弱点）

- ReactOS も Wine も Windows 10 の `kernelbase!VerifyVersionInfoW` そのものではない。ReactOS は自身の FIXME で未確認としている。
- Wine のテストが呼ぶのは **VerifyVersionInfoA** で、Wine の A 版は内部で size を設定してから W 版を呼ぶ。実機 Windows の A 版が同じ動きをするか（W 版に 0 が渡るか）は確認できていない。テストの題材も VER_MAJORVERSION 系で、VER_PRODUCT_TYPE ではない。
- Windows SDK の `versionhelpers.h`（`IsWindowsServer`）は必ず size を設定する。ゲストの f_454030 は、その形で size の設定だけが欠けている。size = 0 の呼び出しは SDK の通常経路に出てこない。
- 実機 Windows での測定は行えていない（ホストは Linux）。ホスト上の結果は、この決定の正しさを示さない。

## 撤回条件

実機 Windows（10 以降、manifest なしの exe）で、`dwOSVersionInfoSize = 0`、`wProductType = 1`、`dwTypeMask = 0x80`、`VerSetConditionMask(0, 0x80, VER_EQUAL)` の `VerifyVersionInfoW` を 1 回呼び、戻り値と `GetLastError` を記録する。**戻り値が FALSE、または TRUE 以外の結果だった場合、この決定を撤回する**（`fle_osver.c` を 0039 の状態へ戻すか、実測の結果に合わせる。FALSE のときゲストは "Windows Server 2019" を選ぶので、その場合のエミュレート OS の扱いは別途 PM が決める）。TRUE だった場合は、この決定の「仮」を外し、実測を出典として記録する。

## 影響

- harness（all-pdata）は VerifyVersionInfoW で止まらなくなり、先へ進む。
- ゲストが選ぶ OS 名の文字列は **"Windows 10"**（major 10、minor 0、build 19045 < 22000、f_454030 = 0）になる。制御フロー上の差は、f_454090 の OS 名文字列の選択だけ（機能分岐に使われる証拠は見つかっていない。ただし文字列の行き先は途中までしか追えていない。`versioninfo-size0-spec.md` §3.5〜§3.6）。
- 変更したファイル: `lift/runtime/fle_osver.c`（size の検査、メンバの読み取りを `dwTypeMask` が示すものに限定）。テスト: `lift/crates/fe-lift/tests/osver.rs`（`undefined_cases_abort` の `size-0-as-f_454030` を外し、size 1、0x11d、上位ビットが立った値の abort を追加。0x114 は既存の `bad-size`。`size_0_is_not_checked` を追加）。`tests/versioninfo.rs` は DLL の逆アセンブル検証で、挙動は変えず、冒頭の注記だけ更新。
- 0012（応答を固定値にしない）と 0014 の方針（1 つの OS 定数群に対して動く）は変えない。0007 の「fail-soft の続行は進捗と認めない」に対しては、仕様にない入力への既定値に当たるため、出典と限界をこの文書に明記し、範囲を size = 0 に限定した。
