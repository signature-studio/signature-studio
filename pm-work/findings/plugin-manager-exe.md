# PluginManager.exe(FL の Plugin Manager): 調査と設計(#1499)

状態: 段 1(PE の表)・段 2(VFS の lazy)・段 3(lift の方式と 1 つの wasm)は完了。受け入れは、この作業の木で通った(9 節。この再試行の実測。E2E の smoke は終了コード 0)。段 4〜6(起動の経路・窓・E2E の印を外す)は段 B(この Issue が閉じた後に起票)。

## 0. 要点

- 実行ファイルは、抽出の段で作業の木に入る(`lift/out-app/host-extra/resources/System/Tools/Plugin Manager/PluginManager.exe`。大きさ 7,158,856 B・SHA-256 は一覧の値と一致)。接頭辞の行は main の `dd2e9e2c` に在る。
- PE は x86-64・GUI・Delphi(1 節)。`fe-pe` の道具で読めた。`fe-pe rebase`(0x10000000)も通った(reloc の 742 ブロック)。
- 方式: 1 つの plugin の行(`lift/plugins/plugin-manager/plugin.tsv`、役割 補助)として、DLL と同じ lift の段を通す(3 節)。VFS は、固定の seed 表の `@lazy-file:` の 1 行と、`pack.args` の `--lazy-file` の 1 組(2 節)。
- 観測した lift: `fe-lift lift` は、このファイルの 1 モジュールを 3.6 秒で C の unit 6 本(約 63 MB)にした。`fe-wasmgen` の lists 段は 25 秒で、21,902 行のうち 21,622 行が ok。

## 1. PE の表

道具: `lift/target/release/fe-pe pe-info` / `pe-exports` / `pe-pdata-info` / `rebase`(`fe-pe` を release で作ったもの)。対象は作業の木の実物(SHA-256 `66100a71…b2eb32f1e7` が一覧と一致)。runner の値。この機械とは別枠。

| 項目 | 値 | 出所(道具の出力) |
|---|---|---|
| Machine | 0x8664(AMD64。x86-64 である) | pe-info `machine` |
| ImageBase | 0x400000 | pe-info `image_base` |
| Subsystem | 2(WINDOWS_GUI。GUI である) | pe-info `subsystem` |
| Characteristics | 0x0022(EXECUTABLE_IMAGE・LARGE_ADDRESS_AWARE。DLL ではない) | pe-info `characteristics` |
| DllCharacteristics | 0x8160(DYNAMIC_BASE・NX_COMPAT・HIGH_ENTROPY_VA・TERMINAL_SERVER_AWARE) | pe-info `dll_char` |
| AddressOfEntryPoint(RVA) | 0x4a6230(VA 0x4a6230 + 0x400000 = 0x8a6230) | pe-info `entry_rva` |
| SizeOfImage / SizeOfHeaders | 0x6fa000(7,315,456 B) / 0x400 | pe-info `size_of_image` |
| `.reloc` | 有る(RVA 0x5fb000・vsize 0x3e054)。rebase 0x10000000 で 742 ブロック・DIR64 123,691・ABSOLUTE 359 を直し、通った | pe-info sections / rebase |
| セクション | 11 件: .text・.data・.bss・.idata・.didata・.edata・.tls・.rdata・.reloc・.pdata・.rsrc | pe-info sections |
| `.pdata`(関数の入口) | 20,927 件(chained 0) | pe-pdata-info |
| 輸出(export) | 2 件: `dbkFCallWrapperAddr`(RVA 0x5d52a8)・`__dbk_fcall_wrapper`(RVA 0x1c360)。FL の DLL の輸出と同じ名前 | pe-exports |
| import する DLL | 16 件・576 名: kernel32 154・user32 207・gdi32 105・comctl32 30・ole32 19・oleaut32 18・shell32 16・advapi32 10・winmm 4・version 3・IMAGEHLP 3・shlwapi 2・comdlg32 2・SHFolder 1・msacm32 1・api-ms-win-crt-string-l1-1-0 1 | pe-info imports(一覧の imports と一致) |
| Delphi の目印(文字列) | unit 名 `System.SysUtils` 76 回・`Vcl.Controls` 62 回・`Vcl.Graphics` 42 回・`Vcl.Dialogs` 35 回・`Vcl.StdCtrls` 24 回。フォーム型 `TPluginListForm` 6 回・`TNameEditForm` 3 回。`TObject` 289 回 | grep -a -o の出現数(実測) |
| 範囲外の印 | `pe-init-table` は、この実行ファイルでは使えない(入口の参照が FL のエンジンの形でなく、「RVA 0xf768b0 is not mapped」で止まる) | pe-init-table(実測) |

