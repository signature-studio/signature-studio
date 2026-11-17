# 0233: LoadLibrary は『build が受け持たない名前』を volume の標準探索で答える(無ければ NULL + 126、ファイルがあれば名前つきの停止)。出荷構成の bundle が参照プロジェクトを『Problems loading the project / 8 plugin(s) could not be opened』まで開く — 仮の決定(load1)

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0233〔仮番号〕、2026-11-04 22:40〜、総計 3 時間)、作業者 `load1`。基準 HEAD は 8e9137c で作業し、`86ae7d85`(ship5/hyb6 の docs・seed 追加後)の `git archive` に `load1.diff` と `load1-gpa.diff` を順に当てて、木がワークツリーと一致することを確認した(`patch -p1 --dry-run` rc=0、offset・fuzz・reject なし)。

## 冒頭の三点

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → wasm にした物。代役・再生・画像・期待値の貼り付けは 0)を、**CI の段スクリプト**(extract → felift → lift → b2d → objects → link → seed → bundle。F2xH5、混成なし〔停止は混成と無関係〕、manifest の env に `FLE_SOFT_STOP`・`FLE_LOADLIB_MISSING` なし)で作った bundle(基準の HEAD の物と、今回の実装を入れた物。各々 `NO_FS=0` と出荷形の `NO_FS=1`)で動かした。(a) node(JSPI・仮想時計)で ref1 の G3(File>Open の選択・Enter・Space はハーネス)を、**出荷の volume**(bundle 自身の seed `vfs-seed-idle1.tsv` + 参照プロジェクトの 49 ファイルだけ。DLL の行なし)と **ship4 の volume**(installer の `oggio_x64.dll` 等の行つき)で、(b) 隔離 Chrome 156 で実時間の CDP 入力(ページのファイル窓にプロジェクトの 49 ファイルを渡す・エンジンの File>Open でプロジェクトを選ぶ・OK はハーネス)を、(c) 起動 16 スナップを基準の bundle と今回の bundle で、(d) native テストと cargo の workspace 全体を、実際に走らせた。この機械(16 コア、SwiftShader、load 5〜50 で他の担当と共用)で、依頼者の機械ではない。入力はすべて模擬(CDP・G3)で人の操作ではない。
2. **結論**:
   - **停止の確定**(§1): ship4 の停止は『volume に installer の `oggio_x64.dll` の行があるのに build に oggio が無い』ときの `LoadLibraryW("C:\Program Files\Image-Line\FL Studio 21\Shared\oggio_x64.dll")`(呼び元 VA 0x44db70)。**出荷の volume では最初の停止は名前のみの `LoadLibraryW("oggio_x64.dll")`(同 0x44db70)で、それを越えると相対パスの `LoadLibraryW("ffmpeg\x64\ilvideo_x64.dll")`(同 0x44db70)、さらに Elastique 失敗後の `GetProcAddress(NULL, "ElAllocator_SetFunctions")`(呼び元 0x1d1d7be)**。どれも探索スイッチ無しの既定では `fle_stop`。
   - **実装**(契約の実装のみ。FL の名前・クラス・窓での分岐なし。`getenv` 3 個〔`FLE_LOADLIB_MISSING`・`FLE_LOADLIB_TRACE`〕と `FLE_SOFT_OR_STOP` の 1 箇所を `fle_load.c` から除去): build が受け持たない名前(path 有無とも)は **Learn の標準探索順で volume を探し、無ければ NULL + ERROR_MOD_NOT_FOUND(126)、ファイルがあれば名前つきの停止**。ただし PM の指示(i)で、**実機の Windows が出荷する DLL の名簿**(`fle_win_dll_roster.inc` 612 名、Wine 10.0 の `dlls/` 710 ディレクトリ名から 610 名 + 決定 0015 の 10 名のうち Wine に無い 2 名、UNOFFICIAL、データ)と **API set の契約名の形**(Learn)にある名前で仮想モジュールでない物は**今までどおり名前つきの停止**。`GetProcAddress(hModule = NULL)` は Wine 10.0 の分岐(NULL = プロセス自身の image)に従い、image(FLEngine)の export 3 名に無い名前は NULL + 127(`load1-gpa.diff`)。
   - **検証(実エンジン、出荷構成)**: 出荷の volume で、node の G3 は 10 点の snap が最後まで通り、窓表に `TMissingPluginsForm "Problems loading the project"`。stderr はエンジン自身の `MessageBoxW`(VM が既定ボタンで返答。表示はしていない)"Elastique DLL couldn't be opened. Please reinstall the application." と末尾の session 終了行だけ。**Chrome 156(実時間・出荷形 `NO_FS=1` の bundle・desk2 のページ)ではエンジン自身の窓『8 plugin(s) could not be opened / 2 sample(s) required by this project could not be located / This is not a serious error, the project will continue to load normally and the plugin(s) will be ignored.』が OK から 5〜7 秒後に出て、Enter で閉じ、Space で再生が進む**(スクリーンショットを目で確認)。停止なし。起動 16 スナップ(84 行 = 82 行 + 停止/メッセージ 2 行)は基準 bundle と今回の bundle で**全行一致**。native テストは cargo の workspace 全体で **193 results / 2,369 passed / 0 failed**(基準 2,361 passed。差 +8 は新テスト)、a4 FRUITY 無効は **942/942・895/895・functions 5,310・entries 1,009,737 で不変**、FLEngine の既定の生成 C の sha256 は **`cd9107ce…e50`(215,811,678 B)で不変**(runtime とテストだけの変更)。
