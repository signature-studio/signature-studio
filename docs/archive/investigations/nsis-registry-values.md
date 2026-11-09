# インストーラが書くレジストリ値（NSIS スクリプトの読み）と仮想パスへの対応

作業日: 2026-10-28。対象: `flstudio_win64_26.1.7.5653.exe`（SHA-256 の先頭 16 桁 `04393e7946c7369a`。照合済み）。**読み取り専用**（インストーラは実行していない。ホストのレジストリ・ファイルには触れていない）。道具: `tools/nsis_script_dump.py`（新規。`tools/extract_nsis2.py` のヘッダ読み出し部を使い、NSIS のスクリプト＝エントリ表と文字列表を復号する。差分 `fruity3.diff` の `tools/nsis_script_dump.py`）。

凡例: **実測** = 道具の出力（このファイルの表）。**読み** = 実測からの解釈。**Learn/一次** = NSIS のソース（`Source/exehead/fileform.h`、`exehead/util.c`、`Source/build.cpp`。https://raw.githubusercontent.com/kichik/nsis/master/ で取得、2026-10-28）。**未確認**。

## 0. 結論（先に）

1. **このインストーラのスクリプトは `HKCU\Software\Image-Line\Shared\Paths` の `Shared data` を書かない**（実測: 下の WriteReg の全 77 入口に無い。文字列表の `Shared Data` は 1 つだけ、入口 402 の **ReadRegStr**）。入口 402 は、値が空でなければ `CreateDirectory` と `$_OUTDIR\DirectWave\...` へのファイルの配置に使う（読み。空なら入口 451 へ飛んで何もしない）。つまり `Shared data` は**インストール後にアプリが書く値**（または、インストーラ内のプラグイン `ILSetup.dll` が書く値。下の §4）で、NSIS スクリプトからは値が読めない。依頼の前提（「`Shared data` はインストーラが書く値で NSIS から読む」）は、**この版では成り立たない**。
2. 書くもののうち、エンジンの走りに関わりうるのは `Software\Image-Line\Shared\Paths` の `FL Studio`、`FL Studio engine`（HKLM と HKCU）、`Install path`（HKCU）。いずれも**文字列（REG_SZ）**で、データは `$INSTDIR\FL64.exe`、`$INSTDIR\FLEngine_x64.dll`、`$INSTDIR`。`FL64.exe` はこのうち `FL Studio engine`（と `Shared DLLs`）を、エンジンのファイルが exe の隣に無いときだけ読む（`host-engine-boundary.md` §5.1）。エンジンが `CreateFruityInstance` の最初の 35,000 入口で読んだレジストリは、`Shared data`、`Shared\General` の `FL Cloud`、`Registrations\Token`（HKCU／HKLM）、`FL Studio 26` の下の `DSTITF` など（実測: 前回の測定 §4.3 と `logs/w3/apis-distinct.txt`）。このうちインストーラのスクリプトが書くのは、`FL Studio 26` のキーへの `Settings cleared`（「設定の消去」を選んだときだけ。入口 930）だけで、エンジンが読んだ値ではない。
3. 型: 実測の WriteReg の第 7 オペランド（型）は `1` = REG_SZ（49 入口）、`4` = REG_DWORD（27 入口）、`3` = REG_BINARY（1 入口、データブロックのオフセット）。REG_EXPAND_SZ（2）は 0 件。

## 1. 手順（実測）

`python3 tools/nsis_script_dump.py <installer> --info` → ヘッダ（エントリ 59,904 個、1 エントリ 36 バイト = 整数 9 個、セクション 17）。`--registry` → レジストリ命令。`--range A B` → 命令の復号。`--var N` → 変数 N への代入。`--strings PAT` → 文字列表の検索。