Delphi の印は、輸出の表ではなく、文字列(unit 名と型名)から読んだ。輸出は 2 件だけ。

## 2. VFS の置き場所: lazy の資源(`@lazy-file:`)

- 固定の seed 表 `lift/harness/seeds/vfs-seed-idle1.tsv` の末尾に 1 行: guest `C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe`・host `@lazy-file:resources/System/Tools/Plugin Manager/PluginManager.exe`・SHA-256 の値。
- `lift/harness/web/pack.args` の末尾に 1 組: `--lazy-file lift/out-app/host/resources/System/Tools/Plugin Manager/PluginManager.exe=seed/resources/System/Tools/Plugin Manager/PluginManager.exe`。bundle の manifest の `lazy` の名前は `PluginManager.exe`(`app/scripts/pack-assets.mjs` は PATH の基底名を使う)。
- 理由: 7 MB は、起動の前の大きさを増やさない。既存の resources(114 件の `@lazy-file:resources/...`)と同じ形で、起動の後に、要るとき(GetFileAttributesW の後の読み)だけ届く。
- 生成の seed 行は作らない(`plugins.rs` の `seed_table`・`seed.args`・`pack_args` が、`.exe` の行を外す)。同じ guest path の行が 2 つになると、VFS の表が衝突するため。
- 実物は `lift/out-app/host/resources/`(抽出の段が `ci/resources-prefixes.txt` の行で置く。全ての作業で `host/resources` にも置かれる)。

## 3. lift の方式と、1 つの wasm

方式: (a) を選んだ。`lift/plugins/<名前>/plugin.tsv` の形を、exe に使えるように最小に広げる。(b)(exe 専用の表)は、同じ 4 つの表(modules・hyb・hybrid・pack)を二重に持つことになるので、取らなかった。

exe の行(`lift/plugins/plugin-manager/plugin.tsv`): key `plugin-manager.pluginmanager.dll`(realmod の名前は鍵。エンジンは、ファイル名から `.<基底>` と `.dll` を外した名前で表を引くので、鍵は `.dll` で終わる。ファイルの種類は guest の `.exe` で決める)・guest `System\Tools\Plugin Manager\PluginManager.exe`・役割 補助・hook none・args(既定の `--pdata`)・omit `-`。