3. **未確認**: 実機 Windows との照合は 0 件(126 と 127 の番号、名簿が実機の System32 と一致するか)。依頼者の機械・Firefox 159・実マウスでは未実施(Firefox の検証もしていない)。`run-g3.sh` の ship3 の表との比較(node 用の別リンクが要る)は繰り返していない(A/B の一致で代えた)。oggio・Elastique を build に入れる F2xH6 は S1 の線待ちで未実施。`GetModuleHandleW/A` の path 付きと `GetModuleHandleExW` は同種の停止だが参照プロジェクトでは踏まないので触っていない。

## 1. 停止の確定(実エンジン、基準 bundle)

`tables/stop-table.tsv` が表。要点:

| 実行 | volume | 止まった API・引数 | 呼び元 VA | fle_load.c の分岐 |
|---|---|---|---|---|
| ship4 と同じ | `vfs-seed-ref1.tsv`(idle1 + install のデータ + installer の `Shared\oggio_x64.dll`・`wavpackdll_x64.dll`・`DelZip64.dll` + プロジェクト) | `LoadLibraryW("C:\Program Files\Image-Line\FL Studio 21\Shared\oggio_x64.dll")` | 0x44db70(`f_44db40`: `SetErrorMode ; LoadLibraryW ; SetErrorMode`。診断を 1 行足した bundle で取得) | `normalize()` の『`\` `/` `:` を見つけたら `fle_stop`』(上にある `FLE_LOADLIB_MISSING` の枝は通らない) |
| 出荷 | `tables/vfs-seed-load1.tsv`(bundle 自身の seed `vfs-seed-idle1.tsv` + プロジェクトのファイル。bundle の seed pack の物と `cmp` で一致を確認) | `LoadLibraryW("oggio_x64.dll")`(名前のみ) | 0x44db70 | `load_by_name()` 末尾の `FLE_SOFT_OR_STOP(..."module is not a virtual module and not a name this environment is defined to lack")` → 既定は `fle_stop` |
| 出荷 + `FLE_SOFT_STOP=1`(決定 0209) | 同上 | 上が NULL/126 になった次に `LoadLibraryW("ffmpeg\x64\ilvideo_x64.dll")`(相対パス) | 0x44db70 | `normalize()` のパス停止(同上の第 1 行) |
| 出荷 + 実装(探索の途中経過) | 同上 | `GetProcAddress(NULL, "ElAllocator_SetFunctions")` ×9(その後 195 回の `LoadLibraryW("Elastique_x64.dll")` はすべて NULL/126) | 0x1d1d7be | `fle_modules.c` の『NULL は仮想モジュールのハンドルではない』の停止 |

**探索スイッチ付きで通った経路との差**: ref1・ref2 は `FLE_SOFT_STOP=1`(名前のみ → NULL/126)と `FLE_LOADLIB_MISSING=1`(path 付き → 『無い』)の両方を使っていた。出荷の volume + スイッチ無しだと 1 つ目の分岐で止まる。ship4 の報告にあった『パス名は run に出ていない』は、停止メッセージが名前を含まないため(診断の `FLE_LOADLIB_TRACE=1` はあった)。

**ship4 の停止が出荷の挙動かどうか**: ship4 が使った volume は ref1 の seed で、installer の `oggio_x64.dll` を volume に置いている。この volume + 6 モジュールの F2xH5 は『volume にはあるが build に無い』状態で、(b) の名前つき停止が**正しい挙動**(§4)。出荷の volume(DLL の行なし)ではエンジンは名前のみで読みに来て、(a) の NULL/126 で通る。

## 2. 契約の出典

`tables/contracts.tsv` が全文(原文の引用・URL・実装の対応)。取得は Learn の markdown(MicrosoftDocs/sdk-api、win32 の raw)と Wine 10.0 のソース(`learn/` に保存、`wine10-loader.c`・`wine10-kb-loader.c`・`wine10-ntdll-error.h`)。要点:

- **LoadLibraryW**: 『If the string specifies a full path, the function searches only that path for the module.』『If the string specifies a relative path or a module name without a path, the function uses a standard search strategy』『If the function cannot find the module, the function fails.』『If the function fails, the return value is NULL.』 → 完全パスはそのパスだけ、相対パス・名前のみは探索順の各フォルダ、無ければ NULL。
- **LoadLibraryExW**: 『If lpFileName specifies a relative path, the entire relative path is appended to every token in the DLL search path list.』『If the function cannot find the module or one of its dependencies, the function fails.』`LOAD_LIBRARY_AS_DATAFILE` ほかのフラグは『ファイルの探索を変える』物で、無いファイルの答えは変えない。
- **エラーコード**: Learn の LoadLibrary・LoadLibraryEx・GetModuleHandle のページに番号の文は**無い**。System Error Codes の 126 =『The specified module could not be found.』。**Wine 10.0(UNOFFICIAL)**: `ntdll/loader.c` の `open_dll_file`(2627〜2658 行。2650〜2658 が『`STATUS_OBJECT_PATH_NOT_FOUND`・`STATUS_OBJECT_NAME_NOT_FOUND` なら探索を続ける』)が `STATUS_DLL_NOT_FOUND` を返し、`find_dll_file`(3227〜3232 行)が完全パスにも相対パスにも使い、`kernelbase/loader.c` の `load_library`(152〜182 行)が `set_ntstatus`、`ntdll/error.h` 674 行が c0000135 を `ERROR_MOD_NOT_FOUND` に写す。**ファイルが無い場合もフォルダが無い場合も 126**(2/3 ではない)。実機の Windows の値は読んでいない(推測: 同じ)。
- **API set**: Learn『Windows API sets』『LoadLibrary (and delay load) succeeds with an API set name』、契約名は『api- または ext- で始まり、l<n>-<n>-<n> で終わり、本体は英数字か `-`』。
- **volume を探す API の戻り値**(依頼の後半): `FindFirstFileW` は『no matching files → ERROR_FILE_NOT_FOUND』(Learn)、無いフォルダは 3(`fle_vfs.c` のコメントは『推測』。Wine `ntdll/error.h` 423 行 `STATUS_OBJECT_PATH_NOT_FOUND → ERROR_PATH_NOT_FOUND` と一致)、`GetFileAttributesW` は失敗で `INVALID_FILE_ATTRIBUTES`(番号の文は無い。ファイル無し 2・フォルダ無し 3、Wine `error.h` 417・423 行と一致)。`PathFileExists*` はエンジンが import していない。**変更なし**(契約どおり)。

## 3. 実装(`load1.diff`、`load1-gpa.diff`)

`lift/runtime/fle_load.c`(正味の規則。表は `tables/boundary.tsv`):

1. 空の名前 → NULL/126(決定 0229、変更なし)。
2. 名前に `\` `/` `:` があれば `load_with_path`: (2.1)リンクの path 表(決定 0183。F2xH5 には表が無い)が実モジュールに結ぶ path は『ファイルが volume にあれば実モジュール、無ければ NULL/126』、(2.2)volume に**ファイル**があれば(完全パスはそのパスだけ、相対パスは cwd・アプリのフォルダ〔`FLE_VIRTUAL_EXE_PATH` の dir〕・System32・Windows・`SetDllDirectoryW` のフォルダ)**名前つきの停止**(境界 (b))、(2.3)名簿・仮想モジュールの名前が C:\Windows・System32・SysWOW64 にあれば名前つきの停止、(2.4)UNC・device・ドライブ相対・他の `:`・ANSI の高位バイトは従来どおり停止、(2.5)それ以外は NULL/126(**他のドライブ文字もここ**: volume は C: だけ)。
3. 実モジュール(リンクの表)→ 仮想モジュール(39 名)→ 『無い』と定義済み(5 名)は従来どおり。
4. **名簿・API set の名前**で仮想モジュールでない物 → 名前つきの停止(PM の(i))。
5. それ以外の名前のみ(`oggio_x64.dll`・`Elastique_x64.dll` など製品・第三者の DLL)→ volume を探し、ファイルがあれば名前つきの停止、無ければ NULL/126(PM の(ii))。

`fle_load.c` から `getenv` 3 個(`FLE_LOADLIB_MISSING` ×2、`FLE_LOADLIB_TRACE`)と `FLE_SOFT_OR_STOP` の 1 箇所を除去。runtime 全体の `getenv` は 39 → 36、`FLE_SOFT_OR_STOP` の箇所は 11 → 10(残りは `fle_locale.c` 8、`fle_wall_ia2v.c` 2。**触っていない**)。

`lift/runtime/fle_win_dll_roster.inc`(新規、データ。`docs/investigations/load1/tools/mk-roster.py` が `docs/investigations/load1/tables/wine10-dlls-dir.txt`〔GitHub API `contents/dlls?ref=wine-10.0` の 710 名〕から作る): `wine*`・16 bit・`.sys .vxd .ds .msstyles .tlb .exe` を除き、拡張子が `cpl ocx acm ax drv` の物は名前のまま、それ以外は `.dll` を付けた 610 名(決定 0015 の調査で測った 10 名〔psapi riched20 urlmon ws2_32 mswsock iphlpapi wship6 fwpuclnt idndl normaliz〕のうち 8 名はこの 610 名に含まれる)+ Wine に無い 2 名(`idndl.dll`・`wship6.dll`)= 612。**実機の System32 そのものではない**(Wine の builtin 名は VC の再頒布物も含む。含む方向は停止が増えるだけで安全側、含まれない実機の DLL があれば 126 になる〔推測〕)。製品の DLL(oggio・Elastique・ILVideo・wavpack・DelZip64・dsp_ippv2・Blend2D・QuickFontCache・FreeType・ILTools・FLEngine)は含まれないことをテストで固定。

`lift/runtime/fle_modules.c`(`load1-gpa.diff`): `GetProcAddress` の hModule が NULL(または `GetModuleHandleW(NULL)` の値 0x400000)のとき、プロセス自身の image = FLEngine_x64.dll の export directory を引く。export は 3 名(`CreateFruityInstance`・`__dbk_fcall_wrapper`・`dbkFCallWrapperAddr`。PE ファイルから読んだ。`tables/flengine-exports.txt`)。それに無い名前は **NULL + ERROR_PROC_NOT_FOUND(127)**。Wine 10.0(UNOFFICIAL): `kernelbase/loader.c` 72 行『`if (!module) module = NtCurrentTeb()->Peb->ImageBaseAddress;`』、`ntdll/loader.c` `LdrGetProcedureAddress`(2027〜2058 行)が『export に無い名前 → `STATUS_PROCEDURE_NOT_FOUND`』、`error.h` 487 行が 127。エンジン自身の export 名・序数は従来どおり停止(自分のアドレスは配らない)。Learn の GetProcAddress のページには NULL の記述が**無い**。

## 4. (a)(b) の境界(表: `tables/boundary.tsv`)

- **lift 済み(build に入っている実モジュール、F2xH5)**: `dsp_ippv2_x64.dll`(0x2000000)・`Blend2D.dll`(0x4700000)・`quickfontcache_x64.dll`(0x1d00000)・`freetype_x64.dll`(0x4b00000)・`ILTools_x64.dll`(0x6000000)。
- **仮想モジュール**: 39 名(`tables/virtual-modules.tsv`)。
- **『無い』と定義済み**: `imm32.dll`・`msimg32.dll`・`propsys.dll`・`windowscodecs.dll`・`webview2loader.dll`(決定 0015・0126)。
- **名簿(停止)**: 612 名と API set の形。
- **(a) NULL/126**: 上のどれでもなく、volume に無い物。出荷の volume で実際に答えた名前(参照プロジェクトの G3 全体での LoadLibrary 313 回の全部、`tables/loadlibrary-calls-shipvol.tsv`。診断を足した別の bundle で数えた): 空 ×46、`oggio_x64.dll` ×3、`ffmpeg\x64\ilvideo_x64.dll` ×2、`Elastique_x64.dll` ×195、`imm32.dll` ×2、`WebView2Loader.dll` ×1(後 2 つは定義済み)。
- **(b) 名前つきの停止**: volume に**ファイルがある**物。出荷の volume には 0 件。ship4 の volume では `Shared\oggio_x64.dll`。

## 5. 環境変数・`FLE_SOFT_STOP`(決定 0209)との関係

`FLE_LOADLIB_MISSING`(決定 0178・0206)は探索専用の『何でも無い』で、path 付きは常に、名前のみは `fle_load.c` の末尾で効いていた。**契約の実装(volume の探索)に置き換え、スイッチを除去**(スイッチを設定しても挙動が変わらないことをテストで固定: `load1_the_environment_variables_of_the_exploration_are_gone_from_the_loader`)。`FLE_LOADLIB_TRACE`(決定 0191、名前の診断)も除去(診断用の `getenv` で結果は変えない)。`FLE_SOFT_STOP`(決定 0209)の他の 10 箇所は別の API の壁で、**触っていない**。`FLE_SOFT_STOP` を付けた出荷 volume の実行も、今回の実装では同じ結果(§6)。

## 6. 検証(実エンジン、出荷構成の bundle)

### 6.1 node(JSPI・仮想時計)、ref1 の G3、出荷の volume、スイッチ無し(`tools/refproj.sh`)

bundle は CI の段スクリプト(`tools/iter.sh` が基準の `ciA` の objects を hard link し、変えた runtime の 2 個〔`fle_load`・`fle_modules`〕だけ作り直して `objects link seed bundle`)で作った F2xH5(混成なし、`COMPRESS=none`)。manifest の env は `FLE_CALL_FRUITY`・`FLE_VFS_SEED`・`FLE_VFS_SEED_ROOT`・`FLE_REG_SEED`・`FLE_AUTH_CLAIMS`・`FLE_FRUITY_NO_IDLE_STOP`・`FLE_FRUITY_MAX_CALLS`・`FLE_FRUITY_SECONDS` だけ(探索スイッチなし)。ハーネスが与えたのは File>Open の選択(`FLE_FILEDLG_PICK`)・Enter・Space・プロジェクトのファイルの volume 上の置き場だけ。

| 実行 | bundle | volume | 結果 |
|---|---|---|---|
| 基準(実装なし) | `ciA-fs` | ref1 の seed(installer の DLL 行つき) | `LoadLibraryW("C:\Program Files\...\Shared\oggio_x64.dll")` で停止(`runs/ref-base-trace`) |
| 基準 | `ciA-fs` | 出荷 | `LoadLibraryW("oggio_x64.dll")` で停止(`runs/ref-base-shipvol`) |
| 実装(`fix3`) | `fix3` | 出荷 | **停止なし**。10 点の snap(wiz0 … end)、窓表に `TMissingPluginsForm "Problems loading the project"`(lid 139)。stderr: エンジン自身の `MessageBoxW`(VM が既定で返答)と session 終了行だけ。ユーザー CPU 130.96 s(load 5〜14 の共用機、壁時計は主張しない)、最大 RSS 1.30 GB(`runs/ref-fix3-shipvol`) |
| 実装 | `fix3` | ref1 の seed | 名前つきの停止 `the virtual volume holds a file at "C:\Program Files\Image-Line\FL Studio 21\Shared\oggio_x64.dll" but it is not a module of this build ...`(境界 (b)。`runs/ref-fix3-ref1vol`) |
| 実装 + `FLE_SOFT_STOP=1` | `fix3` | 出荷 | 上の『実装・出荷』と同じ(10 点、同じ stderr。`runs/ref-fix3-shipvol-soft`)。除去した所では soft stop が働かない |

### 6.2 Chrome 156(CDP 入力、実時間、出荷形 `NO_FS=1` の bundle、desk2 のページ、`tools/sdrive-load1.mjs`)

- 出荷形の bundle(`fix3p`)を `vite preview`(自前の build の `app-run`、`FL_ASSETS_DIR` = bundle + `proj/` に参照プロジェクトの 49 ファイル)で配り、`?debug=1` のページを隔離 Chrome で開いた。シナリオ `tools/scn-ref.json`: ウィザードの窓が窓表から消えたことを確認 → ページのファイル窓の Upload 入力に 49 ファイルを渡す(DataTransfer = 人の選択の代役) → エンジンの File>Open でプロジェクトの行を選び OK(クリックはハーネス) → `TMissingPluginsForm` が出るまで待つ → スクリーンショット → ウィザードが無いことを再確認してから Enter → `TMissingPluginsForm` が消えるまで待つ → Space → 12 秒後のスクリーンショット → 状態確認。
- 結果: `ok (no stop)`。OK から 5.3 秒後(load 10〜20)に窓表に `TMissingPluginsForm 'Problems loading the project' @572,382 691x423`。私がスクリーンショット(最終の bundle の `ref-problems.png`・`ref-after-enter.png`。再生中の `ref-playing.png` は 1 つ前の bundle `fix2p` の撮影)を目で読み、窓の文面が『8 plugin(s) could not be opened / 2 sample(s) required by this project could not be located. / This is not a serious error, the project will continue to load normally and the plugin(s) will be ignored.』であること、Enter で閉じた後にプレイリストの波形・チャンネルラック・ミキサーが描かれていること、Space の後は時計表示が進み再生線とミキサーのメーターが動くことを確認した。state=`engine-idle`、stopped なし。(`2 sample(s)` の行は node の G3 の volume では出ない可能性がある。ページに渡したファイルの置き場がプロジェクトの記録と違うため〔S-fpick1-open-folder〕。未調査。)
- 初期の撮影(`fix2`)では Enter 後 4 秒で窓の消えを検査して不合格にした(エンジンが遅いだけ。ハーネスの待ち不足)。ウィザードの窓の消えは `waitWindow` で確認している(`expectWindow visible:false` は窓表が可視の窓しか載せないため常に不合格になる既存の癖で、使わない)。

### 6.3 起動 16 スナップ(`cmp-short.g3`、node-bundle-run、`tools/g3bundle.sh`)

基準の bundle(`ciA-fs`)と実装の bundle(`fix3`)で、`g3-snap`・`g3-progress`・`g3-surface`・`g3-wintable` と `fruity: == stop`・`runtime message` の **84 行が全行一致**(`cmp`、`runs/g3b-base`・`runs/g3b-fix3`)。84 = 82 行 + 停止・メッセージ 2 行。ship3 の表(`expected/g3-lines-ship3-F2xH4-node.txt`、82 行)との直接比較は、その表が `run-g3.sh`(node 用の別リンク)の物でクロック値が違うため**していない**(A/B の一致で代えた)。

### 6.4 生成 C・a4(runtime だけの変更なので不変の確認)

- FLEngine の既定の生成 C(`fe-lift lift FLEngine_x64.dll X --pdata`、HEAD の CI `felift` 段の fe-lift): sha256 `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B — **不変**(`logs/felift-default.out`)。`lift/crates/*/src` は 1 行も変えていない(テストだけ)。
- a4 FRUITY 無効(`lift/harness/a4-fruity.sh off`、ワークツリー `src-fix3` の runtime で native ビルド): `init_count=942 / 942`、`run_rank=895/895`、functions executed (distinct) **5,310**、function entries **1,009,737**、`== stop: returned from entry`、harness exit 0 — **不変**(`logs/a4-off.log`)。(a4 構成の C は 215,812,525 B / `7fdb5b11…` で、PM の言う『既定の生成 C』〔プラグイン構成なし〕とは別の物。)
- CI の静的検査(`ci/`)は変えていない。


