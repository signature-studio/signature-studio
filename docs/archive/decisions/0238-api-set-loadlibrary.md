# 0238: LoadLibrary / GetModuleHandle が API set の契約名を『契約 → ホスト DLL』で扱う。F2xH6(oggio と Elastique が実モジュール)の bundle が参照プロジェクトを停止なしで開いて再生し、Problems 窓の『2 sample(s)』の行が消えた — 仮の決定(load2)

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0238〔仮番号〕、2026-11-05 02:04〜、総計 4 時間)、作業者 `load2`。作業と測定の基準 HEAD は `ba85d14b`。F2xH6 のプリセットは `lift/out-git/ship4-results/ship4-h6.diff`(決定 0230)を**前提**にして作業した。`load2.diff` は `ba85d14b` の `git archive` にも、`ba85d14b` + ship4-h6.diff にも、**PM の最新 HEAD `6c60cc22` の `git archive` にも** `patch -p1 --dry-run` が通る(rc=0、offset・fuzz・reject なし。`ba85d14b`〜`6c60cc22` で `lift/runtime`・`lift/crates/fe-lift`・`harness/noreturn-audit` は無変更、`lift/crates` の変更は fe-wasmgen の 17 行だけなので、生成 C の sha256 の測定は 6c60cc22 にもそのまま当たる〔読み〕)。2 つの差分(load2 と ship4-h6)は同じファイルを触らない。