オペコードの番号: このインストーラの番号は、現行の NSIS の `fileform.h` の列挙と、`EW_EXTRACTFILE`（20）までは同じで、`EW_DELETEFILE` 以降は **+2**（DELETEFILE 23、MESSAGEBOX 24、RMDIR 25、STRLEN 26、ASSIGNVAR 27、STRCMP 28、INTCMP 30、INTOP 31、PUSHPOP 33、REGISTERDLL 46、CREATESHORTCUT 47、WRITEINI 50、READINISTR 51、DELREG 52、WRITEREG 53、READREGSTR 54、FCLOSE 56、FOPEN 57、FPUTS 58）。根拠は、各番号の命令のオペランドの形（例: 53 は root／key／value 名／data を持ち、`Software\Microsoft\Windows\CurrentVersion\Uninstall\...` の `DisplayName` などを書く。27 は変数への代入の形。46 の大半は `System.dll` の `Call` と `UAC.dll` のプラグイン呼び出し）。58 より後は、確かめた 65（セクションのフラグ操作）と 69（GETOSINFO）だけに名前を付けた。**番号のずれの原因（21、22 に何があるか）は未確認**（どちらも使われていない）。エントリが 36 バイト（現行の `MAX_ENTRY_OFFSETS` は 6 で 28 バイト）なのも、版の差か改造かは未確認。復号に使う文字列の符号（変数 = `\x03` + 1 ワード、シェルフォルダ = `\x02` + 2 バイト（現在ユーザー／全ユーザーの CSIDL。`0x80` が立つと `HKLM\...\CurrentVersion` の `ProgramFilesDir`／`CommonFilesDir` を読む。`0x40` は 32 ビットビュー）、言語文字列 = `\x01`）は `util.c` の `GetNSISString` と `build.cpp` の `init_shellconstantvalues` を読んだ（Learn/一次）。

## 2. WriteReg の全一覧（実測。同一の組は入口番号をまとめた。`$v<N>` は名前の保存されない利用者変数、`$R0`…は組み込み）