## 7. oggio・Elastique の表(未決の点の判断材料)

| 構成 | volume | oggio | Elastique | 結果 |
|---|---|---|---|---|
| F2xH5 + 今回の実装 | 出荷(DLL の行なし) | `LoadLibraryW("oggio_x64.dll")` → NULL/126(エンジンは Vorbis-in-WAV を読めない。参照プロジェクトの該当サンプルは『2 sample(s) required by this project could not be located』の一部かどうかは未確認) | `LoadLibraryW("Elastique_x64.dll")` → NULL/126(195 回)、`GetProcAddress(NULL, "ElAllocator_SetFunctions")` → NULL/127(9 回) | エンジンは自分の箱『Elastique DLL couldn't be opened. Please reinstall the application.』を出し(VM が既定ボタンで返答、表示なし)、続ける。Problems 窓まで進む |
| F2xH5 + 今回の実装 | ship4 の volume(installer の oggio 等の行) | path 付きで volume にファイルがある → (b) 名前つきの停止 | 同上 | **停止が正しい**(volume が build より多くを主張している)。出荷の volume はこうならない |
| F2xH5 + 今回の実装 − `load1-gpa.diff` | 出荷 | 同上 | `GetProcAddress(NULL, …)` で停止 | 参照プロジェクトは開けない |
| F2xH6(oggio・Elastique を 6・7 個目に) | — | 実モジュールになる(path 表と、`ship4-h6.diff` が `oggio_x64.dll` の path 表を CI でリンクするかは**未確認**) | 実モジュールになる | **未実施**(S1 の線の定義待ち)。Elastique の箱は出ない見込み(読み) |