変えた箇所(すべて `.exe` の行に限る):
- `lift/crates/fe-build/src/plugins.rs`: `validate` は、ファイル(guest の基底名)の拡張子が `.dll` か `.exe` であることを確かめる(鍵は `.dll` のまま)。`is_exe()` は guest の拡張子が `.exe`。`seed_table`・`seed.args`・`pack_args` の seed 行は `.exe` を外す(2 節)。`plugin_db_names` は変えない(本体の無いフォルダを飛ばすのは #1429 からの既存の挙動。exe の行は項目を作らない)。試験 2 件(exe の行・第 2 波の数え方)。
- `app/scripts/lib/plugin-rows.mjs`(抽出の行の鏡。Rust と同じ規則): `.exe` を受ける。seed 行は作らない。
- `tools/e2e/scenarios/smoke/plugin-database-lists-builtins.mjs`: この段では変えない。本体の無いフォルダを数えない処理は main(`b0ae9519`。`builtinPluginsOf`・`mainRows`)に在り、衝突では main の版を採った(9 節)。
- `inputs/plugin-image-sizes.tsv`: 第 2 波の試験が image の大きさを引くので、exe の行を足す(SizeOfImage 7,315,456。1 節の値)。
- `lift/plugins/plugin-manager/plugin.tsv`(新規)。

流れ(生成の段 → 既存の段): 抽出(`app/scripts/extract-fl.mjs` が、`ci/extract-dlls.tsv` に `lift/plugins/*/plugin.tsv` の行を足した表で、`lift/out-plugins/plugin-manager.pluginmanager.dll` へ(中身は exe。sha256 で照合))→ lists(`fe-pe rebase` + `fe-wasmgen` の報告)→ gen-all(`modules.tsv` の行。`--pdata --extra-imports ship6`)→ hybrid(`--separate` の 1 本)→ wasm の `plugin-manager.pluginmanager.exe.<base>.wasm` が `xmod/` に出る → pack(`--lazy-dll` の rebased 像と、`--lazy-file` の xmod)。

根拠の観測(この作業の手元の実測):
- `fe-wasmgen`(lists と同じ引数。`--pdata` と `lift/harness/hyb/n0.args`): 25 秒。報告 21,902 行、ok 21,622。残りは「atomic RMW は C の道」95・「bit test」35 など。
- `fe-lift lift`(1 モジュール・omit 空): 3.6 秒・exit 0。関数 21,960・命令 688,477・C の unit 6 本(m1-00〜m1-05、各約 10 MB)。`setjmp_landings 0`(omit は要らない)・`jumptabs 3`・`msvc_tables 2`・`unsupported 38`(IAT を経由する間接ジャンプ。今は lift しない。段 4 で扱う)。
- rebase の出力名は `plugin-manager.pluginmanager.<基底>.dll`。hybrid の出力は `plugin-manager.pluginmanager.<基底>.wasm`(fe-wasmgen は、モジュールの file の stem に `.wasm` を付ける)。manifest の `lazy` の wasm の名前は、`manager` を含み `.wasm` で終わる。

wasm の大きさ: 16,085,407 B(`lift/out-app/xmod/` の生の wasm。bundle の br は 2,225,582 B)。

## 4. 起動の経路(設計だけ。段 B)

現状:
- `fle_imp_ShellExecuteW`(`lift/runtime/fle_wall_ia2.c`)は、関連付け無し(31)を返す(決定 `S-ia2-shellexecute-noassoc`)。`ShellExecuteA`・`ShellExecuteExW` は `lift/runtime/fle_api2_shell.c` で同じ。
- 協調スレッド(`lift/runtime/fle_thread.c`): `fle_thread_spawn(body, arg)` で、ゲストのスレッドを作れる。
- realmod の読み込み(`fle_realmod_find_path` の接尾辞の照合。今回の exe の接尾辞は `system/tools/plugin manager/pluginmanager.exe`。realmod の名前は `plugin-manager.pluginmanager.dll`)。

段 B で決めること(この段では結ばない): ShellExecuteW の lpFile が、この接尾辞に一致するとき、realmod を読み、入口をスレッドとして走らせる。`GetModuleHandle(NULL)`・`ExitProcess`・終了コードの扱い。

## 5. 窓(未着手)

Plugin Manager の窓の表(クラスと題名)は、窓が出た後に確かめる。exe には `TPluginListForm` の型名が在る(1 節)。今の窓の表の実測(`pm-work/findings/crash-hunt-2026-11-25.json`)の `TPlugListForm`(題名 `PlugList`)とは別物かも知れない。確かめるのは段 B。

## 6. E2E(`plugin-manager-opens`)

`tools/e2e/scenarios/smoke/plugin-manager-opens.mjs` の `expectFail: '#1499'` の印は残す(窓が出るのは段 B の後)。

## 7. 段ごとの進み

| 段 | 状態 | 中身 |
|---|---|---|
| 1 PE | 完了(道具の出力。1 節) | pe-info・pe-exports・pe-pdata-info・rebase・strings |
| 2 VFS | 完了(manifest の lazy に `PluginManager.exe`) | 固定の seed の 1 行・pack.args の 1 組(2 節) |
| 3 lift | 完了(wasm 16,085,407 B・br 2,225,582 B。起動・流れ A〜D・npm test も通った) | plugin 行(3 節)。hybrid の separate は 3 本 |
| 4 起動 | 未着手(段 B) | 設計だけ(4 節)。観測: ShellExecuteW が exe のパスで呼ばれ、関連付け無し(31)で返る(9 節) |
| 5 窓 | 未着手(段 B) | — |
| 6 E2E | 印は残した(段 B で外す)。smoke は全て ok(expectFail の `plugin-manager-opens` を除く) | 9 節 |

## 8. 監査への依頼

1. 段 A(この Issue)は、受け入れを通すまでを 1 つの作業とする。受け入れの中の `plugin-manager-opens` は印(`expectFail`)のままなので、ok は要らない。
2. 段 B は、この Issue が閉じた後に起票する(4 節・5 節・6 節)。

## 9. この作業の受け入れの実測(この再試行の run)

実エンジン: この runner で、作業の木(main の `1dc942df` に段 A の変更を載せたもの。前回の差分を取り込み、衝突を解いたもの)に対して、`npm run build --prefix app`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle` を実行し、`boot-bundle`(engine-idle を観測)・`welcome-flow`(流れ A〜D の 4 行)・manifest の `lazy`・PE の表・`npm test --prefix app`・`run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2`・`guard/scan.sh` の出力を観測した。受け入れのコマンドは終了コード 0 で通った(`ACCEPT_EXIT=0`・`rows 2, accepted true`・`scan: 0 finding(s) in 0 file(s).`)。加えて `cargo test --release --locked -p fe-build`(30 件・終了コード 0)・`spec/guard.test.sh`(104 件 ok)・`spec/workflow.test.sh`(602 件 ok)・`spec/plugin-rows.test.mjs`・`spec/place-plugin-dlls.test.mjs`・`tools/e2e/test/plugin-database-lists-builtins.test.mjs`(8 件)も通した。

runner の値。この機械とは別枠。nproc 4。loadavg は作業の開始時 0.55 0.53 0.24、流れ D の後 2.48 1.81 1.91。

| 検査 | 結果 | 値 |
|---|---|---|
| 起動(boot.json) | 通った | engine-idle・standIn false・error null・elapsedMs 9,187・coldMs 9,378(受け入れの上限 12,500 以下。P1 の 10 秒の内) |
| 流れ A〜D | 通った(4 行) | 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs 7,896〜8,238 |
| manifest の lazy | 通った | `PluginManager.exe`(7,158,856 B・encodedSize 1,761,276)・`plugin-manager.pluginmanager.cf60000.wasm`(16,085,407 B・encodedSize 2,225,326)・`plugin-manager.pluginmanager.cf60000.dll`(7,158,856 B・encodedSize 1,761,921) |
| PE の表 | 通った | 表の行 16(`ImageBase`・`Machine` を含む。受け入れは 12 行以上) |
| `npm test --prefix app` | 通った | Test Files 75 passed・1 skipped(76)。Tests 1,465 passed・10 skipped(1,475) |
| E2E smoke(2 表示) | 通った | 行は 2 つ。両方 error null・servedHeadersMatch true。achievedFps は 46.25(4k-dpr1)・47.41(4k-dpr2)。`plugin-manager-opens` は両表示で ok false(expectFail #1499。期待どおり)。他の 9 件は両表示とも ok true。`plugin-database-lists-builtins` は両表示で「3 entries listed (3 built-in plugins)」 |
| `guard/scan.sh` | 通った | 0 finding(s) |

注:
- 衝突(`tools/e2e/scenarios/smoke/plugin-database-lists-builtins.mjs`): main の版(`b0ae9519` の `mainRows`・`builtinPluginsOf`)を採った。前回の差分の同じ目的の変更(本体の無いフォルダで throw せず飛ばす `builtinPlugins`・その注釈)は取り除いた。この file は main(`1dc942df`)と同一になる。
- `lift/crates/fe-build/src/plugins.rs`・`inputs/plugin-image-sizes.tsv` は、自動の併合で衝突しなかった。`cargo test` の複写の数(lift/plugins の 164 行)は通った。
- 抽出の木: 受け入れの前に `tools/place-plugin-dlls.mjs`(判定と同じ順)を呼んだ。出力は `{"placed":1,"present":113,"missing":0,"mismatch":0}`。複写した物の sha256 は `66100a71…b2eb32f1e7`(一覧の値と一致)。
- lift の lists(`fe-wasmgen`): `plugin-manager.pluginmanager.dll: listed 21622, emitted ok 21622, listed but not emitted 0`。
- wasm の大きさ: 16,085,407 B(`lift/out-app/bundle/raw/fl-Oz/manifest.json` の size)。
- Options の 6 番目で `fle: ShellExecuteW NOT CARRIED OUT (no association in the virtual machine; S-ia2-shellexecute-noassoc)` が出て、答えは 31(SE_ERR_NOASSOC)。窓は出ない(段 B)。
- E2E の fps: smoke の `achievedFps` は目標の 120 fps に届いていない(段 A の対象外。runner の値)。

判定: 受理(段 A。段 1〜3 と受け入れは通った。段 4〜6 は段 B)。

## 10. 段 B(#1501): 起動・窓・E2E の印

判定: 差し戻し(受け入れは通らず)。exe の入口を同じ engine の協調スレッドで走らせるところまでは実測で届いた。窓は出ていない。

実エンジン: この runner(nproc 4。作業の終わりの loadavg 1.45 2.51 2.63)で、`fe-build engine`・`fe-build bundle` を 3 回実行した(最後の版は 3 回目。最後の 1 行の変更は、この後にビルドしていない)。`boot-bundle`(engine-idle・13,251 ms)・`welcome-flow`(流れ A〜D の 4 行)・`npm test --prefix app`(76 files passed・1 skipped。1,468 passed)・`run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2`(`plugin-manager-opens` だけ ok false。他の 10 本は両表示 ok true)・`guard/scan.sh`(0 finding)・`spec/workflow.test.sh`(602 ok)を観測した。

(a) 起動の要求の経路:
- 観測: Options の 6 番目(Down x 6・Enter)で呼ばれたのは `ShellExecuteW`。verb `open`、lpFile は `C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe`、parameters は ` -flpath "C:\Program Files\Image-Line\FL Studio 21\" -flcaption "FL Studio 2026" -deepscan`。`ShellExecuteExW`・`CreateProcessW` は観測されなかった。
- 行(今の版): `fle: ShellExecuteW starts the program "C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe" as a thread of this engine (verb "open"; parameters " -flpath ..." are its command line's; the directory is not passed on; S-ia2-program-start)`。
- 走ったゲストのスレッド: `ShellExecuteW` が協調スレッドを 1 つ作り(`fle_thread_spawn`)、その本体が exe の AddressOfEntryPoint を引数なしで呼ぶ。終了は `ExitProcess` のときだけ、そのスレッドを終える(engine は止めない)。
- 初回の実測の問題: `[js] fle thread 26 failed: SuspendError: trying to suspend JS frames`。分離 wasm(exe の lazy の wasm)の呼び出しが JS の stub(`app/src/engine/xmod.ts`)を経由するため、待ちで中断できない。#1515 と同じ原因。候補の 07 パッチのうち `xmod.ts` と `xmod.test.ts` の部分を当てて解消した(JS の枠を、wasm の関数表の中継にする。`ott-host.ts` の部分は含めない)。この後の実測で `SuspendError` は 0 件。

(b) 論点ごとの理由:
- `GetModuleHandle(NULL)`: exe のスレッドからは exe の基底を返す。理由: exe の起動は `FindResource(GetModuleHandle(NULL))` で自分のフォームの資源を読むため。他のスレッドは従来どおり `FLE_EXE_HANDLE`。
- `ExitProcess`: exe のスレッドからは、そのスレッドだけを終える(`fle_realmod_program_end` → `fle_thread_end_running`)。他のスレッドは従来どおり止める。
- `hProcess` と `GetExitCodeProcess`: `SEE_MASK_NOCLOSEPROCESS` のときだけ、exe のスレッドのハンドルを hProcess にする(その待ちが exe の終了を表す)。`GetExitCodeProcess` は未実装のまま(観測されていない)。
- コマンドライン: exe のスレッドの `GetCommandLineW/A` は `"<path>"` と ShellExecute の parameters(実測で FL が `-flpath` などを渡していたため)。カレントディレクトリは engine のまま(lpDirectory は観測されず、未対応)。
- 戻り値: 成功は 33。`ShellExecuteExW` は hInstApp 33 と TRUE。動詞が NULL か `open` で、パスが VFS に在る exe の時だけ。それ以外は従来どおり 31(決定 `S-ia2-shellexecute-noassoc` の範囲はこの exe だけ)。`CreateProcessW` は観測されず、変えていない。
- `LoadLibrary` で同じ exe を読む要求は止める(DllMain として呼ばないため)。

(c) 窓の表: 窓は出ていない(未確認)。停止の前に exe 由来の窓として `TApplication`・`TPUtilWindow`・`TLightweightLayeredControl`(lid 8、thread 2)が現れた。Plugin Manager の窓のクラスと題名は確かめられていない。

(d) 流れ A〜D(最後の版。runner の値): 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs 12,595・12,921・12,721・12,601。

(e) `plugin-manager-opens`(両表示): 両方 ok false、expectFail null、engineStops 1。停止は下の (f)。

(f) やること 1〜6 の進み:
- 1 完了: manifest の lazy に `PluginManager.exe`(7,158,856 B)と `plugin-manager.pluginmanager.cf60000.wasm`(16,085,407 B)。
- 2 完了: 上の (a)。
- 3 途中: exe の入口を協調スレッドで走らせる。`GetModuleFileNameW`(exe の基底)・`GetCommandLineW/A`・`VirtualProtect` の一部を、exe のスレッドに合わせた。最後の止まり: exe のコードの 5 バイトの書き換え(`VirtualProtect` 0xcf98f50 と 0xcf98f70。旧 0x20・新 0x40。`jmp rel32` の inline hook と見られる。未確認)。決定 0105 に従い、この変更は止めに戻した(この止めの版は、ビルドも測定もしていない。構文検査のみ)。その前の版では、この書き換えを黙って TRUE で通したため、後の `GetFileVersionInfoSizeW`(exe の版の資源。決定 0098 で仮想ファイルの版資源は未対応)まで進んだ。
- 4 未完了: 窓は出ていない。
- 5 完了: `tools/e2e/scenarios/smoke/plugin-manager-opens.mjs` の `expectFail` を `null` にした。`smoke/index.mjs` の `ALLOWED_MARKS` は `[]`。`spec/workflow.test.sh` の検査を新しい状態に合わせた。注: `tools/e2e/scenarios/plugins/plugin-manager-opens.mjs`(e2e-full。非ゲート)は、まだ印 `#1499` のまま。
- 6 未完了: 受け入れは通らず。起動確認は 13,251 ms(上限 12,500 ms)。この runner では 3 回とも 13.2 秒前後で、段 A の 9,187 ms とは別の runner。今回の変更は起動の経路に入らないが、同じ版での基準は取っていない(未確認)。`plugin-manager-opens` は両表示で失敗。

次の一手(段 B の続き): (1) exe のコードへの書き換え(inline hook)を、決定 0105 の注釈つきの hook 点として扱うか、exe の入口の前に起きる書き換えの元を特定する。(2) `GetFileVersionInfoSizeW`/`GetFileVersionInfoW`/`VerQueryValueW` を、exe の RT_VERSION 資源から答える(決定 0098 の拡張)。(3) 窓が出た後に、Plugin Manager の窓のクラスと題名を、この表に書く。

## 11. 段 B(#1501)の再試行: 受理

判定: 受理(受け入れの全部が通った。窓は出て、engine は 10 秒 engine-idle のまま)。

実エンジン: この runner(nproc 4。作業の終わりの loadavg 1.34 2.70 2.71)で、`fe-build engine`・`fe-build bundle`・`boot-bundle`・`welcome-flow`・`npm test --prefix app`・`run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2`・`judge.jq`・`guard/scan.sh` を実行し、出力を観測した。加えて `spec/workflow.test.sh`(602 ok)・`spec/guard.test.sh`(104 ok)を通した。runner の値。この機械とは別枠。

(a) 起動の要求の経路(観測): Options の 6 番目(Down x 6・Enter)で呼ばれたのは `ShellExecuteW`(verb `open`、lpFile は exe の完全なパス、parameters は ` -flpath "C:\Program Files\Image-Line\FL Studio 21\" -flcaption "FL Studio 2026" -deepscan`)。`ShellExecuteExW`・`CreateProcessW` は観測されなかった。engine のログの行: `[err] fle: ShellExecuteW starts the program "C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe" as a thread of this engine (verb "open"; parameters " -flpath ..." are its command line's; the directory is not passed on; S-ia2-program-start)`。走ったゲストのスレッドは協調スレッド 1 本(`fle_thread_spawn`)で、本体は exe の AddressOfEntryPoint を引数なしで呼ぶ。exe の `ExitProcess` はそのスレッドだけを終える(engine は止めない)。この run では exe は終了せず、窓が残った。

(b) 論点ごとの理由:
- `GetModuleHandle(NULL)`: exe のスレッドからは exe の基底(段 B の前回の判断どおり。exe の起動は自分の資源を `FindResource` で読む)。
- `ExitProcess`: exe のスレッドからは、そのスレッドだけを終える。
- `hProcess`: `SEE_MASK_NOCLOSEPROCESS` のときだけ、exe のスレッドのハンドル。`GetExitCodeProcess` は未実装のまま(観測されていない)。
- コマンドライン: exe のスレッドの `GetCommandLineW/A` は `"<path>"` と ShellExecute の parameters。カレントディレクトリは engine のまま(lpDirectory は観測されず、未対応)。
- 戻り値: 成功は 33。動詞が NULL か `open` で、パスが VFS に在る exe の時だけ。それ以外は従来どおり 31(決定 `S-ia2-shellexecute-noassoc` の変更範囲は exe だけ)。

今回の再試行で足したもの(この作業の差分。commit `95cd4fde`・`4e9239b0`・`80f269da`):
- 方式の hook 点(exe の Delphi RTL の inline hook)。`lift/plugins/plugin-manager/plugin.tsv` の hook 列は `rva:0x1b650,0x38f50,0x38f70`。根拠は objdump の逆アセンブル: 5f6e00 が 3 つの hook 対象(0x9dd0a8・0x9dd0b0・0x9dd0b8)を据え、5fa97b・5fa9ac・5fa9cb が、それぞれ r8 = 0x41b650・0x438f50・0x438f70(原像の VA。ImageBase 0x400000 からの RVA は 0x1b650・0x38f50・0x38f70)、r9 = 0x5f1280・0x5f1170・0x5f11e0(差し替え先)で、5f6cf0 が `VirtualProtect(target, 5, 0x40, &old)` を呼ぶ。先頭 5 バイトは 55 56 53 c6 44・53 48 83 ec 20・55 48 83 ec 50 で、`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` と同じ 3 種の型。段 B の前回の止めの 0xcf98f50・0xcf98f70 は、この 2 つ(base 0xcf60000 + RVA)。
- exe の RT_VERSION: `GetFileVersionInfoSizeW`・`GetFileVersionInfoW` は、起動した exe のパスに対して、その image の RT_VERSION を答える(`fle_res_version_block_of`。決定 0129 の engine の版と同じ扱い)。
- `PathCanonicalizeW`(新規。Learn の規則: "." は飛ばし、".." は前の区切りを消す。根の区切りは消さない。例 4 件と `C:\..` を standalone で照合した)。
- `OpenFileMappingW`(新規。名前付きの物は run の中に作られない(CreateFileMappingW は名前で止まる)ので、ERROR_FILE_NOT_FOUND)。
- `memset`(新規。C の定義どおり)。
- `CreateMutexW`・`ReleaseMutex`(新規。event の表の自動リセットの物。初めは信号。名前は run の間だけ覚え、二度目は ERROR_ALREADY_EXISTS。読みの選択: bInitialOwner は使わない)。
- `TextOutW`(新規の名前付きの停止。`GDI_TEXT_STOP`。文字の描画は決定 0111 のとおり未対応のまま)。
- `lift/harness/hyb/c-imports-engine.txt`: 75 行を足した(73 行は exe の thunk が「C で答えられる」と分かる名前。C の実装が在るもの。CreateMutexW・ReleaseMutex の 2 行を含む。engine の他の関数の下ろし方も変わる。同じ C の関数を呼ぶので意味は変えないが、差は実測では見ていない)。足さなかった 15 件(`ShellExecuteA`・`ShellExecuteW`・タイマー・`MessageBoxW`・`TrackPopupMenu`・`DoDragDrop`・`CallNextHookEx`・`SendMessageA`・`DispatchMessageA`・`EnumSystemLocalesW`・OLE のクリップボード): スレッド・モーダルの待ち・コールバックを含み、wasm に下ろすと協調スレッドの中断の問題(#1515 の形)が起きうるため。exe の thunk がそれらに届いたら、名前付きでなく "unsupported" で止まる。

(c) 窓の表: 窓が出た。クラス `TFLStudioPluginScannerMainForm`、題名 `FL Studio Plugin Manager v0.0`(E2E の notes。両表示)。題名の `v0.0` は未確認: この run では exe 自身の GetFileVersionInfo は呼ばれておらず(ログに出るのは FL64.exe の分だけ)、exe がどこから版を読むかは確かめていない。

(d) 流れ A〜D(最後の bundle。runner の値): 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs 11,906・11,985・11,940・11,738。起動確認(boot.json)は engine-idle・standIn false・error null・elapsedMs 12,229。

(e) `plugin-manager-opens`(両表示): 両方 ok true・expectFail null・engineStops 0・engine-idle(10 秒)。E2E smoke の judge.jq は通った(true)。`npm test --prefix app` は 76 files passed・1 skipped(1,470 passed・10 skipped)。`guard/scan.sh` は 0 finding。

(f) やること 1〜6 の進み:
- 1 完了(manifest の lazy に exe と wasm)。
- 2 完了(上の (a))。
- 3 完了: exe の入口を同じ engine の協調スレッドで走らせ、(b) の論点を決めた。今回の差分で、exe の RTL の hook 点・RT_VERSION・上の 5 つの API を足した。
- 4 完了: 窓が窓の表に出た(c)。
- 5 完了: smoke の `plugin-manager-opens` の印は外れている(前回の差分)。e2e-full の `tools/e2e/scenarios/plugins/plugin-manager-opens.mjs` は非ゲートのまま印 `#1499` が残っている(生成物。`_generate.mjs` の由来。受け入れの外)。
- 6 完了: 受け入れが通った。

残り(未確認・未着手):
- 文字の描画(ExtTextOutW・DrawTextW・TextOutW・GetTextExtentPoint32W・GetTextMetricsW)は、決定 0111 のとおり名前付きの停止のまま。この run では、exe の窓がこれらを呼ぶ前に窓の表に出た(停止は出なかった)。窓の中身が描かれるかは未確認。
- 未実装の import(この exe の 576 件のうち、C の定義が runtime に無いもの。CreateMutexW を足した後で約 45 件): 例 `AddVectoredExceptionHandler`・`AssignProcessToJobObject`・`CreateProcessW`・`GetExitCodeProcess`・`GlobalSize`・`GlobalDeleteAtom`・`HeapDestroy`・`IsBadWritePtr`・`SetEndOfFile`・`VirtualQueryEx`・`RtlUnwind`。届いたら、thunk の場合は名前の付いた停止ではなく "unsupported" で止まる。
- 題名の `v0.0`(c)。
- exe の `0x73c670`(8 バイトを書く別の hook の経路。VirtualProtect の大きさが 8)は、この run では届いていない。届いたら、ih_protect の規則(5 バイトの hook 点だけ答える)で止まる。