| 入口(entry) | root | key | 値名 | 型 | データ(スクリプトの式) | 場所 |
|---|---|---|---|---|---|---|
| 61 | HKLM | `Software\Image-Line\Shared\Paths` | `$v35` | REG_SZ | `$v38` | function (entry 61) |
| 65 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v36` | `DisplayName` | REG_SZ | `$v36` | function (entry 65) |
| 66 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v36` | `HelpLink` | REG_SZ | `http://www.image-line.com` | function (entry 66) |
| 67 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v36` | `Publisher` | REG_SZ | `Image-Line` | function (entry 67) |
| 68 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v36` | `UninstallString` | REG_SZ | `$v38\uninstall.exe` | function (entry 68) |
| 94,96 | HKLM | `Software\SoftVoice\ImageLine` | `Fruityloops` | REG_DWORD | `0` | function (entry 94) |
| 97 | HKLM | `Software\WoW6432Node\SoftVoice\ImageLine` | `Fruityloops` | REG_DWORD | `0` | function (entry 97) |
| 101 | HKCU | `Software\SoftVoice\ImageLine` | `Fruityloops` | REG_DWORD | `0` | function (entry 101) |
| 128,131 | HKLM | `Software\Image-Line\Shared\Paths` | `FL Studio` | REG_SZ | `$R0\FL64.exe` | function (entry 128) |
| 129,132 | HKLM | `Software\Image-Line\Shared\Paths` | `FL Studio engine` | REG_SZ | `$R0\FLEngine_x64.dll` | function (entry 129) |
| 288,291 | HKCU | `Software\Image-Line\Shared\Paths` | `FL Studio` | REG_SZ | `$R0\$R2` | function (entry 288) |
| 289,292 | HKCU | `Software\Image-Line\Shared\Paths` | `FL Studio engine` | REG_SZ | `$R0\FLEngine_x64.dll` | function (entry 289) |
| 456,458,467 | HKCU | `$0` | `AllowProgramEditing` | REG_DWORD | `1` | function (entry 456) |
| 459,468 | HKCU | `$0` | `RenderingModeNotify` | REG_DWORD | `1` | function (entry 459) |
| 460,469 | HKCU | `$0` | `SendLoopPosition` | REG_DWORD | `1` | function (entry 460) |
| 461,470 | HKCU | `$0` | `RealMonoInputs` | REG_DWORD | `1` | function (entry 461) |
| 462 | HKCU | `$0` | `ProcessInactiveOutputs` | REG_DWORD | `1` | function (entry 462) |
| 463,472 | HKCU | `$0` | `CheckProgramOnDisplayChange` | REG_DWORD | `1` | function (entry 463) |
| 464,473,478,480,482,484 | HKCU | `$0` | `UseFixedBuffers` | REG_DWORD | `1` | function (entry 464) |
| 465,474 | HKCU | `$0` | `SendPBRange` | REG_DWORD | `0` | function (entry 465) |
| 471 | HKCU | `$0` | `ProcessInactiveOutputs` | REG_DWORD | `0` | function (entry 471) |
| 476 | HKCU | `$0` | `NoSecureCPUInCallback` | REG_DWORD | `1` | function (entry 476) |
| 486 | HKCU | `$0` | `InvalidateEditor` | REG_DWORD | `0` | function (entry 486) |
| 529 | HKCU | `Software\Classes\.$0` | `(既定)` | REG_SZ | `FL64.$0.$R3` | function (entry 529) |
| 530 | HKCU | `Software\Classes\.$0\OpenWithProgids` | `FL64.$0.$R3` | REG_BINARY | `<binary: data block offset 2408156>` | function (entry 530) |
| 531 | HKCU | `Software\Classes\FL64.$0.$R3\shell\open` | `FriendlyAppName` | REG_SZ | `$2 $R3 (64bit)` | function (entry 531) |
| 532 | HKCU | `Software\Classes\FL64.$0.$R3\shell\open\command` | `(既定)` | REG_SZ | `"$R0\$R2" "%1"` | function (entry 532) |
| 557 | HKLM | `Software\Image-Line\Capabilities\FL64.$R3` | `ApplicationDescription` | REG_SZ | `$2 $R3 (64bit) - The fastest way from your brain to your speakers.` | function (entry 557) |
| 558 | HKLM | `Software\Image-Line\Capabilities\FL64.$R3\FileAssociations` | `.flp` | REG_SZ | `FL64.flp.$R3` | function (entry 558) |
| 559 | HKLM | `Software\Image-Line\Capabilities\FL64.$R3\FileAssociations` | `.fsc` | REG_SZ | `FL64.fsc.$R3` | function (entry 559) |
| 560 | HKLM | `Software\Image-Line\Capabilities\FL64.$R3\FileAssociations` | `.fst` | REG_SZ | `FL64.fst.$R3` | function (entry 560) |
| 561 | HKLM | `Software\RegisteredApplications` | `Image-Line.FL64.$R3` | REG_SZ | `Software\Image-Line\Capabilities\FL64.$R3` | function (entry 561) |
| 562 | HKLM | `Software\Classes\FL64.flp.$R3` | `(既定)` | REG_SZ | `$2 project file` | function (entry 562) |
| 563 | HKLM | `Software\Classes\FL64.flp.$R3` | `FriendlyTypeName` | REG_SZ | `$2 project file` | function (entry 563) |
| 564 | HKLM | `Software\Classes\FL64.flp.$R3\DefaultIcon` | `(既定)` | REG_SZ | `$R0\$R2,0` | function (entry 564) |
| 565 | HKLM | `Software\Classes\FL64.fsc.$R3` | `(既定)` | REG_SZ | `$2 score file` | function (entry 565) |
| 566 | HKLM | `Software\Classes\FL64.fsc.$R3` | `FriendlyTypeName` | REG_SZ | `$2 score file` | function (entry 566) |
| 567 | HKLM | `Software\Classes\FL64.fsc.$R3\DefaultIcon` | `(既定)` | REG_SZ | `$R0\$R2,0` | function (entry 567) |
| 568 | HKLM | `Software\Classes\FL64.fst.$R3` | `(既定)` | REG_SZ | `$2 preset file` | function (entry 568) |
| 569 | HKLM | `Software\Classes\FL64.fst.$R3` | `FriendlyTypeName` | REG_SZ | `$2 preset file` | function (entry 569) |
| 570 | HKLM | `Software\Classes\FL64.fst.$R3\DefaultIcon` | `(既定)` | REG_SZ | `$R0\$R2,0` | function (entry 570) |
| 572 | HKCR | `flstudio` | `(既定)` | REG_SZ | `URL:FL Studio protocol` | function (entry 572) |
| 573 | HKCR | `flstudio` | `URL Protocol` | REG_SZ | `` | function (entry 573) |
| 574 | HKCR | `flstudio\shell\open\command` | `(既定)` | REG_SZ | `"$R0\$R2" "%1"` | function (entry 574) |
| 575 | HKCR | `flstudio\DefaultIcon` | `(既定)` | REG_SZ | `$R0\$R2,1` | function (entry 575) |
| 930 | HKCU | `Software\Image-Line\$v52` | `Settings cleared` | REG_SZ | `1` | section#0 'FL Studio (required)' |
| 58764 | HKLM | `Software\Image-Line\Shared\Paths` | `VST plugins` | REG_SZ | `$v41` | section#0 'FL Studio (required)' |
| 58795,58957 | HKLM | `Software\Image-Line\Shared\Paths` | `VST plugins` | REG_SZ | `$v43` | section#0 'FL Studio (required)' |
| 58927 | HKCU | `Software\Image-Line\Shared\Plugins\$2` | `VstDir64` | REG_SZ | `$v43` | section#9 'VSTi' |
| 59689 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `DisplayName` | REG_SZ | `$v40` | function (entry 59689) |
| 59690 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `DisplayIcon` | REG_SZ | `$INSTDIR\Data\System\icon.ico,0` | function (entry 59690) |
| 59691 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `DisplayVersion` | REG_SZ | `$v51.5653` | function (entry 59691) |
| 59692 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `Publisher` | REG_SZ | `Image-Line` | function (entry 59692) |
| 59693 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `URLInfoAbout` | REG_SZ | `$v53` | function (entry 59693) |
| 59694 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `HelpLink` | REG_SZ | `$v53` | function (entry 59694) |
| 59695 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `VersionMajor` | REG_SZ | `$v48` | function (entry 59695) |
| 59696 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `VersionMinor` | REG_SZ | `$v49` | function (entry 59696) |
| 59697 | HKLM | `Software\Microsoft\Windows\CurrentVersion\Uninstall\$v40` | `UninstallString` | REG_SZ | `$INSTDIR\uninstall.exe` | function (entry 59697) |
| 59820 | HKCU | `Software\Image-Line\Shared\Paths` | `$v54` | REG_SZ | `$INSTDIR` | function (entry 59820) |