## 8. 旧テストの期待の変更(出典つき)

旧テストが『停止』を期待していた物を、契約の文(Learn の『If the function cannot find the module, the function fails』『the return value is NULL』、126 は決定 0015 と Wine 10.0)に沿って更新した。`-` 行 115(全部読んだ。内訳: `fle_load.c` 61、`load.rs` 26、`realmod.rs` 16、`msctf.rs` 4、`wall_dec1.rs` 5、`deferred.rs` 2、`realmod_s1/check_runs.py` 1。`load1-gpa.diff` は 3 行。意味を消した行は無く、停止の期待が応答の期待に変わった物と、除去した分岐そのもの)。

| テスト | 旧 | 新 |
|---|---|---|
| `load.rs` `external_and_unnamed_modules_stop_by_name` | 13 名(製品の DLL と Windows の DLL)が停止 | `external_and_unnamed_modules_fail_with_null_and_126`(製品の DLL 10 名 + 『点で終わる』『二重拡張子』『拡張子なし』『非 ASCII』の名前が NULL/126)と `windows_dlls_that_are_not_virtual_modules_and_api_set_names_stop_by_name`(名簿・API set の 12 名は停止のまま) |
| `load.rs` `the_other_ten_windows_dlls_still_stop_by_name_after_decision_0088` | 停止 | **変更なし**(10 名は名簿にあるので停止のまま) |
| `load.rs` `undefined_load_library_inputs_stop` | path 付き 4・名前の 5 件が停止 | それらを除く(新テストに移した) |
| `realmod.rs` ほか 5 件 | 未知の名前で停止(`HEAD_STOP`) | NULL/126(`NOT_FOUND`)。対照実験『table なしでは実モジュールにならない』は『host が NULL を受け取り、DllMain は走らない』に |
| `deferred.rs` `what_decision_0096_does_not_change_...` | `blend2d2.dll` などが停止 | NULL/126 |
| `msctf.rs` `names_near_msctf_are_not_answered` | 6 名が停止 | `msctfp.dll` だけ名簿にあるので停止、他は NULL/126 |
| `wall_dec1.rs` | `FLE_LOADLIB_MISSING` の枝を固定 | スイッチが無く `load_with_path` があることを固定 |