## 冒頭の三点

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL + `oggio_x64.dll` + `Elastique_x64.dll` を x86-64 → C → wasm にした物。代役・再生・画像・期待値の貼り付けは 0)を、**CI の段スクリプト**(objects → link → seed → bundle、F2xH6、ハイブリッド込み)で作った bundle で動かした。(a) node(JSPI・仮想時計)で ref1 の G3(File>Open で参照プロジェクトを選ぶ・Enter・idle 30 s・Space・再生 30 s・Space)を、診断トレース付きの bundle 3 本(t1・t2・t4、`NO_FS=0`)で、(b) 隔離 Chrome 156(CDP の実時間入力、出荷形 `NO_FS=1` のクリーンな bundle c3)で File>Open → Problems 窓 → Enter → Space → 30 秒再生 → Space、(c) 起動 16 スナップを F2xH5 の FS bundle で変更前(b1)・変更後(a1)、(d) native テスト・cargo 全体・a4・mutation を実際に走らせた。この機械(16 コア、load 2〜26 で他の担当と共用)で、依頼者の機械ではない。入力はすべて模擬(CDP・G3)で人の操作ではない。
2. **結論**:
   - **壁は 2 個だけだった**(§4): (1) `LoadLibraryExW` に渡る API set の契約名(oggio の CRT 初期化)、(2) oggio の CRT のファイルを開く処理(`_wsopen_nolock`: `CreateFileW` の直後)が呼ぶ `GetFileType`(名前つき停止、決定 0078)。契約実装の後、F2xH6 の bundle は**参照プロジェクトを停止なしで最後まで通る**(node: 10 スナップ全部。『Elastique DLL couldn't be opened』の MessageBox は 0 回〔F2xH5 では出る〕。`oggio_x64.dll`・`Elastique_x64.dll` は各 1 回の `LoadLibraryW` で実モジュール〔0xc000000・0xb000000〕になり、`OggioLoadFile` / `OggioLoadFileW` / `OggioFreeInfo` / `ElAllocator_SetFunctions` が実 export に解決)。
   - **Chrome 156 の Problems 窓は『8 plugin(s) could not be opened / and the plugin(s) will be ignored.』だけになり、『2 sample(s) required by this project could not be located』の行が消えた**(スクリーンショットを目で読んだ。同じ手順の F2xH5 の bundle〔ref3 の `assets-A`〕では、自分の実行で『8 plugin(s) could not be opened / 2 sample(s) required by this project could not be located. / This is not a serious error, the project will continue to load normally / and the plugin(s) will be ignored.』と 4 行出た〔`shots/h5-1-problems.png`〕。F2xH6 では『2 sample(s)』の行と『This is not a serious error …』の行がともに無い)。2 本の Vorbis(`anvil_land.ogg`・`dltzk glass break 1.wav`)は oggio の CRT が `CreateFileW` → `GetFileType`(戻り 1 = DISK)→ `SetFilePointerEx` ×18 で実際に開いて読んだ(node のトレース、§5.2)。
   - **実装は契約のみ**(§2、すべて (b) 外部 API の契約の実装。FL のクラス名・窓・DLL 名・アドレスによる分岐なし): API set スキーマ(Wine 10.0 の `apisetschema.spec` 511 行、UNOFFICIAL、データ)と `fle_apiset_resolve`(バージョン末尾を除いたキーで引く)、`LoadLibrary*` はホストが仮想モジュールならそのハンドル +1 参照、ホストなしの項は NULL/126、ホストが仮想モジュールに無い契約名は名前つき停止(ホスト名つき)、`GetModuleHandle*` はホストが読み込み済みならそのハンドル。仮想モジュール `kernelbase.dll`(N=40)と `combase.dll`(N=41)を追加(export の行なし = GetProcAddress は NULL/127)。決定 0206(dec1)が opt-in の harness wrap でだけ答えていた `GetFileType` / `SetFilePointerEx` / `FlushFileBuffers` / `FindFirstFileExW` を runtime の契約実装に昇格。
   - **検証(実エンジン・実測)**: 起動 16 スナップは F2xH5 の FS bundle で変更前後が **84 行全行一致**。FLEngine の既定の生成 C の sha256 は **`cd9107ce…e50`(215,811,678 B)で不変**(`lift/crates/*/src` は 1 行も変えていない)。cargo 全体 `test.sh full` は **194 results / 2,381 passed / 0 failed**(基準 193 / 2,369 → +1 result〔`load2_files.rs`〕、+12 passed)。a4 FRUITY 無効は **942/942・895/895・functions 5,310・entries 1,009,737 で不変**。F2xH6 の wasm は **146,033,464 B**(ref3 の基準 146,011,644 B + 21,820 B。S1 の線 154,885,888 B より 8,852,424 B 下 = ネイティブ 77,442,944 B の 1.886 倍)。mutation 10 本はすべてテストが落ちて検出。
   - **決定 0233 §10 の訂正**(§8): 『Elastique を使うクリップは出荷構成〔F2xH5〕では鳴らない』は、この参照プロジェクトでは成り立たない。F2xH5 でも Elastique 系のチャンネルのミキサーメーターは動く(エンジンが別の経路で鳴らす読み)。
3. **未確認**: ① Elastique の**出力が正しい音か**は実機 Windows と比べていない(PCM の F2xH5 / F2xH6 の A/B は位置が合わず〔相関 0.09〕判別に使えなかった。Elastique が動く証拠は、MessageBox が出ない・実 export の解決・node の再生 CPU が 49.6 → 97.7 s・Chrome の Elastique 系インサートのメーター、の間接のみ)。② スキーマは Wine 10.0 の表で、実機 Windows 10 のスキーマそのものではない(S-load2-schema-data)。表に無い契約名は『ファイル探索 → 126』になる(S-load2-absent)。③ 契約名のうち実際に引かれたのは string / datetime / localization-obsolete(→ kernelbase)の 3 つ。file・processthreads・sysinfo・xstate・winrt・ntuser・security・kernel32-package は oggio の文字列にあるが、この再生では CRT が引かなかった(テストでのみ確認)。④ 依頼者の機械・Firefox 159・実マウスでは未実施。⑤ ReadFile の末尾読み(決定 0206 §9)は触っていない。⑥ F2xH7 ベースへの載せ替え(PM の言う F2xH9 相当)は未実施。⑦ **出荷 bundle がランタイムの静的配置によって Chrome の起動直後に `CreateIconIndirect` の名前つき停止になりうる**(§9、別文書 0238b。この差分も配置を変えるが、c3 は 21/21 で停止なし = 運のよい配置の可能性)。

## 1. 契約の出典(原文は `tables/contracts/`、対応は `tables/contracts.tsv`)

取得は 2026-11-05(Learn の HTML から本文だけを `tools/html2txt.py` で取り出した)と Wine 10.0 のソース(tag `wine-10.0`、commit `b073859675060c9211fcbccfd90e4e87520dc2c2`、sparse clone)。

| 出典 | 文 | runtime での扱い |
|---|---|---|
| Learn『Windows API sets』 | 『An API set name identifies a *contract*, not a file. At run time the loader resolves that contract through the API set schema on the current device, and routes the reference to the DLL that hosts the implementation.』 | 契約名 → ホスト DLL をスキーマの表で引く |
| 同 | 『The loader recognizes a name that begins with `api-` or `ext-` and routes it to the API set runtime ... the `.dll` suffix isn't part of the contract name being resolved.』 | 接頭辞(`api-` / `ext-`、5 文字以上)で判定。拡張子は引くキーに入れない |
| 同 | 『You can pass an API set name to LoadLibrary ... The operation succeeds when the schema on the current device maps that contract to a usable host; there isn't necessarily an actual file with that name anywhere on the PC. If the contract isn't mapped on the current device, a direct LoadLibrary fails.』 | ホストが仮想モジュール: そのハンドル。写像されない: 失敗 |
| Learn『API set loader operation』 | 『Resolution is a three-step sequence: ... finds the host binary that the schema maps it to. The loader loads that host binary and binds the imported function to the host's export.』 | LoadLibrary(契約名) = **ホストのハンドル**。GetProcAddress はホストの名前で応答表を引く |
| Learn『Detect API set availability』 | 『a given API set contract name might be intentionally mapped to an empty module name on some Windows devices』 | スキーマにあってホストなしの項(3 件): NULL + 126 |
| Learn『Dynamic-link library search order』 | 標準の探索順に『DLL redirection → API sets → SxS → loaded-module list → known DLLs → ...』 | API set は探索順の 1 段。表に無い名前は次の段(volume の探索)へ進む |
| Learn LoadLibraryW / GetModuleHandleW / GetProcAddress | 失敗は NULL(番号の文は無い) | 126 / 127 は決定 0015 の番号 |
| Wine 10.0(**UNOFFICIAL**) | `dlls/apisetschema/apisetschema.spec`(511 行 `apiset <契約> = <ホスト>`、ホストなし 3 件)。`dlls/ntdll/loader.c` `get_apiset_entry`(キーは『最後の `-` より前』のハッシュ = サブバージョンは引かない)、`get_apiset_target`(値なし → STATUS_DLL_NOT_FOUND)、`find_apiset_dll`(スキーマに無い → STATUS_APISET_NOT_PRESENT でファイル探索が続く)、`LdrGetDllHandleEx`(ホストのパスの読み込み済みモジュール) | 表のデータ、サブバージョン・ホストなし・表に無い名前・GetModuleHandle の規則 |

Learn は契約名とホストの表を**出していない**(実機のスキーマは `apisetschema.dll` / PEB の ApiSetMap にあり、ここでは読めない)。表は Wine 10.0 の物で、実機 Windows 10 の契約名の一部(Wine は 511、実機はもっと多い)。

## 2. 実装(`load2.diff`。表: `tables/boundary.tsv`)

`lift/runtime/fle_apiset_schema.inc`(新規、データ。`docs/investigations/load2/tools/mk-apiset.py` が `docs/investigations/load2/tables/wine10-apisetschema.spec` から生成): `{ キー, ホスト }` 511 行(キー = 契約名から最後の `-<n>` を除いた物。strcmp 昇順、重複なし)。

`lift/runtime/fle_modules.c` / `.h`:
- `fle_apiset_resolve(name, &host)`(小文字・拡張子つきの正規化名): `NOT_A_CONTRACT`(接頭辞なし・4 文字以下)/ `UNKNOWN`(接頭辞はあるが表に無い)/ `NO_HOST`(表にあってホストなし)/ `HOST`(ホスト名)。キーは Wine の `get_apiset_entry` と同じく『最初の `.` までの最後の `-` より前』。
- 仮想モジュール `kernelbase.dll`(N=40)と `combase.dll`(N=41)を**末尾に追加**(決定 0015: append only)。スキーマの 511 契約のうち 123 が kernelbase、13 が combase にある。export の行は無い(S-load2-kernelbase-combase)。
- `GetModuleHandleW/A`: 表の名前・実モジュールで見つからず、契約名でホストが仮想モジュールなら、そのハンドル。ホストが無い・ホストなし・表に無い: NULL + 126(『読み込まれていない』)。

`lift/runtime/fle_load.c`: `is_apiset_name`(『`l<n>-<n>-<n>` の形』の判定)を除去し、`api_set_load` に置き換えた。順序は 実モジュール → 仮想モジュール(名前一致。**契約名の 5 個〔N=2、27〜30〕は自分のハンドルのまま**)→ 『無い』と定義済み → **契約名**(`api_set_load`)→ 名簿(決定 0233)→ volume の探索。
- ホストが仮想モジュール: そのハンドルと**ホストに 1 参照**。
- ホストなし: NULL + 126。
- ホストが仮想モジュールに無い(ucrtbase・wintypes・sechost ほか): 名前つき停止『the API set contract "…" is mapped to the host "…" … this environment has no virtual module for that host』(PM の指示)。
- 表に無い名前: 通常の裸の名前として volume を探す(無ければ NULL + 126、ファイルがあれば名前つき停止)。

`lift/runtime/fle_vol.c`(`GetFileType`)と `fle_vfs.c`(`SetFilePointerEx` / `FlushFileBuffers` / `FindFirstFileExW`): 決定 0206(dec1)が `harness/dec1/dec1_imp.c` ほかの `--wrap` でだけ答えていた 4 つを runtime の契約実装にした(Learn の引用つき、`tables/contracts.tsv`)。
- `GetFileType`: volume の開いているファイルは 1(DISK)。NULL・INVALID_HANDLE_VALUE・閉じたハンドル・find ハンドルは 0 + 6(番号は推測)。volume が発行していない値は従来どおり名前つき停止。
- `SetFilePointerEx`: `SetFilePointer` と同じファイルポインタ。位置が負 → 0 + 131(`SetFilePointer` のページにある番号)。`FILE_BEGIN` で 2^63 以上・方法が 2 を超える値は停止。
- `FlushFileBuffers`: 書込権のあるハンドルは 1、書込権なしは 0 + 5(推測)、無効は 0 + 6。
- `FindFirstFileExW`: level 0/1・op 0/1・filter NULL・flags 0 か LARGE_FETCH は `FindFirstFileW`。他は停止。
- harness の wrap(opt-in)は残る(`__real_` が runtime の実装になるだけで二重定義にならない)。`fle_vol.h` の `noreturn` 宣言を外し、`harness/noreturn-audit/allow.tsv` の該当 10 行(GetFileType 4・SetFilePointerEx 4・FlushFileBuffers 1・FindFirstFileExW 1)を除いた。

### (a)(b) の区分(lift の忠実性)
新しい処理は**すべて (b) 外部 API の契約の実装**: API set のスキーマ(Learn の『契約 → ホスト』)、ホストのハンドル(Learn『the loader loads that host binary』)、4 つの Win32 関数(Learn の戻り値・失敗)。FL のクラス名・窓の種類・DLL 名(oggio・Elastique を名指ししない)・アドレスによる分岐は **0**。余計な処理の除去 (a): `is_apiset_name`(契約名の形による停止の判定)。通信・変換の削減は無関係。**既存の動作が変わる所**は §7。

## 4. 壁の表(`tables/stop-table.tsv`、実エンジン)

| 壁 | 実行 | API・引数 | ゲストの戻り番地 | 出たもの | 直し |
|---|---|---|---|---|---|
| W1 | PM / ref3 の測定(**load2 は再実行していない**。コードで確認) | `LoadLibraryExW("api-ms-win-core-string-l1-1-0.dll")` | 0xc071f89(oggio の起動) | 名前つき停止(決定 0233 の『API set の形の名前は仮想モジュールでなければ停止』) | スキーマで kernelbase.dll に解決、kernelbase.dll を仮想モジュール N=40 に |
| W2 | node G3(t1、診断トレース、`NO_FS=0`) | `GetFileType` | 停止メッセージには出ない。t4 で測った: oggio の CRT のファイルを開く処理 `_wsopen_nolock` の 0xc07a81f(戻り 0xc07a825) | `fle: GetFileType: no standard handle exists …`(`fle_vol.c`、決定 0078) | `GetFileType`(と同じ系統の `SetFilePointerEx` / `FlushFileBuffers` / `FindFirstFileExW`)を runtime の契約実装に |
| - | node G3(t2・t4) | (停止なし) | - | 10 スナップ全部。MessageBox なし。oggio・Elastique が実モジュール | - |
| - | Chrome 156(c3、クリーン) | (停止なし) | - | Problems 窓から『2 sample(s)』が消えた。再生できる | - |

以降の壁(2 本の Vorbis の読み込み・Elastique のロード・再生)は**出なかった**。探した範囲: node の G3 全体(開く → 30 s idle → 30 仮想秒の再生 → 停止)と Chrome の 30 秒の再生。oggio の import で行の無い物は 3 つ(`PeekNamedPipe`・`GetTimeFormatW`〔oggio〕、`HeapQueryInformation`〔Elastique〕)あるが、この手順では踏まなかった(踏めば『unsupported IAT slot』の停止になる。未確認の壁として §6 に残す)。

## 5. 検証(実エンジン。すべてこの機械、load を併記)

### 5.1 node(JSPI・仮想時計)、ref1 の G3(`tools/refproj.sh`)

bundle は CI の段スクリプト(`tools/iter.sh` が ref3 の `ciH6`〔HEAD ba85d14b + ship4-h6.diff の F2xH6〕の複製 `ciH6base` を hard link で複製し、変えた runtime の .o だけ消して `objects link seed bundle`)で作った F2xH6(ハイブリッド込み、`COMPRESS=none`、`NO_FS=0`)。volume は bundle 自身の seed + 参照プロジェクトの 49 ファイル(nested の置き場、`tables/vfs-seed-nested.tsv`)。manifest の env に探索スイッチなし。

| 実行 | 結果 |
|---|---|
| t1(壁 W1 を越えた直後) | 2 スナップ目の後で GetFileType の名前つき停止(W2) |
| t2(W2 の直し込み) | 10 スナップ全部。user CPU 145.1 s(load 6〜12)、再生 30 仮想秒の部分 97.7 s。MessageBox なし |
| t4(ファイル呼び出しのトレース付き) | 10 スナップ全部。user CPU 164.0 s(load 7〜10)。`CreateFileW` 378 回、`GetFileType` 2 回、`SetFilePointerEx` 18 回、`FlushFileBuffers` 0、`FindFirstFileExW` 0 |
| 比較(ref3、F2xH5、同じ手順、load 異なる) | 再生 30 仮想秒の部分 49.6 s(user CPU) |

CPU 秒は load で 10〜25% 動く。F2xH6 の再生が F2xH5 の約 2 倍の CPU を使うのは、mt2(決定 0234)の『Elastique が再生中のミキサーの CPU の 74%』と向きが合う(読み)。

### 5.2 2 本の Vorbis が読まれた証拠(t4、`tables/trace-t4.tsv`)

```
CreateFileW  ret=c07a790 path="C:\Users\user\Downloads\serina1-serum2\anvil_land.ogg"   -> 7ffa00000510
GetFileType  ret=c07a825 a1=7ffa00000510 -> 1
SetFilePointerEx ret=c06ff96 a1=7ffa00000510 ... ×(この handle で一連)
CreateFileW  ret=c07a790 path="…\dltzk glass break 1.wav" -> 7ffa0000054c
GetFileType  ret=c07a825 a1=7ffa0000054c -> 1
```

(戻り番地 0xc07a790 / 0xc07a825 / 0xc06ff96 は oggio_x64.dll の CRT の `_wsopen_nolock` と `_lseeki64`。ほかの `CreateFileW`〔戻り 0x674a82 など〕はエンジン自身の読み。)

### 5.3 Chrome 156(実時間・CDP の入力はハーネス = 人の選択の代役)、クリーンな出荷形(`NO_FS=1`)の bundle c3

`vite preview`(自前の build の `app-run`)で配り、`?debug=1` のページを隔離 Chrome で開いた(`tools/sdrive-ref3.mjs`、シナリオ `scn-ref3.json`)。ウィザードを閉じる → ページのファイル窓に 49 ファイルを渡す → エンジンの File>Open で `serina1-serum2.flp` を選び OK → Problems 窓(`shots/1-problems.png`)→ Enter → 6 秒 → Space → 30 秒再生 → Space。

- **`shots/1-problems.png` を目で読んだ**: 窓の題『Problems loading the project』、本文『8 plugin(s) could not be opened』『and the plugin(s) will be ignored.』、ボタン Ok / Missing plugins help / Missing samples info。**『2 sample(s) required by this project could not be located』の行も、それに続く『This is not a serious error, the project will continue to load normally』の行も無い**(同じ手順の F2xH5 の `shots/h5-1-problems.png` には 4 行とも出ている)。プレイリストには `[fadr.com] Vocals`・`car crash and siren`・`Other - Dot To Dot`・`schnozz atmos pad` などの波形が描かれている。
- `shots/4-play-end.png`: 再生中(時計 8:04:04、再生線が 8 小節目付近)、ミキサーのマスターと Elastique 系の 3 本のインサート(`[fadr.c.m - Cmin`、`Other - ... C#maj`、`schnozz... (Gmaj)`)のメーターが動いている。
- 音(ページ自身の統計、`play30`): 30.7 audio 秒、`nonSilent` +9,930 / `playingQuanta` +10,587、peak 1.000、rms(累積)0.371。**load 10 の機械で壁時計が追いつかない**(仮想時計 +11.63 s が実時間 +30.95 s = rtf 0.376)ので、underrun・途切れの数字は P3 の評価に使わない。停止なし(`state=engine-idle`)。
- Elastique の A/B(同じ手順、F2xH5 の bundle = ref3 の `assets-A`): F2xH5 でも同じ 3 本のインサートのメーターが動く(`runs/chrome-h5a/4-play-end.png`)。→ §8。
- PCM を F2xH6 / F2xH5 で突き合わせる試み(`tools/cap-ab.py`、相互相関): 位置が合わず(正規化相関 0.09)、判別に使えなかった。

### 5.4 起動 16 スナップの同一(F2xH5、`tools/fsv.sh`、`tools/g3bundle.sh`)

F2xH5(ref3 の `ciA`)の FS bundle を、変更前の木(`git archive HEAD`)と変更後の木で作り、node で `cmp-short.g3` を走らせた: b1(変更前)と a1(変更後)の `g3-snap` / `g3-progress` / `g3-surface` / `g3-wintable` と `fruity: == stop` / `runtime message` の **84 行が全行一致**(`cmp` rc=0、`tables/g3-lines-b1-84.txt`)。ユーザー CPU 13.83 s / 13.69 s。F2xH5 の bundle には kernelbase・combase の追加でモジュールのハンドルが変わる所があるが、スナップに出る値は変わらなかった。

### 5.5 生成 C・全体テスト・a4・mutation

| 項目 | 結果 |
|---|---|
| FLEngine の既定の生成 C(`fe-lift lift FLEngine_x64.dll X --pdata`、HEAD の CI `felift` 段の fe-lift) | sha256 **`cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`**、215,811,678 B — **不変**(`logs/felift-default.sha256`)。`lift/crates/*/src` は 1 行も変えていない(テストだけ) |
| `bash lift/harness/test.sh full --jobs 3`(cargo、`logs/test-full-1.log`・`test-full-2.log`) | **194 results(190 test executables + doc-tests 4)/ 2,381 passed / 0 failed**(基準 193 / 2,369)。1 回目に `wall_fr8` の仮想モジュール数の期待(39 → 41)が 1 本落ちたので期待を直して再実行(キャッシュ済みの 189 executable + 再実行 1 = 全部緑) |
| a4 FRUITY 無効(`lift/harness/a4-fruity.sh off`、`logs/a4-off.log`) | `init_count=942 / 942`、`run_rank=895/895`、functions executed (distinct) **5,310**、function entries **1,009,737**、`== stop: returned from entry`、harness exit 0 — **不変** |
| F2xH6 の wasm(クリーン、`NO_FS=1`、c3) | **146,033,464 B**(ref3 の基準 146,011,644 B + 21,820 B)。S1 の線(ネイティブ 77,442,944 B の 2 倍 = 154,885,888 B)より **8,852,424 B 下**(1.886 倍) |
| native テスト | `load.rs`: 新 8 本(`load2_*`)+ 旧テストの期待の更新。`load2_files.rs`: 新 4 本。関連 15 binary が 0 failed(§6) |
| mutation(`tools/mutation.sh`、`tables/mutation.tsv`) | 10 本すべて、テストが落ちて検出(キーがサブバージョンを持つ・ホストなしを成功にする・接頭辞が `api-` だけ・参照を足さない・ホストなしを停止にする・GetModuleHandle が契約名を無視する・ホスト名の前方一致・4 文字以下を契約名にする・LoadLibrary がスキーマを見ない・ホストの無い契約名を失敗にする) |

## 6. テスト(`load2.diff`)

新: `load.rs` に `load2_*` 8 本(契約名 → ホストのハンドルと参照、サブバージョン・大文字小文字・拡張子なし、表に無い名前・ホストなしの失敗とファイルのある場合の停止、ホストが仮想モジュールに無い契約名の停止とメッセージ、GetModuleHandle、GetProcAddress は NULL/127、スキーマの形〔昇順・一意・小文字・CRT が引く契約のホスト〕、`fle_apiset_resolve` の 4 値)、`load2_files.rs` に 4 本(GetFileType / SetFilePointerEx / FlushFileBuffers / FindFirstFileExW)。

旧テストの期待の更新(`-` 行は全部読んだ。意味を消した物は無い。停止の期待が応答の期待に変わった物、モジュール数 39 → 41、除去した宣言):

| テスト | 旧 | 新 |
|---|---|---|
| `load.rs` `windows_dlls_that_are_not_virtual_modules_and_api_set_names_stop_by_name` | 契約名 4 つも停止 | 契約名は新テストへ。名簿の 8 名は停止のまま |
| `load.rs`・`deferred.rs`・`msctf.rs` の『表の末尾より先』 | M(39)・M(40) など | モジュール数 41 に合わせて +2 |
| `modules.rs` `name_rows` ほか 3 本 | `api-ms-win-core-libraryloader-l1-1-0.dll`・`api-ms-win-core-synch-l1-2-1.dll` は『無い』 | kernelbase のハンドル(表に無い名前はスキーマに行く) |
| `modules.rs` の表のテスト、`wall_fr5.rs`・`wall_fr6.rs`・`wall_s.rs`・`wall_fr8.rs` | count 39 | count 41(kernelbase.dll・combase.dll) |
| `k32_dsp.rs` | `STOPPED` に GetFileType ほか 4 | `ANSWERED` へ(21 本) |
| `noreturn_audit.rs` と `allow.tsv` | GetFileType の宣言を『plain にした』変異と、fle_vol.h を include の閉包に入れる変異 | 同じ構造のまま、対象を RtlPcToFileHeader / fle_proc.h に変更。4 関数の `allow.tsv` 10 行を除去 |

## 7. PM の判断点(既存の動作が変わる所)

1. **契約名の既定が『名前つき停止』から『ホストが仮想モジュールならそのハンドル』に変わる**(ホストなしは NULL/126、ホストが仮想モジュールに無いときだけ名前つき停止)。決定 0233 の PM の指示(i)『API set の形の名前で仮想モジュールでない物は停止』のうち、ホストが仮想モジュールで解決できる物が成功になった。
2. **表に無い契約名(接頭辞は `api-` / `ext-`)は停止でなく『ファイル探索 → NULL/126』**(S-load2-absent。Wine の `find_apiset_dll` と同じ。表は Wine の 511 行で実機のスキーマより小さい。元の 0233 の方針〔停止〕に戻すなら `api_set_load` の `default:` を停止にする 1 行)。
3. **仮想モジュールが 2 個増えた**(kernelbase.dll N=40、combase.dll N=41)。`LoadLibrary("kernelbase.dll")` / `GetModuleHandle("kernelbase.dll")` は 0233 の名簿の停止 / NULL・126 から成功になる。**census / 名簿(決定 0233)との整合**: 仮想モジュールは名簿より先に調べるので、名簿に kernelbase.dll・combase.dll が残っていても矛盾しない(名簿は『仮想モジュールでない物』にだけ働く)。`census_report.py` が拾うのは停止行なので、この 2 名は拾われなくなる(解決されたから)。
4. **export の行が無い**(GetProcAddress は NULL/127)ことの意味: 実機の kernelbase・combase は多くの関数を export する。CRT は NULL のときの代替の経路を使う(決定 0081 の 4 契約モジュールと同じ形で、dsp_ippv2 の起動で既に起きている)。この環境で『export がある』ことを答えるには応答表に kernelbase / combase の行を足す(S-load2-kernelbase-combase)。
5. 契約名の既存の仮想モジュール 5 個(N=2、27〜30)は**スキーマを通らず自分のハンドルを返す**まま(同じ契約の別の sub-version はスキーマ → kernelbase、という非対称が残る: `api-ms-win-core-synch-l1-2-0` は N=27、`-l1-2-1` は kernelbase)。N=27〜30 を表から外すとモジュールのハンドルが後ろにずれる(append only)ので触っていない。
6. `GetFileType` / `SetFilePointerEx` / `FlushFileBuffers` / `FindFirstFileExW` の既定が『停止』から応答になった(opt-in の harness wrap は残る)。`ReadFile` の末尾読み(決定 0206 §9、FLAC が欠ける)は**触っていない**。
7. F2xH7 ベースへの載せ替え(PM の言う F2xH9 相当): load2 の差分は preset に依存しない(runtime・テスト・データ)ので、`ship4-h6.diff` と同じ手順で当てられる。**実機確認はしていない**。

## 8. 決定 0233 §10 の訂正(PM が 0233 に追記する)

0233 §10 の 2.『時間伸縮(Elastique)を使うクリップは出荷構成では鳴らない(F2xH6 まで)』と、§7 の表『Elastique の箱は出ない見込み(読み)』のうち**前半は、この参照プロジェクトでは成り立たない**。

- 同じ手順(Chrome 156、出荷形の F2xH5 bundle = ref3 の `assets-A`、File>Open → Enter → Space → 30 秒再生)のスクリーンショット(`shots/h5-4-play-end.png`)で、Elastique の非 resample 伸縮モード(`channels.tsv` の mode -1 / -2 / 5)のチャンネル 3 本(vocals = ch7〔mode -2〕、`Other - Dot To Dot` = ch35〔mode -1〕、`schnozz atmos pad` = ch45〔mode 5〕)のミキサーのインサートのメーターが**動いている**(F2xH6 の `shots/4-play-end.png` と同じ 3 本)。
- ref3 の測定(`lift/out-ref3/tables/tempo-runs.tsv`)でも、Elastique の伸縮モード(mode -1)のチャンネル 16 `celtix_Albiorix_Snare_05.wav` の音が F2xH5 の捕捉の PCM に 26〜27 個のピークとして出る。
- 読み: Elastique の DLL が開けないとき、エンジンは自分の箱を 1 回出して、伸縮に別の経路(resample 相当)を使い、音は出す。**F2xH5 と F2xH6 で違うのは、その経路の音の質と CPU**(F2xH6 の再生 CPU は約 2 倍、§5.1)。どの経路か・音がどう違うかは**調べていない**(PCM の位置合わせに失敗、§5.3)。

したがって F2xH6 の価値は『鳴るようになる』でなく、**エンジンの本来の経路(Elastique)で動くこと**と **Problems 窓の『2 sample(s)』が消える(Vorbis が読める)こと**。

## 9. CreateIconIndirect の停止(**原因確定: ビルド方式の事故**。別文書 `0238b-createiconindirect-layout.md`)

PM の追加依頼(『診断 bundle の停止 3/3 とクリーン bundle の 0/N の差の再現・原因・契約どおりの実装』)の結果。**第 1 の作業(上)とは独立**で、この文書の結論はこの調査の結果に依存しない。要点だけ(訂正: この節の以前の版は『ランタイムの静的配置に依存したゲストの挙動』と書いたが、**誤り**):

- 診断 bundle t3 は Chrome で 9/9 停止、クリーン bundle c3 は 21/21 で停止なし。タイミングでなくバンドルごとに決定的だった。
- **原因**: 停止したバンドル(c4)の**直接 wasm(hybrid)に焼き込まれた `fle_teb`・`fle_mxcsr`・`fle_hyb_buf`・`fle_progress_count` の番地が、同じバンドルの C 部分が使う番地と違っていた**。診断ビルドの作業ディレクトリが `cp -al` で `hyb/`(link.map・n-*.wasm)を共有し、2 本同時にビルドしたため、兄弟の link.map を読んだ。同一の C 部分に正しい番地で直接 wasm を作り直した c5 は Chrome で 8/8 停止なし(c4 は 6/6 停止)。ランタイムの欠陥でも、ゲストが配置に依存するのでもない。
- 安全網: ハイブリッド段が、焼き込む番地が C 部分でも使われていること・link.map が C 部分を作った link のものであることを検査し、食い違えばビルドを失敗させる(`load2-guard.diff`)。
- Learn の CreateIconIndirect は失敗の条件を書いていないので、NULL を返す実装は**足していない**(Wine 10.0 は hbmColor NULL で奇数の高さなら成功して高さを /2 する)。
- 詳細・表(`tables/icon-stop.tsv`)・再現手順・安全網は 0238b。

## 10. 出典のない選択(`S-load2-*`、撤回条件つき。表: `tables/s-choices.tsv`)

| 番号 | 選択 | 出典・理由 | 代替 | 撤回する時 |
|---|---|---|---|---|
| S-load2-schema-data | The contract -> host rows are Wine 10.0's dlls/apisetschema/apisetschema.spec (511 rows, 3 of them without a host), written into runtime/fle_apiset_schema.inc by docs/investigations/load2/tools/mk-apiset.py | Learn gives no table of contracts and hosts; Windows' own schema (apisetschema.dll / the PEB's ApiSetMap) was not readable here; the per-function requirement tables of Learn are not machine-readable and partial | Windows 10's schema has more contracts than Wine's 511 (a contract that is missing here is treated as 'the schema lacks it', see S-load2-absent); alternative: read the real schema from a Windows installation | a real schema is read (replace the data file), or the engine meets a contract name that Windows maps and the table lacks |
| S-load2-subversion | The lookup key is the contract name without its last '-<n>' component, so l1-1-0 and l1-1-7 are one entry and l1-2-0 is another | Wine 10.0 ntdll/loader.c get_apiset_entry (hash and compare only the part before the last '-'; UNOFFICIAL). Learn only says a versioned name 'identifies one specific version' | exact match of the whole name (a name with a sub-version that the schema lacks would then fail) | a real Windows measurement of LoadLibrary('api-ms-win-core-string-l1-1-9') shows another rule |
| S-load2-absent | A contract name (api-/ext- prefix) that the table lacks is searched like any other bare name (the volume: nothing -> NULL + 126; a file -> named stop) | Wine 10.0 find_apiset_dll gives STATUS_APISET_NOT_PRESENT and the file search goes on (UNOFFICIAL); Learn 'Dynamic-Link Library Search Order': API sets are one step of the search, a name that is not one goes on; 'If the contract isn't mapped on the current device, a direct LoadLibrary fails' | a named stop (decision 0233's rule for the API set form); the cost is a crash for every name Wine's table lacks | the table is replaced by a complete schema, or a name that Windows maps is met |
| S-load2-nohost | A schema entry with no host (3 in Wine's table) fails: NULL + 126 | Learn 'Detect API set availability': a contract 'might be intentionally mapped to an empty module name'; 'If the contract isn't mapped ... a direct LoadLibrary fails'; Wine get_apiset_target -> STATUS_DLL_NOT_FOUND -> 126 (UNOFFICIAL). 126 is this runtime's code for a module that is not there (decision 0015) | 87, 2 | a Windows measurement of the error code |
| S-load2-host-handle | LoadLibrary(contract) returns the HOST module's handle and adds one reference on the host; the 5 contract-named virtual modules of the table (N=2, 27-30, decisions 0015 and 0081) keep their own handles because they are found by name first | Learn 'API set loader operation': the loader 'loads that host binary and binds'; Wine load_dll -> find_dll_file -> find_apiset_dll replaces the name by the host's path (UNOFFICIAL) | a handle per contract (what N=27-30 do); making the 5 table entries yield to the schema (their GetProcAddress answers are NULL/127 either way; not done because FLEngine's PE import api-ms-win-crt-string-l1-1-0.dll is a row key and the handle values of the later modules would shift) | a Windows measurement shows LoadLibrary(contract) is not the host's handle |
| S-load2-gmh-host | GetModuleHandle(contract) is the host's handle when the host is a loaded (virtual) module of this environment, else NULL + 126 | the GetModuleHandle pages name no case for a contract; Wine 10.0 LdrGetDllHandleEx -> find_dll_file(find_loaded) (UNOFFICIAL) | NULL + 126 always | a Windows measurement |
| S-load2-host-missing-stops | A contract whose host has no virtual module (ucrtbase.dll, wintypes.dll, sechost.dll, ...) stops by name (host named); no handle is invented | PM's instruction for this task; nobody has said whether such a host is loaded or what it exports | NULL + 126 | a virtual module for the host is added (kernelbase.dll and combase.dll are, decision 0238) |
| S-load2-kernelbase-combase | Two virtual modules appended: kernelbase.dll (N=40) and combase.dll (N=41). No response-table row has them in its dll column, so GetProcAddress on them is NULL + 127 for every name (the CRT then takes its fallbacks, as it already does for the 4 contract modules of decision 0081) | kernelbase.dll is loaded in every Windows 10 process (kernel32 sits on it) and the schema maps 123 of Wine's 511 contracts to it; combase.dll is loaded by ole32 (which FLEngine imports). Not an exact export model of those DLLs (they do export, e.g., CompareStringEx and RoInitialize) | kernelbase rows = kernel32's rows intersected with Wine's kernelbase.spec exports (more faithful, more CRT paths taken) | a measurement that the NULL answers cause a wrong path in oggio / Elastique, or the rows are added |
| S-load2-filetype-codes | GetFileType: open volume file -> 1 (DISK); invalid handle -> 0 + ERROR_INVALID_HANDLE (6) | Learn GetFileType (types, 'GetLastError will return the error code'); 6: System Error Codes / Wine 10.0 (UNOFFICIAL) as the page names no code | (none worth) | a Windows measurement |
| S-load2-flush-codes | FlushFileBuffers: a handle without write access -> 0 + ERROR_ACCESS_DENIED (5); invalid -> 0 + 6 | Learn: 'The file handle must have the GENERIC_WRITE access right'; the page names no code: 5 = Access is denied (System Error Codes; Wine UNOFFICIAL) | success for any open file (the opt-in wrap of decision 0206) | a Windows measurement, or the CRT calls it on a read-only handle and needs success |
| S-load2-sfpex | SetFilePointerEx = SetFilePointer on the same pointer: negative result -> 0 + 131; FILE_BEGIN with a distance above 2^63 and a method above 2 stop | Learn SetFilePointerEx / SetFilePointer (ERROR_NEGATIVE_SEEK) | FILE_BEGIN unsigned: fail with 87 | a Windows measurement |
| S-load2-findex | FindFirstFileExW answers the plain cases as FindFirstFileW (level 0/1, op 0/1, no filter, flags 0 or LARGE_FETCH); the rest stops | Learn FindFirstFileExW (advisory op, hint flag); decision 0206's wrap | all cases stop (before) | the CRT meets another combination |
| S-load2-promotion | The four answers are promoted from the opt-in harness wraps (harness/dec1, wrp1, vst) into the runtime; the wraps stay (they call __real_ for the cases they do not take, which is the runtime now) | PM's (b) | remove the wraps (not done: they are harness code of other lanes) | the harness lanes stop using them |

`S-load2-*` の追加(CreateIconIndirect の調査で): 後述 §9。
## 11. 再現表(作業ディレクトリ `lift/out-load2/` の相対。`tools/` はこの結果の `tools/`)

| 項目 | コマンド | 出力 |
|---|---|---|
| ワークツリー | `git archive HEAD` を `src-base/`、`+ ship4-h6.diff` を `src-h6base/`、そこに load2 の変更を足した物を `src/` | - |
| スキーマの表 | `python3 docs/investigations/load2/tools/mk-apiset.py docs/investigations/load2/tables/wine10-apisetschema.spec lift/runtime/fle_apiset_schema.inc` | `fle_apiset_schema.inc`(511 行) |
| F2xH6 の bundle(node 用・診断) | `tools/mk-trace-tree.sh` で `src-trace/`(診断のオーバーレイ)を作り `tools/iter.sh t2 $PWD/src-trace 0 fle_load fle_modules fle_env fle_vol fle_vfs` | `t2/engine/bundle/`(`NO_FS=0`) |
| F2xH6 の bundle(Chrome 用・クリーン) | `tools/iter.sh c3 $PWD/src 1 fle_load fle_modules fle_env fle_vol fle_vfs` | `c3/engine/bundle/`(`NO_FS=1`、wasm 146,033,464 B) |
| node の G3 | `BUNDLE=<bundle> SEED_TSV=tables/vfs-seed-nested.tsv tools/refproj.sh t2` | `runs/ref-t2/`(10 スナップ) |
| トレースの表 | `python3 tools/trace-table.py runs/ref-t4/stderr.txt` | `tables/trace-t4.tsv` |
| Chrome の手順 | `node tools/sdrive-ref3.mjs --browser chrome --url http://127.0.0.1:PORT/?debug=1 --proj-dir <49 files> --out DIR --scenarios tools/scn-ref3.json --viewport 2176x1224 --app app-run` | `1-problems.png` ほか |
| 起動 16 スナップ A/B | `tools/fsv.sh b1 <src-base>`(変更前)/ `tools/fsv.sh a1 <src> fle_load fle_modules fle_env fle_vol fle_vfs`(変更後)→ `tools/g3bundle.sh <bundle> b1` / `a1` → `cmp` | `tables/g3-lines-b1-84.txt` |
| 生成 C | `ciH6base/engine/cargo-target/release/fe-lift lift binaries/win64/FLEngine_x64.dll X --pdata` → `sha256sum X` | `logs/felift-default.sha256` |
| 全体テスト | `cd src && FLE_INPUT_ROOT=<main checkout> CARGO_TARGET_DIR=... bash lift/harness/test.sh full --jobs 3` | `logs/test-full-1.log`・`test-full-2.log` |
| a4 | `OUT=... bash lift/harness/a4-fruity.sh off` | `logs/a4-off.log` |
| mutation | `tools/mutation.sh` | `tables/mutation.tsv` |
| 差分 | `tools/mkdiff.sh load2.diff` | `load2.diff` |
## PM の判断と取り込み(2026-11-05)

取り込んだ基準(第 1 回): HEAD 6c60cc22 に `load2.diff`(22 ファイル、`-` 89 行は全部読んだ: `is_apiset_name` の旧実装と、停止を固定していたテストの期待値、4 関数が停止しなくなったことによる `allow.tsv` の noreturn 許可行の除去。offset・fuzz・reject なし)を `patch -p1`。PM が測った値(この機械、`lift/out-pmchk-l2/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、全体テスト **194 results / 2,381 passed / 0 failed / ignored 1**、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737。app には触れていない。

1. **API set の契約名の解決(Wine 10.0 の apisetschema.spec 由来のデータ、UNOFFICIAL)、仮想モジュール kernelbase.dll(N=40)・combase.dll(N=41)(export 行なし = GetProcAddress は NULL/127)、GetFileType・SetFilePointerEx・FlushFileBuffers・FindFirstFileExW の runtime の契約実装(決定 0206 の harness wrap の昇格)を採る。** すべて (b) 外部 API の契約の実装。推測の値(FlushFileBuffers の 5/6 など)は S-load2-* 13 件に撤回条件つき。
2. **F2xH6(oggio と Elastique を 7・8 番目のモジュールにした構成)で参照プロジェクトが停止なしに開いて再生でき、Chrome の Problems 窓から『2 sample(s)』の行が消える**(担当の実測。wasm 146,033,464 B、8 モジュールの S1 の線 154,885,888 B の下 8.85 MB)。F2xH6 のプリセット(ship4-h6.diff)は本取り込みに含まれない。決定 0239(ship6)が F2xH7 ベースで作る。
3. **決定 0233 §10 の訂正:** 『Elastique を使うクリップは出荷構成では鳴らない』は、この参照プロジェクトでは成り立たない(F2xH5 でも Elastique 系のチャンネルのメーターが動く)。
4. **決定 0238b(CreateIconIndirect の停止)は原因確定: ビルド方式の事故。ランタイムの静的配置への依存ではない。** 初回のコミットメッセージ(4977340a)と、この文書の以前の版の『the guest depends on the static layout of the runtime』は**誤り**で、PM が訂正する。停止したバンドルの直接 wasm(ハイブリッド)に焼き込まれた `fle_teb`・`fle_mxcsr`・`fle_hyb_buf`・`fle_progress_count` の番地が、同じバンドルの C 部分の番地と違っていた(診断ビルドの作業ディレクトリが `cp -al` で `hyb/` を共有し、2 本同時のビルドが兄弟の link.map を読んだ)。正しい番地で作り直した c5 は 8/8 停止なし(c4 は 6/6 停止)。CI の単独ビルドは影響を受けない。安全網 `load2-guard.diff`(焼き込む番地が C 部分でも使われていること、link.map の sha256 が一致することを検査し、食い違えばビルドを失敗させる。出荷 wasm のバイトは不変、`ci/test-hybrid.sh` に 13 テスト)を採る。
5. **`align.diff`(HeapAlloc・GlobalAlloc 等を 16 バイト整列に。契約 (b)、Emscripten の malloc は 2,000 ブロック中 986 が非整列)は、まだ取り込まない。** ゲストのヒープの番地が変わるので、起動 84 行・参照プロジェクトの行の同一性と F2xH7 の起動を実エンジンで測ってから取り込む(次の ship の作業に入れる)。停止の原因ではない。

追記(第 2 回の取り込み、HEAD 52034ca9 + `load2-guard.diff` + `rt1.diff`): 決定 0238b の最終版と本文 §9 の訂正を置き換えた。`load2-guard.diff` は `app/scripts/build-engine-bundle.sh`・`ci/plan.sh`・`ci/test-hybrid.sh` の 3 ファイル(出荷 wasm のバイトは不変)。PM が測った値(この機械、`lift/out-pmchk-r2/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、`ci/test-hybrid.sh` 全 ok、全体テスト **196 results / 2,386 passed / 0 failed / ignored 1**。