読み（変数）: `$R0` = `$INSTDIR`（入口 58803、58839）、`$R2` = `FL64.exe`（58805）、`$v35` = `$v36` = `FL Studio ASIO`（13〜15。入口 61 はその ASIO ドライバーの `Paths\FL Studio ASIO` で、`$v38` はその ASIO の置き場所）、`$v40` = `FL Studio 2026`（59031）、`$v52` = `FL Studio 26`（59030）、`$v54` = `Install path`（59036）、`$v0`（456〜486）= `Software\Image-Line\Shared\Plugins\Fruity Wrapper\Plugins\...` の各キー（455〜485 の代入）、`$v48.$v49.$v50` = `26.1.7`（59032〜59035）。「場所」が `function` の行は、セクション外の関数（`.onInit` の呼ぶ補助関数）で、入口 61〜132 と 288〜292 は `CALL #103`（HKLM）と `CALL #70`（HKCU）の先。

## 3. ReadRegStr と DeleteReg（実測）

| 入口 | 読む先 | key | 値名 |
|---|---|---|---|
| 19 | → $1 ← HKCU | `Software\Image-Line\Shared\Paths` | `$v35` |
| 402 | → $0 ← HKCU | `Software\Image-Line\Shared\Paths` | `Shared Data` |
| 59090 | → $v41 ← HKLM | `Software\VST` | `VSTPluginsPath` |
| 59092 | → $v41 ← HKLM | `Software\Image-Line\Shared\Paths` | `VST plugins` |
| 59120 | → $v43 ← HKLM | `Software\VST` | `VSTPluginsPath` |
| 59122 | → $v43 ← HKLM | `Software\Image-Line\Shared\Paths` | `VST plugins` |
| 59323 | → $0 ← HKCU | `Software\Image-Line\Shared\Paths\` | `$v54` |
| 59333 | → $0 ← HKCU | `Software\Image-Line\Shared\Paths` | `Image-Line` |

DelReg: 入口 929（`HKCU\Software\Image-Line\$v52` = `FL Studio 26` のキーを消す。「設定の消去」の選択時）、931（`HKLM\Software\Image-Line\Shared\Plugins\Chrome`）。

## 4. インストーラ内のプラグイン `ILSetup.dll`（読み。逆アセンブルしていない）

スクリプトは `$PLUGINSDIR\ILSetup.dll`（x86、Delphi、1,206,784 B、SHA-256 `bee6fc43…cd25`）の関数 `StoreILPath`（入口 926）、`FindFLPath`、`MigrateSettings` などを呼ぶ。この DLL は `RegSetValueExW` と `RegCreateKeyExW` を import し、UTF-16 の文字列として `Software\Image-Line\Shared\Paths`、`FL Studio engine`、`FLEngine.dll`、`FL Studio`、`FL Studio Engine`、`FLEngine_x64.dll`、`Image-Line`、`Software\Image-Line\FL Studio 11/12/20/21\`、**`shared data`**（オフセット 1023100。直前は `FL Studio 21` のキー名）などを持つ。したがって**この DLL が `Shared data` を読む／書く可能性は残る**（`shared data` は旧版のキーからの移行（`MigrateSettings`）の文脈にあるように見えるが、確かめていない）。抽出したのは文字列と import だけで、関数の中身を読んでいない。値は NSIS スクリプトからは読めない。

## 5. パス変数の展開と仮想パス（読み）

| スクリプトの式 | 実機での展開（NSIS の規則から。読み） | このリポジトリの仮想パス（既存の決定。**選択**） |
|---|---|---|
| `$INSTDIR`（入口 59049、59789、59818 で代入） | 既定は入口 59332〜59346: `$R0` = HKCU `Paths\Image-Line` があればそれ、なければ `$PROGRAMFILES32{HKLM\...\CurrentVersion\ProgramFilesDir（32 ビットビュー）、無ければ $PROGRAMFILES64{...、無ければ C:\Program Files}}\Image-Line`、そこに `$3` = `FL Studio 2026`（59026〜59028）を付ける形。実機の既定は `C:\Program Files (x86)\Image-Line\FL Studio 2026` のはず（**読み。実機照合なし**）。ユーザーが選べる | `C:\Program Files\Image-Line\FL Studio 21`（`FLE_VIRTUAL_DLL_PATH` のディレクトリ部。決定 0018、0104、0110、0112。PM の値で、Windows の規則ではない） |
| `$R0\FL64.exe` | `$INSTDIR\FL64.exe` | `C:\Program Files\Image-Line\FL Studio 21\FL64.exe` |
| `$R0\FLEngine_x64.dll` | `$INSTDIR\FLEngine_x64.dll` | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll` |
| `$INSTDIR`（`Install path` の値、59820） | `$INSTDIR` | `C:\Program Files\Image-Line\FL Studio 21`（末尾の `\` なし） |
| `$PROGRAMFILES32` / `$PROGRAMFILES64`（`\x02` + `0xC1` / `0x81`） | `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\ProgramFilesDir` を 32／64 ビットビューで読む。空なら文字列 49 `$PROGRAMFILES` / 32 `C:\Program Files` | 仮想レジストリは空。この値を読む経路は `CreateFruityInstance` では未到達 |
| `$SMPROGRAMS`、`$DESKTOP`、`$SYSDIR` | CSIDL_PROGRAMS／COMMON_PROGRAMS、DESKTOPDIRECTORY、SYSTEM | 使わない（ショートカットの作成で、エンジンは読まない） |
| （`Shared data`、`Shared Data`） | **インストーラは書かない** | `C:\Users\user\Documents\Image-Line\`（`reg-seed-app.tsv`。**出典なしの選択**。エンジンの既定の組み立て（`f_653130`: CSIDL_PERSONAL + `Image-Line` + `\`）と同じ形。`C:\Users\user\Documents` は Learn の KNOWNFOLDERID の表（FOLDERID_Documents の既定 `%USERPROFILE%\Documents`）と、決定 0107 のユーザー名 `user` から） |

## 6. 種付けファイルとの対応

- `lift/harness/seeds/reg-seed-nsis.tsv`（**インストーラ由来**。5 行）: 上の表の `FL Studio`（HKLM、HKCU）、`FL Studio engine`（HKLM、HKCU）、`Install path`（HKCU）を、仮想パスで。値の「仮想パス」は選択、キー・値名・型・データの形はスクリプトの読み。
- `lift/harness/seeds/reg-seed-app.tsv`（**インストーラ由来ではない**。1 行）: `Shared data`。
- 測定（`CreateFruityInstance`、両方または app だけ）で、最初の停止点は同じ（`WinVerifyTrust`）。nsis を足すと呼び出しの入口数が 35,093 → 35,096、新規の関数が 313 → 312。インストーラの値は壁を動かさない。

## 7. 限界

- スクリプトの**条件分岐**は追っていない（どの経路でどの書き込みが走るかは、入口番号と「場所」だけ。たとえば 64 ビット判定 `$v39`、「全ユーザー」の選択 `$v56`、`$v46`）。
- `ILSetup.dll` と `UAC.dll` と `System.dll` のプラグイン呼び出しは中身を読んでいない（§4）。`UserMgr.dll`、`InstallOptions.dll` も同じ。
- インストール**後**にアプリ自身が書く値（`Shared data`、`Registrations\Token`、`General\FL Cloud`、`FL Studio 26\DSTITF` ほか）は、スクリプトからは読めない。
- 実機 Windows との照合は 0 件。