新テスト: `load.rs` に `load1_*` 5 本(path 付きの NULL/126 19 呼び出し、volume のファイルでの停止 11 通り、名簿・virtual module の path の停止 10 通り、環境変数が効かないこと、`GetProcAddress` の NULL)と名簿の形(昇順・一意・小文字 ASCII・製品の名前を含まない)。

## 9. 出典のない選択(`S-load1-*`。表: `tables/s-choices.tsv`)

| 番号 | 選択 | 代替 |
|---|---|---|
| S-load1-code | 見つからないとき 126(Learn に番号なし。Wine 10.0 が同じ、UNOFFICIAL) | 2 / 3 / 87 |
| S-load1-roster | 『実機の Windows が出荷する DLL の名簿』= Wine 10.0 の `dlls/` 名 + 決定 0015 の 10 名(データ。実機の System32 そのものではない) | 全部停止(旧)、全部 126(最初の案)、実機の一覧(入手不可) |
| S-load1-roster-folder | path 付きで停止するのは C:\Windows・System32・SysWOW64 の物だけ | 全フォルダで停止 / 全部 126 |
| S-load1-search-folders | 探索するのは cwd・アプリ・System32・Windows・SetDllDirectory(C: の完全パス)。16 bit system と PATH は探さない | PATH も |
| S-load1-volume-file | volume のファイルは停止、ディレクトリは module でない(126) | ACCESS_DENIED |
| S-load1-other-drive | 他のドライブ文字は 126 | 停止 |
| S-load1-unmodelled-forms | UNC・device・ドライブ相対・他の `:`・ANSI の高位バイトは従来の停止 | 126 |
| S-load1-api-set-form | API set の契約名の形は、仮想モジュールでなければ停止 | 126(FLE_SOFT_STOP=1 の答え) |
| S-load1-gpa-process-image | `GetProcAddress(NULL)` = プロセス image(FLEngine)の export 3 名を引く。無い名前 → NULL/127。自分の export・序数は停止 | NULL/6(最初の案、出典なし) |
| S-load1-seed | 出荷の volume のテスト用 seed = bundle の seed + プロジェクトの 49 ファイル | ref1 の seed(installer の DLL 行つき) |
| S-load1-trace-removed | `FLE_LOADLIB_TRACE` も除去 | 残す |
| S-load1-chrome-harness | Chrome ではページのファイル窓に 49 ファイルを渡し File>Open の行を選ぶ(ハーネス = 人の選択の代役)。`FLE_FILEDLG_PICK` は使わない | manifest の env |

## 10. PM の未決の点

1. **名前のみの未知 DLL の既定が『停止』から『NULL/126(名簿にある名前は停止のまま)』に変わる**(従来は `FLE_SOFT_STOP=1` か `FLE_LOADLIB_MISSING=1` の探索だけ)。製品・第三者の DLL(oggio・Elastique・ILVideo・プラグイン)が対象。名簿の網羅は Wine 10.0 の名前に依存(S-load1-roster)。
2. **`load1-gpa.diff` は参照プロジェクトを出荷構成で開くのに必要**(PM は採る方針)。Elastique が出荷 6 モジュールに無いため、エンジンは自分の箱『Elastique DLL couldn't be opened. Please reinstall the application.』を出し(VM が既定ボタンで返答、画面には出ない)、`GetProcAddress(NULL, "ElAllocator_SetFunctions")` を 9 回呼んで続ける(実機の Windows で Elastique が無ければ同じ道を通ると読む〔推測〕)。**時間伸縮(Elastique)を使うクリップは出荷構成では鳴らない**(F2xH6 まで)。
3. oggio が無い間、Vorbis-in-WAV のサンプルは読み込まれない(エンジンは該当の DLL を見つけられない物として扱う)。『2 sample(s) required by this project could not be located』との関係は未調査。
4. `census_report.py`(`module "X" is not a virtual module` の停止行から未対応の module 名を集める測定ツール)は、名簿にない名前では停止行が出なくなるので、この方法では拾えなくなる(名簿の名前は引き続き拾える)。歴史的な探索の記録(`docs/investigations/*` の `FLE_LOADLIB_MISSING` 設定)は無害(スイッチは何もしない)。

## 11. 再現表(作業ディレクトリ `lift/out-load1/` の相対。`tools/` はこの結果の `tools/`)

| 項目 | コマンド | 出力 |
|---|---|---|
| bundle(基準) | `bash tools/build.sh $PWD/src $PWD/ciA LOG [stage…]`(`HYBRID_SETS= COMPRESS=none`、`run-ci-build.sh` が CI の段を順に呼ぶ) | `ciA/engine/bundle/<prefix>/`(`NO_FS=1`)、`bash tools/fs-variant.sh ciA $PWD/src` で `NO_FS=0` |
| bundle(実装) | `bash tools/iter.sh NAME TREE NOFS fle_load fle_modules`(基準の objects を hard link し runtime の 2 個だけ作り直して `objects link seed bundle`) | `NAME/engine/bundle/<prefix>/` |
| 参照プロジェクト(node) | `SEED_TSV=<tsv> BUNDLE=<NO_FS=0 の bundle> [EXTRA_ENV='K=V'] bash tools/refproj.sh NAME` | `runs/ref-NAME/` |
| 16 スナップ | `bash tools/g3bundle.sh BUNDLE NAME` | `runs/g3b-NAME/g3-lines.txt` |
| Chrome | `vite preview`(`app-run`、`FL_ASSETS_DIR=<manifest.json + bundle + proj/>`)→ `node tools/sdrive-load1.mjs --browser chrome --url 'http://127.0.0.1:PORT/?debug=1' --proj-dir <proj/> --out DIR --scenarios tools/scn-ref.json --viewport 2176x1224 --app app-run` | `DIR/report.json`、`*.png` |
| 名簿 | `python3 docs/investigations/load1/tools/mk-roster.py docs/investigations/load1/tables/wine10-dlls-dir.txt lift/runtime/fle_win_dll_roster.inc` | `fle_win_dll_roster.inc`(612 名) |
| native テスト | `cd lift && cargo test --offline --workspace --no-fail-fast` | 193 results / 2,369 passed |
| a4 | `INPUT_LIFT=<lift> OUT=<new> JOBS=4 bash lift/harness/a4-fruity.sh off` | 942/942・895/895・5,310・1,009,737 |
| 生成 C | `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata` → `sha256sum` | `cd9107ce…e50` |
| 差分 | `bash tools/mkdiff.sh BASE SRC OUTDIR` | `load1.diff`・`load1-gpa.diff` |

## 12. 規則違反の自己申告と確認できなかったこと

- 存在しない `bash` ツールを 1 回誤って呼んだ(shell に読み替えられ、副作用なし)。
- 出力が長いコマンドを 1 回そのまま端末に出した(`LoadLibrary` の全呼び出しのトレース。context を無駄にした。内容は表に集約済み)。
- 同じ木 `src-fix2`・`src-fix3` を作る `cp -a` と、それを使う bundle 作りを並列の tool call にしてしまい、2 回 bundle の段が『EXTRA_IMPORTS を読めない』で失敗した(作り直して解決。結果には影響なし)。
- 他担当の `vite preview`(port 4961)に当たったまま 1 回測定を走らせかけた(結果は使わず、別 port で撮り直した)。
- 作業ディレクトリが一時 9 GiB(指示は 8 GiB 以内)になった(cargo のテスト出力 2.5 GB と hard link の木の du 重複)。終了時に 500 MB 以内に整理する。
- `sleep`/`nohup`/`pkill -f`/`pgrep -f`/末尾 `&` は使っていない(`tail --pid` の前景待ちと `ps | grep` を使った)。
- 確認できなかったこと: 実機 Windows との照合(126・127・名簿)、Firefox 159・依頼者の機械、F2xH6、`run-g3.sh` の ship3 の表との直接比較、`2 sample(s)` の行が出る理由、`GetModuleHandle*` の path 付き、PATH の探索。

## PM の決定と取り込み(2026-11-05)

取り込んだ基準: HEAD 86ae7d85 に `load1.diff`(11 ファイル、`-` 113 行は全部読んだ。削除は探索スイッチの `getenv` 3 個と `FLE_SOFT_OR_STOP` 1 箇所、旧挙動を固定していたテストの期待値)と `load1-gpa.diff`(3 ファイル、`-` 3 行)を `patch -p1`(offset・fuzz・reject なし)。PM が測った値(この機械、`lift/out-pmchk-l1/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、全体テスト **193 results / 2,369 passed / 0 failed / ignored 1**、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737、typecheck rc=0、vitest **1,377 passed + 7 skipped**、Worker 50/50、e2e **893/893**。

1. **LoadLibrary の既定を『停止』から『Learn の標準探索順で volume を探し、無ければ NULL + ERROR_MOD_NOT_FOUND(126)』に変える。** 番号 126 は Learn に無く Wine 10.0 の出典つきの UNOFFICIAL。ファイルが volume にあるのに build が受け持たない場合は名前つきの停止のまま(volume が build より多くを主張している状態)。探索スイッチ(`FLE_LOADLIB_MISSING`・`FLE_LOADLIB_TRACE`・`FLE_SOFT_OR_STOP` の 1 箇所)は除去(方針 (a))。
2. **名簿(`fle_win_dll_roster.inc`、Wine 10.0 の dlls/ 由来 612 名+決定 0015 の 10 名、データ、UNOFFICIAL)と API set の形にある名前は、仮想モジュールでなければ今までどおり名前つきの停止。** 名簿は網羅の保証が無い。名簿にない実機の DLL は 126 を返してしまう(推測)。census_report.py は『is not a virtual module』の停止行を読むので、名簿にない名前は拾えなくなる(後続の課題)。
3. **GetProcAddress(NULL) は Wine 10.0 の kernelbase/loader.c と ntdll の LdrGetProcedureAddress を出典に、プロセス image(FLEngine)の export 3 名にあれば答え、無ければ NULL + 127。** 最初の案(エラー 6)は出典なしだったので worker が捨てた。Learn に NULL の記述は無い(UNOFFICIAL)。
4. **出荷構成(F2xH5)の bundle は参照プロジェクトをエンジン自身の『Problems loading the project / 8 plugin(s) could not be opened … will be ignored』まで開く**(node の 10 点の snap、Chrome 156 の実時間入力で窓・再生線・メーターが動く)。出荷 6 モジュールでは Elastique を使うクリップは鳴らない(F2xH6 まで)。oggio が無い間 Vorbis-in-WAV は読まれない。
5. **未解決:** Chrome の窓には『2 sample(s) required by this project could not be located』が出る(node の G3 では未確認。volume の違いか、ファイル名の扱いかは未調査)。実機 Windows との照合は 0 件(126・127・名簿が実機の System32 と一致するか)。
