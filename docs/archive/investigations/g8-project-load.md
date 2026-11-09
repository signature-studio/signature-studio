# G8（プロジェクトの読み込みと再生・内蔵の音源だけ）に何が要るか: 静的調査

作成: 2026-11-02、基準 HEAD **93881c0**（ブランチ main）。**調査と文書作成のみ。何も決めない**（推奨は書く。決めるのは PM）。**ビルドも FLEngine の実行もしていない**（capstone・pefile・標準ライブラリの python と、読み取りだけのインストーラ復号）。実機の Windows の画面・音は無い。補助物は `docs/investigations/g8-project-load/`（合計 約 150 KB）。

## 凡例

- **実測** = 今回のコマンドの出力（ファイルの中身・命令・表の突き合わせ・保存済みの実行ログの再読）。**読み** = 逆アセンブルや文書を読んだ解釈（実行して確かめていない）。**未確認** = 調べていない、または決められない。
- 「FLEngine の VA」は ImageBase 0x400000 を含む値（`f_XXXXXX` は関数の先頭 VA）。到達は **直接 call / jmp rel32 だけ**（lea を含める場合は「tier 2」と書く）。仮想メソッド・メッセージ配送・コールバックは辿れないので、**到達の集合は下限**、「辿れない」は「実行されない」ではない。
- **出典なしの選択**は `S-g8-…` と名前を付けて §10 に列挙した（値・理由・撤回条件）。
- 製品のバイト列（.flp、DLL、サンプル）は repo に入れていない。表に載せたのは名前・サイズ・SHA-256（先頭）・構造の要約。機械語の抜粋は §2.3 の 1 ブロック（**12 行**）のみ。

## 0. 要約表（先頭に置く 5 項目）

| # | 問い | 答え（根拠の区分） |
|---|---|---|
| 1 | **プロジェクトを開く経路（FLEngine に実在するもの）と推奨の入口** | **実在**（実測）: ① **起動時のコマンドライン**（`GetCommandLineW` ← `ParamStr`、`TFLCommandLine`／`CommandLineParser` の RTTI、起動処理 `f_11ef180` が `.FLP`／`.ZIP` を判定）。② **`WM_COPYDATA`**（受け側 = 主窓クラス `TFruityLoopsMainForm` の `f_11d1c80`。`dwData=10` + UTF-16 のパス → `TLoadFlpAction` をキューへ。送り側もエンジン自身に有る `f_117b930` = FL を 2 回目に起動したときの引き渡し）。③ **File > Open**（`FileOpenActionExecute` = `f_f1f010` → COM の `IFileOpenDialog`〔Delphi の `TFileOpenDialog`〕→ 文字列 `0x16bcca8` → `PostMessageW(main, 0x566)` → `f_11d14a0`）。④ **OLE のドロップ**（主窓は `TDropFileTarget`、`RegisterDragDrop` に行あり）。⑤ ウィザード／ブラウザのデモ・テンプレート・最近のファイル（`TDemoSongsModule`／`TChooseTemplateModule`／`TRecentFilesModule`）。**実在しない**: DDE（`Dde*` の import 0）、`WM_DROPFILES` は主窓では使われない（`OnDropFiles` を持つ公開ハンドラは `EnvPanelDropFiles` だけ）。**FLEngine の export は `CreateFruityInstance` と Delphi の 2 つだけで、読み込み用の入口は無い**。**FL64.exe はファイルを渡さない**（ホストの寄与は `GetCommandLineW` で見える同一プロセスのコマンドラインだけ）。**推奨の入口（意見）**: 起動時は ①（ランタイムの `GetCommandLineW` を可変にする）、実行中は ②（`fle_input` と同じ形のキューから `SendMessageW(WM_COPYDATA)` を主窓へ）。③ は G10、④ は後。 |
| 2 | **最小の G8 デモ** | **`Data\System\Render tests\Sampler.flp`**（81,831 B、FL 20.0.3、Sampler 1 チャンネル、パターン 1 本・ノート 7 個、ミキサーのプラグイン 0）+ サンプル **`Data\Patches\Packs\Shapes\Misc\SHP_Sawing Synth.wav`**（8,336 B、**RIFF PCM 16 ビット・モノ・44.1 kHz**）。**プラグイン DLL 0、復号 DLL 0**（実測: 165 本の解析と先頭バイトの照合。「FLEngine が PCM の WAV を自前で読む」は読み: `f_670fd0` の閉包に acm／mmio／LoadLibrary が無い）。無音で開くだけなら `Empty.flp`（エンジンが起動時に自分で開く）。内蔵だけで音が出る次の候補: `Templates\Other\Blocks\Blocks.flp`（要 oggio）、プラグイン 1 個の `Render tests\3OSC.flp`（要 `3x Osc_x64.dll`）、サンプルもエフェクトも無い `Product demos\Synth_dfr - Overcome - Sytrus.flp`（要 `Sytrus_x64.dll` 6.27 MB）。 |
| 3 | **G8 に要る未 lift の DLL と状況** | 最小デモには **0 個**。サンプル全般では **`oggio_x64.dll`（1.87 MB）が事実上必須**（インストーラのデモ／テンプレート 165 本が参照するサンプル 2,008 ファイルのうち Vorbis-in-WAV 1,324 + OggS 439 = **1,763（87.8%）**。PCM／float は 69（3.4%））、`wavpackdll_x64.dll`（272 KB）は 167（8.3%）、FLAC 7、MP3 2。**`ildsp_x64.dll` はサンプルの復号ではない**（export 10 個はすべて DSP 効果の `*_CreateInstance`。goal-ladder §3.10 G8 の行の「ildsp」は復号の要件としては訂正。ただし FLP 解析の閉包に `LoadLibraryW("ildsp")` のローダー `f_66ea10` が入る〔NULL を許す分岐。実際に呼ばれるかは未確認〕）。いずれも **x64・`.pdata` 有り**。wavpack は素直（SSE2 61 命令、AVX 0）、oggio は IPP の静的リンク（SSE2 3.4 万・AVX 1,954・AVX2 72・AVX-512 130・x87 14、jump-table 様 114）。**`fe-lift` は未実行**なので「通る」とは言えない。実在モジュールの枠 `RM_MAX = 8` に対し現在 5 個使用。 |
| 4 | **供給物の増分** | 最小デモ: **ファイル 2 個・約 90 KB**（.flp 81,831 B + wav 8,336 B。現在の種 218 ファイル 5.64 MB、`Empty.flp` は含まれる）。`Basic.flp` 経由なら +5 個・約 232 KB。同梱のすべての .flp は **165 本・87.7 MB**（デモ 46 本 49.1 MB、テンプレート 35 本 30.8 MB、Render tests 82 本 7.7 MB）。それらが参照するサンプルは **2,008 ファイル・約 160 MB**（LZMA 後の大きさで近似）、デモ 46 本分は 1,528 ファイル・約 140 MB。165 本の全部が要るプラグインの合併は **111 種・DLL 139 個（補助 DLL を含む）・約 524 MB**（無圧縮の PE。デモ曲 20 本だけなら 88 種・110 個・約 408 MB）。`Data` 全体は圧縮後 483.6 MB（うち `Data\Patches` 455.1 MB）。V8 の単一 wasm 上限・MEMFS の二重保持の論点は §6。 |
| 5 | **段階案 G8a〜d の受け入れ条件と壁の見込み** | §7。要点: **G8a** = Empty 以外の内蔵だけの .flp（`Basic.flp`）を開く、**G8b** = サンプル 1 個＋ノートの `Sampler.flp` を開く、**G8c** = 再生して ring に**非ゼロ**（ノート位置 0／0.25／…／1.5 s に対応する塊。**実時間の連続性は条件に入れない**: 決定 0139 は実エンジンで 98.1% アンダーラン）、**G8d** = oggio 1 個で通る `Blocks.flp`（Vorbis-in-WAV 3 + PCM 1）→ その先に G9 のプラグイン。**壁の数は見えない**（過去 16 壁連続）。 |

## 1. 手順・出典・実行環境

- 作業木は読み取り専用（新規の docs ファイルのみ）。`PYTHONDONTWRITEBYTECODE=1` と `sys.dont_write_bytecode`。FLEngine は `binaries/win64/FLEngine_x64.dll`（SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e…`、22,227,016 B。`lift/out-idle1/inputs/install/` のものと同一）、FL64.exe は `lift/out-fruity6/inputs/FL64.exe`（597,064 B、SHA-256 `4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65`）。
- 索引: `docs/investigations/goal-ladder/fle_index.py`（既存。7 秒、RSS 0.69 GB）の pickle を `/tmp/opencode/g8in/fle.pkl` に作り、本調査の `fle_query.py`／`fle_closure_table.py` で問い合わせた。
- インストーラ `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（SHA-256 `04393e79…5fc02`）は `lift/harness/seeds/gen_install_tree.py` の `walk`（決定 0129。'FL Studio (required)' の節、**27,402 ファイル**）と `tools/extract_nsis2.py` で**読み取りのみ**。`/tmp/opencode/g8in/` に抽出した .flp 165 本（87,694,538 B）・サンプル 12 個・DLL 10 個（合計 113 MB）。**終了時に全削除**（§11）。
- 抽出の再現: `python3 docs/investigations/g8-project-load/g8_extract.py INSTALLER OUTDIR MANIFEST.tsv '\.flp$'`（以下同様）。表の再生成: `flp_tables.py` → `sample_sniff.py` → `g8_trim_tables.py`（README は §12）。
- ネットワーク（FLP 形式の公開資料のみ）: ① https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html（検索結果の抜粋。「FLP は 2 つの chunk、'FLdt' の後に events。ID 0–63 は 1 バイト、64–127 は 2、128–191 は 4、192–255 は長さ前置」）。② https://github.com/Holzchopf/flp-file（README: 「.fst の状態ファイルも同じ形式」「FLhd／FLdt」「値の型は ID の範囲で 1/2/4 バイト、192 以上は binary か utf-16le」）。③ https://raw.githubusercontent.com/demberto/PyFLP/master/pyflp/{_events,channel,plugin,project,mixer,pattern,arrangement}.py（2026-11-02 取得。**GPL-3.0 の第三者のリバースエンジニアリング**であって Image-Line の資料ではない。イベント ID の名前と構造体の欄の出典）。④ https://forum.image-line.com/viewtopic.php?t=53021（Image-Line のフォーラム「Documentation for .flp file format?」、14 件・ロックされている。ログインしていないため部分しか見えず、見えた断片は「Alphons 2019-03-16: It's been 9 years. But yes, it's very unlikely…」「reflex（Site Admin）2019-03-18: It's not too complex, it's just very low priori…」のみ）→ **Image-Line の公式の FLP 仕様書は見つからなかった**。⑤ `Holzchopf/flp-file` の `src/flp-event.ts` の raw URL は 404。

## 2. プロジェクトを開く経路（項目 1）

### 2.1 ホスト（FL64.exe）と FLEngine の役割分担

実測（`docs/investigations/host-engine-boundary.md` §5 の再読と FL64.exe の import）: FL64.exe は `Mfplat` の確認 → 自分のホストオブジェクト（0x38 B: vtbl、version=1、flags=0、名前 "FL Studio"、基準ディレクトリ）→ エンジンの場所決め → Authenticode（署名者 "Image Line NV"）→ `LoadLibraryW` → **`CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)`** → 戻りが 0／2 なら `instance->vtable[0]`。**コマンドラインは「空のときだけエンジン選択を出す」判定に使うだけ**で、ファイル名を FLEngine へ渡す処理は FL64.exe に無い（import: `CreateFileW`／`FindFirstFileExW`／`GetCommandLineA/W`／`ShellExecuteA` など。`RegisterWindowMessage`・`DragAcceptFiles`・`FindWindow` は無い）。FLEngine は同じプロセスなので **`GetCommandLineW` でホストのコマンドラインを自分で読む**。我々のハーネスは FL64.exe を走らせず `CreateFruityInstance` を直接呼ぶので、**ホストの寄与は「`GetCommandLineW` の値」と「ホストオブジェクトの `flags`」の 2 つだけ**（現在、前者は引用符付きの仮想 DLL パスで引数無し: `lift/runtime/fle_env.c` 265 行）。

**エクスポート**（実測）: `CreateFruityInstance`、`__dbk_fcall_wrapper`、`dbkFCallWrapperAddr`。**読み込みに関わる export は無い。**

### 2.2 経路の表

| 経路 | FLEngine に実在？ | 根拠（実測/読み） | ランタイムの状況（HEAD の `imports_table.rs` 601 行） |
|---|---|---|---|
| 起動時のコマンドライン | **実在** | `GetCommandLineW` の直接の呼び元 `f_409fe0`（ParamStr）・`f_40a060`（ParamCount）・`f_41ce10`。`f_11b32a0`（BasicInitialization。`CreateFruityInstance` が `TApplication` の窓の前に呼ぶ）→ `f_65dd10` → `f_409fe0`。RTTI `TFLCommandLine`／`TCommandLineParser`／`TCommandLineSwitch`（0x10e18ca〜）、`f_10e3e50` が ParamStr を読む。起動処理 `f_11ef180`（"ProcessFirstTimeStuff"、"CrashLoadingStartupProject"。`f_1174fc0` が `[0x15c2641]` の「起動待ち」フラグを見て `f_11efbf0` → `f_11ef180` を呼ぶ）は、コマンドラインの解析結果のグローバル `[0x15e1400]` が指すオブジェクトの `+8`（モード。2 = ファイル引数あり）と `+0x3c`（`TStrings`。`Count` は vtable `+0x28`、`Strings[0]` は `+0x18`）から先頭のファイル名を取り、`flstudio:` で始まれば URL（モード 4、`f_117f780`）、そうでなければ拡張子 `.FLP`／`.ZIP` で判定して存在（`f_819110`）すれば開く（読み。実行はしていない）。スイッチの名前と意味は**未解析** | 行あり（固定値。引数無し） |
| **WM_COPYDATA**（外部から「このファイルを開け」） | **実在**（受け・送りとも） | 受け: 主窓のメッセージ表（0x117409f、42 項。主窓の公開メソッド表〔`AutoSaveTimerTimer` 0x1171fae など〕と同じ領域、読み）の 0x4a → **`f_11d1c80`**。`COPYDATASTRUCT.dwData` で分岐（dwData 1／10／11 の枝は先頭で `[[0x15e4790]+8]->vtable[+0x80]`〔11 は `+0x88`〕の真偽を見る**前提条件**がある。意味は未読）: 1 = 文字列 "Received %s"、**10 = 通常のパス**（クラス参照セル `0x1180a08` が `TLoadFlpAction` の TClass〔自己ポインタ 0x1180ad0。実測: TFLProject・TSavedProjectInfo でも VMT 先頭 + 0x70 の規則〕を指す。0x18 B のオブジェクトを作り +8 に UTF-16 のパス、`f_92d0f0` でクリティカルセクション付きのキューへ）、12 = `flstudio:` の URL（`f_117f780`）、11 = 別。送り: **`f_117b930`** = ParamStr(1) → `FindWindowW(L"TFruityLoopsMainForm")` → ShowWindow／SetForegroundWindow → `SendMessageW(hwnd, WM_COPYDATA, …)`（dwData = 先頭が "flstudio:" なら 0xC、そうでなければ 0xA、cbData = (長さ+1)×2）。**FL を 2 回目に起動したときの引き渡し**と読める。キューのオブジェクトの `Execute`（`f_1180b50`、実測）は `[[0x15e4790]+0x10]->vtable[+0x40](this+8 のパス, フラグ)` を呼ぶ。これは 0x566 の受け側と**同じ呼び出し**で、`TFLProject` の仮想メソッド **`f_ed5d40`**（VMT の 22 番）→ **`f_11d58c0` を直接 call**（0xed5e34。"Backup" の文字列処理つき）。**キュー `f_92d0f0` を誰がいつ消費するかは未追跡** | `FindWindowW` 行あり（窓が無ければ NULL）。`SendMessageW`／`PostMessageW` 行あり。主窓への COPYDATA を作る入口は**無い** |
| File > Open | **実在** | `FileOpenActionExecute`＝`f_f1f010` → `f_f1f020`: 状態 `[[0x15e4790]+0x10]->vtable[+0x70]` が 2 なら何もしない → ダイアログ `[[0x15e0ff8]+0x998]`（`InitialDir` を `f_879270`）→ `vtable[+0xa8]`（Execute）→ 真なら `f_8790c0` でファイル名 → 文字列 `0x16bcca8`（ポインタ・セル `0x15e2a98` 経由）へ代入 → **`PostMessageW(main, 0x566, 0, 0)`**。0x566 の受け側 `f_11d14a0`（同じ表の 29 項）: `lParam>0` なら `lParam-1` で自分に再投稿、0 のとき拡張子が `.FLP`／`.ZIP` なら `f_11cf9b0` → `[[0x15e4790]+0x10]->vtable[+0x40]`（= `f_ed5d40` → `f_11d58c0`）→ `f_11cfa00`。ダイアログの実体は `TFavOpenDialog`／`TFavFileOpenDialog`／`TFileOpenDialog`（RTTI 0x7a1c52／0x7a2a0a／0x873337）、**`GetOpenFileNameW` は `lea` でしか取られない（直接の呼び元 0）**＝旧式の `TOpenDialog` 用。`IID_IFileOpenDialog` は 0x87c358／0x87c488、`CLSID_FileOpenDialog` は 0x13d1c08 にあり、`f_87c1d0`／`f_87c4a0` が `CoCreateInstance`。**Windows Vista 以降で COM の `IFileOpenDialog` を使うかの分岐は読んでいない（未確認）** | `GetOpenFileNameW` 行なし。`CoCreateInstance` 行あり（未登録の CLSID は `REGDB_E_CLASSNOTREG`、S-com-registry-empty）＝**ダイアログの作成は失敗する**。`SHCreateItemFromParsingName`（遅延 import）行なし |
| OLE のドロップ（主窓） | **実在** | 主窓の公開ハンドラ `DropFileTargetEnter`／`DragOver`(`f_124b6d0`)／`GetDropEffect`(`f_124be30`)／`Drop`(**`f_124b8d0`**。`.FST`／`.FSC` などの拡張子を判定)。`RegisterDragDrop` の呼び元 `f_700ac0`／`f_700b60`／`f_7011b0`／`f_9f3420`／`f_a0f130`、`IID_IDropTarget` 0x9f3578、`CLSID_DragDropHelper` 0x14d3d2c（`f_9f4a70`／`f_9f79c0`） | `RegisterDragDrop`／`RevokeDragDrop` **行あり**（決定 0126。`fle_ole.c` が (hwnd, IDropTarget*) の表を持つ）。**DragEnter／Drop を呼ぶ側と `IDataObject` は無い**。`OleGetClipboard`／`DoDragDrop` 行なし |
| `WM_DROPFILES`／`DragAcceptFiles` | 基底フォームに**実在するが主窓は使わない** | `f_827560`（CreateWnd）・`f_8275a0`（`OnDropFiles` の setter → `DragAcceptFiles`）・`f_827600`（`WMDropFiles` = `DragQueryFileW`×2、`DragQueryPoint`、`FOnDropFiles(Sender, files, point)`、`DragFinish`）。メッセージ表 0x81fbbe の 1 項（0x233）。`[this+0x500]` へメソッドを書くコードは**無い**（`fle_scan_store.py` の結果 0 件。DFM のストリーミングで入る）。公開ハンドラ名に DropFiles を持つのは `EnvPanelDropFiles`（`f_f62ef0`）だけ | `DragAcceptFiles`／`DragQueryFileW`／`DragFinish`／`DragQueryPoint` **行なし**（4 件）。基底の `OnDropFiles` が未設定なら呼ばれない（読み） |
| DDE | **無い** | FLEngine・FL64.exe とも `Dde*` の import 0（実測） | — |
| ウィザード／ブラウザの「デモ」「テンプレート」「最近」 | **実在** | RTTI `TWelcomeWizard`（0x113b795）・`TRecentFilesModule`／`TMRUItemData`・`TDemoSongsModule`・`TChooseTemplateModule`・`TOpenDemoProjectsTask`・`TLoadFlpAction`（0x1180afa）。起動時に `Data\Demo projects\*`、`Documents\Image-Line\FL Studio\Projects\*`（`Projects\Templates`／`Backup`）の `FindFirstFileW` が出る（`docs/investigations/idle1/logs/files-touched.tsv`、実測） | 必要な API は同じ（`FindFirstFileW`…は行あり）。現在のアプリの画面に出ているウィザード（"Start a new project"）は同じ `gui.WelcomeWizard` 系 |
| 起動時の既定プロジェクト（`Data\System\Empty.flp`） | **実在し、現在のランタイムでも通っている**（実測） | 決定 0129 の壁 16 の実際のシャドウスタック: `Run`→…→`DispatchMessageW`→`f_52a570`→`f_976b30`→`f_92d710`→`f_1174fc0`→`f_11efbf0`→`f_11ef180`→`f_ed5c40`→**`f_11db420`**→**`f_11d58c0`**（`docs/investigations/a4f0129/logs/a4/x0129a/stop-excerpt.txt` #8〜#22）。`f_11db420` は名前が無い／存在しないとき `Data\System\Empty.flp` に置換し、無ければ "Error\|Could not find default empty project."（旧 idle の TMsgForm の原因。決定 0129 発見 B）。決定 0139 の native gdb でも同じ鎖で `f_11d58c0` が 112 s に `f_1253ae0` を呼んでいる（`audio1/logs/s0-gdb/gdbaudio.txt` 551 行） | 行は揃っている（`Empty.flp` は種に有る）。**`f_f54760`／`f_f51fd0` に実際に入ったかを示す記録は読めた範囲に無い（未確認）**。壁 16 の停止は `f_11d58c0` の**前半**（`f_11fba20` = 0x11d60f8、"Opening file..." の前）なので、その時点では `f_f54760` に未到達。SEH 付きの現在の idle の画面（決定 0129／0141）のテンポ "140.000" は `Empty.flp` のテンポ（140000）と一致するが、FL の既定値と区別できないので根拠にしない |

### 2.3 呼び出しの鎖（抜粋。全体は `open-path-chains.txt`）

- 開く本体 `f_11d58c0`（5,442 B、直接 call 93）は拡張子で分岐する: `.zip` なら "Unzipping: %s"（`f_9c7570`／`f_9c7f20`…）、`.flp` なら "Opening file..." の後 **`call f_f54760`（0x11d63ec）**、その後 `f_11d9340`・`f_11f16b0`（"Project loaded..."）。MIDI の取り込み設定（"main:MIDIImportAuto"…）、"load mixer state"、"load effect plugin"、"insert channel"、`ScoreTrack.fst`、`FLStudioMobileTrack.fst` の文字列がある（実測）。
- `f_f54760`（258 B）→ **`f_f51fd0`**（8,672 B。文字列 "FLhd"／"FLdt"／"Project format is not compatible with target, aborting"／"This project contains invalid data and may cause the program to crash."、`SHFolder.dll`+`SHGetFolderPathW`、`GetComputerNameExW`、`HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`）→ **`f_f46490`**（43,382 B、直接の呼び出し先 363。イベントごとの処理と読む）。
- **訂正（goal-ladder §3.2 の読みとの食い違い）**: goal-ladder は「`f_1375950` が Flags のビット 0 で `f_86a660`（Run）か `f_11efbf0`（別のループ）に分かれる」と読んだ。**実スタックでは Flags=0 の FRUITY で `f_11efbf0` が Run の下の WndProc（`f_1174fc0`）から呼ばれる**。`f_11efbf0` は主窓のメッセージ処理の一部で、「別のループ」ではない。
- 機械語の抜粋（12 行）。`f_11db420` の既定プロジェクトの置換と、`f_f1f020` の「開く」の後半（要点のみ）:

```
11db4e9: call f_819110          ; FileExists(name)            -> true: call f_11d58c0(prog, name, 1)
11db50a: lea  rcx,[rbp+0x88]    ; else: name := <data root> + u"Data\System\Empty.flp"   (f_414c70)
11db52b: call f_819110 ; test al,al ; je -> u"Error|Could not find default empty project."
11db545: call f_11d58c0         ; the project loader, second call site (the return address 0x11db54a of the stack above)
f1f061:  call [rax+0x70] ; cmp eax,2 ; je skip                 ; busy/state check on [[0x15e4790]+0x10]
f1f0f3:  call [rax+0xa8] ; test al,al                          ; dialog->Execute
f1f110:  mov rcx,[->15e2a98] ; mov rdx,[rbp+0x28] ; call f_4133f0   ; string assign into 0x16bcca8
f1f12b:  mov edx,0x566 ; call PostMessageW(main hwnd, 0x566, 0, 0)
11d14de: mov edx,0x566 ; r9 = lParam-1 ; call PostMessageW      ; the 0x566 handler re-posts itself while lParam>0
11d1517: call f_414e70 (".FLP") ; ... (".ZIP") ; call f_11cf9b0   ; extension test, then the open
11ba1a:  mov edx,0x4a ; call SendMessageW                       ; f_117b930, the in-engine sender of WM_COPYDATA
```

### 2.4 まとめと「偽りの有無」の観点

実在する 3 つの外部入口は、**どれも「ファイル名をゲストの文字列／メッセージで渡す」形で、ファイルの中身はゲストが VFS の `CreateFileW`／`CreateFileMappingW` で読む**。したがって「仮想ボリュームに置く」ことが共通の前提になる（§8）。

## 3. 読み込みで要る API の範囲（項目 2）

### 3.1 閉包の大きさ（実測。`fle_closure_table.py`。HEAD の `imports_table.rs` 601 行に対する突き合わせ）

| 根 | tier | 関数 | import 名 | 行あり | 行なし |
|---|---|---:|---:|---:|---:|
| `f_f51fd0`（FLP の解析） | 1 | 5,217 | 287 | 261 | **26** |
| 同 | 2 | 6,548 | 322 | 275 | 47 |
| `f_11d58c0`（OpenProject） | 1 | 5,862 | 332 | 291 | **41** |
| 同 | 2 | 7,230 | 371 | 304 | 67 |
| `f_11d14a0`（0x566 の受け） | 1 | 288 | 49 | 48 | 1 |
| `f_f1f020`（File>Open の処理） | 1 | 303 | 48 | 47 | 1 |
| `f_124b8d0`（主窓のドロップ） | 1 | 503 | 62 | 61 | 1 |

`f_11d58c0`（tier 1）の import 332 名を API 名の正規表現で粗く分けると（`fle_closure_table.py` の `GROUPS`。**分類は読み**）、行あり／行なし: ファイル I/O 38／6、COM・OLE 17／0、ネット（wininet ほか）4／1、シェル・ダイアログ 1／3、クリップボード 0／4、スレッド・同期 23／0、窓・GDI・入力 136／18、その他（RTL・ロケール・レジストリ・時刻など）72／9。メモリマップ（`CreateFileMappingW`／`MapViewOfFile`）は「ファイル I/O」に含まれ**行あり**。COM は `CoCreateInstance`／`CoInitialize` 系などすべて行あり（ただし CLSID は未登録で失敗する）。

FLEngine 全体の import は 795 名のうち **519 に行あり（goal-ladder 作成時の 371 から増えた）**、276 が行なし。閉包が 5 千関数を超えるのは、プロジェクトを開くとプラグイン・UI・ミキサーの構築まで 1 回の呼び出しで届く（goal-ladder §3.5 の読みと一致）ため。**この閉包の「行なし」は上限であり、実際に踏むものは実行しないと分からない。**

### 3.2 行なしの API とその役割（`open-path-none-apis.tsv` が全 67 名。読みを含む。実行はしていない）

| 行なし | 閉包 | 役割（読み） | .flp の素直な経路で踏むか |
|---|---|---|---|
| `GetDiskFreeSpaceW` | `f_11d58c0`（tier 1） | `.zip` の枝（"Unzipping: %s" → `f_9c7f20` → `f_9af830` → `f_9aeb00` → `f_9ab1b0` → `f_99b490` → `f_99b880`）の空き容量の確認 | **踏まない見込み**（zip の枝） |
| `GetFileInformationByHandle`／`CompareFileTime`／`SHFileOperationW` | 同（zip の枝） | 展開物の識別・時刻比較・削除／コピー | 踏まない見込み（zip）。`wavpackdll`／`oggio` も CRT の fstat 用に import するが使用は未確認 |
| `SetEndOfFile` | tier 2 のみ（`f_99a780`、RTL の `TFileStream` のサイズ変更 `f_40ca70`、`f_513150`） | 書き込み系 | 読み込みのみなら踏まない |
| `OpenFileMappingW` | 閉包に**入らない**（呼び元は `f_c83820` のみ。3 つの呼び元） | 他プロセスとの共有メモリ（ブリッジ等）と読む | 踏まない |
| `SHGetKnownFolderPath`（遅延 import） | 閉包に入らない（呼び元 `f_5a0c90` のみ） | 既知フォルダーの取得 | 踏まない（`SHGetFolderPathW` に行がある） |
| **`SHAddToRecentDocs`** | `f_11d58c0`（`f_11d9340` → `f_11e6690`） | 開いたファイルを OS の最近の項目へ | **条件付き**: `f_11d9340` の該当ブロックは `[0x15c2640]`（byte）が真のときだけ。`f_11db420` は読み込みの直前にこれを 0 にして後で 1 に戻す（`11db4c7`／`11db703`）。素直な「開く」では**踏まない読み**。踏めば名前付き停止（void の no-op 行で足りる見込み） |
| `GetFinalPathNameByHandleW`（遅延） | `f_11d58c0` と `f_f51fd0` の両方（`f_438ba0`←`f_43b830`←`f_11fecf0`←…） | 開いたハンドルの最終パス（`CreateFileMappingW` を併用する旧式の代替実装が同じ関数にある） | 未確認。遅延ロードの欠落時の分岐は読んでいない |
| `ShellExecuteW`（17 の呼び元）／`InternetReadFile` | `f_f46490` の下（`f_c31890`） | イベントのハンドラから URL／ファイルを開く／ネット読み | 条件付き。ネットは起動時に既に `InternetOpenUrlW` 経路を踏んでいる（画面の "Failed downloading news. Error 4." は決定 0141） |
| `MessageBoxW`／`SendMessageA`／`DispatchMessageA`／`IsDialogMessageA/W` ほか user32 の約 20 名 | 窓・ダイアログ系 | エラー・確認の表示（`f_86a920`）。エンジン自前の `TMsgForm` は既に通る（決定 0129） | ダイアログが出たときのみ |
| `EnumClipboardFormats`／`GetClipboardData`／`SetClipboardData`／`IsClipboardFormatAvailable` | tier 1（`f_f46490` 近傍） | 貼り付け | 通常の読み込みでは踏まない見込み |
| `msacm32` の `acm*`（12 名） | tier 2 のみ | Windows の ACM（圧縮 WAV）。**PCM の `.wav` では使わない読み**（`f_670fd0` の閉包に無い） | Vorbis-in-WAV の経路が ACM を使うか oggio かは未確認 |

### 3.3 「行あり」でも名前付き停止になるもの（ソースの読み。実行はしていない）

`CreateFileW`: `lpSecurityAttributes`／`hTemplateFile` が非 NULL、`dwShareMode` の範囲外、`GENERIC_EXECUTE` 等、未知の `dwFlagsAndAttributes` は停止。`ReadFile`: `lpOverlapped` 非 NULL は停止。`CreateFileMappingW`: 名前付き・`PAGE_READWRITE`（仮想ファイル）・`size>ファイル長`・SEC_* は停止。`MapViewOfFile`: オフセットが割り当て粒度でない／2 つ目の同一オフセットのビューは停止。**エンジンの WAV 読み（`f_670fd0`）の閉包は `MapViewOfFile`／`UnmapViewOfFile` を使い、`CreateFileMappingW` の呼び元には `f_6749f0`（WAV の領域）・`f_8e1500`・`f_a64da0`・`f_63b680` 等がある**（実測）。wasm では「マッピング = アリーナの範囲」で実装済み（`fle_vmap.c`。native は `memfd_create`）。**サンプルの読み取り専用マップは通る設計**だが、実走は未確認。`CoCreateInstance` 行は未登録の CLSID で成功の形の失敗を返す。

### 3.4 ファイルダイアログは G8 に必須か

**必須ではない**（読み）。ダイアログの CLSID は `CLSID_FileOpenDialog` = DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7（実測: 0x13d1c08 に同じバイト列）、`IID_IFileOpenDialog` = D57C7288-D4AD-4768-BE02-9D969532D960（0x87c358／0x87c488）。`Show` はシェル内のモーダルループで、ゲストから見れば「戻らない呼び出し」（ブラウザの選択を待てばよい）。Delphi 側の前後処理（他の窓の無効化など）は**未読**。起動時のコマンドラインと `WM_COPYDATA` はダイアログを経由しない（§2.2）。File > Open を**ブラウザの操作として再現する**には、COM の `IFileOpenDialog`（`CoCreateInstance` → `SetOptions`／`SetFileTypes`／`SetFolder`／`Show`〔モーダル。JSPI 等でブラウザのファイル選択を待つ〕→ `GetResult` → `IShellItem::GetDisplayName`）のモデルが要る。**使うメソッドの数え上げと `IFileDialogCustomize`（`TFavFileOpenDialog`）の使い方は未実施**。これは G10 の項目と読む。

## 4. FLP の形式と同梱プロジェクト（項目 3）

### 4.1 形式（出典 §1。実ファイルで検証）

`FLhd`（長さ 6: u16 format、u16 channels、u16 ppq）→ `FLdt`（u32 長さ）→ イベント列。ID < 64: 1 バイト、< 128: 2 バイト、< 192: 4 バイト、≥ 192: 可変長（7 ビットずつ・下位から）+ データ。**実測: 165 本すべてで、イベント列の長さが `FLdt` の長さとちょうど一致（読み残し 0、超過 0）**。文字列は UTF-16LE が多く、`FLVersion`（199）などは ASCII。主なイベント ID（PyFLP による名前。**Image-Line の資料ではない**）: 64 NewChan（チャンネル番号）、21 チャンネル種別（0 Sampler、2 Native、3 Layer、4 Instrument／Audio clip、5 Automation）、201 プラグインの内部名、203 ユーザー名、196 サンプルのパス、212 プラグイン枠、213 プラグインの状態、65 NewPat、224 ノート（24 B 刻み）、99 NewArrangement、233 プレイリスト（32 B 刻み、FL 21 以降は 60 B 刻み）、236 ミキサーの挿入（InsertFlags）、98 スロット番号（古いファイルには無い）、156 テンポ。**不明の ID が多く（`flp_events.py` の名前表は 80 余り）、`PluginWrapper`／`MixerParams`／`TrackData` の中身は読んでいない。**

### 4.2 `Empty.flp`（46,361 B、SHA-256 `ae67bc83…685d4`。`empty-flp-structure.txt`）

FL 24.2.99.4720（ビルド 4720）、format 0、PPQ 96、FLdt 46,339 B・イベント 1,413 個。テンポ **140.000 BPM**（イベント 156 = 140000。現在のアプリの画面の "140.000" と一致）。**チャンネル 1 個**（iid 0、種別 0 = Sampler、名前 "Sampler"、サンプル無し、プラグイン状態 0 B、ラッパー枠 52 B）、**パターン 0 個**（現在のパターン番号 1 だけ）、アレンジメント 1 個（"Arrangement"、プレイリスト 0 B）、**ミキサーの挿入 18 個・スロット 180（= 10 × 18）・プラグイン入りのスロット 0**、プレイリストのトラック 500 個（`TrackData` 36,000 B = ファイルの 78%）。**必要なプラグイン DLL: 無し。必要なサンプル: 無し。** 同一内容（SHA-256 一致）が `Data\System\Empty.flp`・`Templates\Empty\Empty.flp`・`Templates\Minimal\Empty\Empty.flp`・`Templates\Utility\Launch loop starter\…`（計 4）。165 本のうち一意は 161。

### 4.3 165 本の概要（`demos.tsv` が全行。`flp_tables.py`）

- 内訳: Demo projects 46（デモ曲 20、製品デモ 10、Performance 6、Visual 9、NewStuff 1。`NewStuff.flp` は `Ookay - Thief.flp` と同一）、Templates 35、`Data\System\Render tests` 82、`Data\System` 直下 2（`Empty.flp`、`FL Mobile\Song.flp`）。FL のバージョンは 10.9〜26.1（`flp_tables.py` の `FLVersion` 列）と幅広い。`Render tests` は 11.1.1〜24.2（53 本が FL 11.x、9 本が 12.x、9 本が 20.x、7 本が 21.x、4 本が 24.x）。
- **「Render tests」82 本**は、各プラグインに 1 本（生成器の試験はその生成器、エフェクトの試験は `3x Osc` + そのエフェクト）。IL 自身の回帰試験用と読める（**実機の書き出し WAV は無い**）。`render-tests.tsv`。
- **エンジン内蔵だけで開けるプロジェクト（プラグイン DLL もエフェクトも VST も無い）は 12 本**: `Empty`（SHA-256 一致の 4 本: `Data\System\Empty.flp`、`Templates\Empty\Empty.flp`、`Templates\Minimal\Empty\Empty.flp`、`Templates\Utility\Launch loop starter\…`）、`FL Mobile\Song.flp`、`Render tests\Sampler.flp`、`Basic.flp`、`Empty with 4 sends`、`All MIDI CC`、`Blocks.flp`、`Record Audio.flp`、`Remix a song`（独立の照合: 12 本とも 201〔プラグイン内部名〕が空、213〔プラグイン状態〕0 B）。**再生ボタンだけで音が出るのは `Render tests\Sampler.flp`（ノート 7）と `Blocks.flp`（プレイリスト 260 = パターン 208 + サンプルのクリップ 52、サンプル 4）の 2 本**。`Basic.flp`／`Record Audio.flp` はサンプル 4 を持つがパターンもプレイリストも空（チャンネルを手で叩けば鳴る）。`Empty with 4 sends` はセンドの経路だけでプラグインは空。（`Minimal\Delay and reverb` は `Fruity Delay Bank` と `Fruity Reeverb 2` を持つので対象外。）
- 同梱のデモ・テンプレートに**外部 VST（Fruity Wrapper の実体が VST）は 0**（`external_vst` 列）。古い版の「Fruity Wrapper」はネイティブのプラグイン DLL 名（例 `Fruity Fast LP.dll`）を包んでいる。

### 4.4 代表 18 本の使用プラグイン・サンプル（`demos.tsv`、`samples-candidates.tsv`）

略称: RT = `Data\System\Render tests`、PD = `Product demos`、DS = `Demo songs`。サンプルの形式は先頭バイトの照合（§4.6）。「圧縮後」はインストーラ内の LZMA の大きさ（音声はほとんど縮まないので無圧縮の近似）。

| プロジェクト | .flp B | FL 版 | チャンネル | パターン/ノート | プレイリスト | 生成器（種×個） | エフェクト種 | サンプル（形式×数） | サンプル圧縮後 B |
|---|---:|---|---|---|---|---|---:|---|---:|
| System\Empty | 46,361 | 24.2.99 | Sampler 1 | 0/0 | 0 | - | 0 | - | 0 |
| **RT\Sampler** | 81,831 | 20.0.3 | Sampler 1 | 1/7 | 0 | - | 0 | **RIFF PCM 1** | 8,117 |
| Templates\Minimal\Basic | 49,457 | 24.2.99 | Sampler 4 | 0/0 | 0 | - | 0 | RIFF PCM 4 | 109,615 |
| Templates\Other\Blocks | 57,737 | 24.2.99 | Sampler 4 | 0/0 | 260 (pat 208, clip 52) | - | 0 | RIFF PCM 1、RIFF Vorbis(0x674f) 3 | 25,196 |
| Templates\Utility\Record Audio | 49,385 | 24.2.99 | Sampler 4 | 0/0 | 0 | - | 0 | RIFF PCM 4 | 133,323 |
| RT\3OSC | 65,798 | 12.5.0.59 | native 1 | 1/7 | 0 | 3x Osc×1 | 0 | - | 0 |
| RT\Fruity Scratcher | 118,042 | 20.2.0 | Sampler 1 | 1/0 | 0 | - | 1 | RIFF Vorbis 1 | 193,011 |
| PD\Synth_dfr - Overcome - Sytrus | 492,590 | 11.1.0 | native 72、Automation 16 | 264/5,335 | 0 | Sytrus×72 | 0 | - | 0 |
| PD\Olbaid - Sytrus Essentials | 561,434 | 12.4.0 | native 125、Automation 36 | 125/4,559 | 0 | Sytrus×125 | 2 | - | 0 |
| PD\Electric Loop - The Ogun Dream | 582,657 | 20.0.5 | native 14、Automation 3 | 28/376 | 218 | Ogun×14 | 12 | - | 0 |
| PD\Olbaid - Harmless Essentials | 489,646 | 20.5.0 | native 77、Sampler 13、Automation 23 | 116/2,762 | 839 | Harmless×77 | 7 | RIFF Vorbis 12 | 102,720 |
| PD\Vocodex demo | 139,596 | 20.99 | native 5、AudioClip 2 | 8/49 | 8 | Sytrus×5 | 1 | RIFF Vorbis 1 | 186,754 |
| DS\Ookay - Thief（= NewStuff） | 313,356 | 25.2.3 | AudioClip 32、native 5、Automation 8 | 13/213 | 1,992 | 3x Osc×2、FLEX×2、Harmless×1 | 14 | wvpk 2、RIFF Vorbis 10、OggS 11 | 3,629,955 |
| DS\P.Priime - Use Me | 183,263 | 25.2.5 | AudioClip 20、Sampler 6、native 2、Automation 6 | 10/419 | 760 | Sytrus×2 | 14 | OggS 22 | 10,749,885 |
| DS\Electroconductor - Inner Days - Original | 1,974,800 | 21.2.99 | Sampler 8、native 5、Automation 1 | 24/348 | 51 | DirectWave×2、Harmor×1、Sytrus×2 | 8 | RIFF Vorbis 30 | 3,739,510 |
| DS\DEDZII - NIGHT TERROR | 738,895 | 24.2.99 | AudioClip 30、native 7、Automation 16、Sampler 1 | 46/1,351 | 3,850 | FLEX×2、Harmor、Morphine、PoiZone、Sytrus×2 | 23 | wvpk 19、OggS 2、PCM 2、fLaC 1、RIFF Vorbis 5 | 4,950,897 |
| DS\Astes - Bien Duro | 1,704,507 | 25.1.0 | AudioClip 160、Sampler 70、native 16、Layer 16、Automation 46 | 162/1,937 | 2,260 | BooBass、FLEX、Harmless、Harmor×4、MIDI Out×2、Ogun×4、Sytrus×3 | 29 | OggS 147、RIFF Vorbis 16、wvpk 11、fLaC 6、PCM 1 | 10,070,659 |
| Performance mode\nucleon - Housework | 138,269 | 11.0.3 | native 5、AudioClip 1、Automation 11 | 28/328 | 0 | Harmless×3、Slicex×2 | 4 | RIFF Vorbis 1、PCM 2（他 3 参照はインストーラに無い） | 1,039,555 |

- **FL 10〜12 の古いデモ（Performance、Visual、Square a Saw 等）は、サンプルの参照がインストーラに無いパス**（作者の機械の絶対パス）を含む（unresolved 46 参照 / 3,519）。そのままでは「サンプルが見つからない」処理に入る（未観測）。
- 20 本のデモ曲が要るプラグインの合併は **88 種（DLL 110 個、無圧縮の PE 約 408 MB）**、46 本で 97 種・123 個・約 472 MB、165 本で 111 種・139 個・約 524 MB（補助 DLL を含む。`plugins.tsv`）。**デモ曲の字義どおりの再生は G9 のプラグイン群を要求する。**

### 4.5 内蔵の音源だけで再生できるデモ（最小の G8 デモの提案）

| 候補 | 要るもの | 備考 |
|---|---|---|
| **`Render tests\Sampler.flp` + `SHP_Sawing Synth.wav`**（提案） | FLEngine のみ | **ノート 7 個**（鍵盤 24／36／48／60／72／84／96、各 48 tick = 0.25 s、位置 0／0.25／…／1.5 s（テンポ 120 BPM、PPQ 96）、velocity 100、pan 64）。サンプルは data 8,224 B = 4,112 フレーム（93.2 ms）、`smpl` チャンク付き。ノート位置と鍵から**期待する音の時間構造が計算できる**（実機の波形は無い） |
| `Templates\Other\Blocks\Blocks.flp` | FLEngine + **oggio**（3 サンプルが Vorbis-in-WAV）+ PCM 1 | プレイリストにサンプルのクリップ 52 個。実曲の形に近い最小 |
| `Render tests\3OSC.flp` | `3x Osc_x64.dll`（Delphi 4,348,928 B）+ 同じディレクトリの `engine_x64.dll`（MSVC 221,696 B） | プラグイン 1 個の最小。G9 の入口 |
| `Product demos\Synth_dfr - Overcome - Sytrus.flp` | `Sytrus_x64.dll`（Delphi 6,269,440 B） | サンプルもエフェクトも無い「本物の曲」（FL 11.1、ノート 5,335）。72 インスタンス＝負荷が高い |

### 4.6 サンプルの形式（先頭バイトの照合。`sample-formats-summary.tsv`。インストーラ内の 2,008 ファイル。書き出さずに最初の復号チャンクだけを見た）

| 拡張子 | 先頭バイトの形式 | ファイル数 |
|---|---|---:|
| .wav | RIFF/WAVE、fmt タグ **0x674f（Ogg Vorbis を RIFF で包んだもの）** | 1,268 |
| .ogg | OggS | 257 |
| .wav | OggS（中身は ogg） | 182 |
| .wv | wvpk（WavPack） | 167 |
| .wav | RIFF/WAVE、fmt タグ **0x6750**（Vorbis の別形） | 56 |
| .wav | RIFF/WAVE **PCM** | 59 |
| .wav | RIFF/WAVE float | 10 |
| .flac | fLaC | 7 |
| .mp3 | MP3 | 2 |

**拡張子で形式を決めると誤る**（`.wav` の 1,506 ファイルは PCM でも float でもない）。m2-inventory §2.4 の「.wav の 322 個が RIFF でない」とは別の数え方（こちらは RIFF だが Vorbis のもの 1,324 を数える）。

### 4.7 プラグインの読み方（読み）

`f_d8cab0`: プラグイン名 + `_x64.dll` を組み、`LoadLibraryW` → `GetProcAddress("CreatePlugInstance")` → 呼び出し → 失敗なら `FreeLibrary`。呼び元 `f_d950d0`←`f_d91280`←`f_d969d0` は**仮想メソッド**（`TPluginFormatFL`、RTTI 0xabe917。直接の呼び元 0）。FLP の解析からの直接 call の経路は**無い**。ホスト側 API は `TFruityPlugHost`（RTTI 0x7b6947、インスタンス 0x108 B）。**プラグインの名前→パスの対応（`TPluginLocator`／`TPluginScanner`／`Data\System\Plugin databases` の 1,310 ファイル 7.0 MB）は読んでいない（G9 の供給の不確実性）。**

## 5. サンプルの復号に要る未 lift の同梱 DLL（項目 4。`unlifted-dlls.tsv`）

抽出: `Shared\` から読み取りのみ（wavpack／oggio／ildsp／LAME の SHA-256 は m2-inventory と一致）。`dll_profile.py` は `.pdata` の関数範囲を capstone で数えた（命令の群は capstone の group 名）。**どれも `fe-lift` は未実行**。

| DLL（インストーラ内 `Shared\`） | サイズ B | 機種 | `.pdata` | EH／UH | import（名／行あり／行なし） | export | SSE2 | AVX／AVX2／AVX-512 | x87 | jmp reg／jump-table 様 | 役割（読み）と必要な場面 |
|---|---:|---|---:|---|---|---:|---:|---|---:|---|---|
| `wavpackdll_x64.dll` | 271,672 | x64 | 706 | 35／62 | 83／70／13 | 48 | 61 | 0／0／0 | 0 | 8／5 | WavPack。`.wv` 167 ファイル（8.3%）。ローダー `f_a58ba0`（"WavpackDLL_x64.dll"、`WavpackOpenFileInputEx`）。**最も素直** |
| `oggio_x64.dll` | 1,872,384 | x64 | 2,575 | 229／380 | 95／81／14 | 9 | 34,188 | 1,954／72／130 | 14 | 131／114 | Ogg Vorbis と FLAC の読み（`OggioLoadFile`／`OggioLoadFileW`／`OggioLoadFLACFromMemory`）と Vorbis 書き出し。Vorbis-in-WAV 1,324 + OggS 439（87.8%）。ローダー `f_675310`（"oggio.dll"）。IPP の静的リンク（`IPPCODE`／`IPPDATA`）、CPUID による分岐あり。**CPU 申告は SSE2 までに固定済み（決定 0009／0075）なので AVX 系の本体は到達しない読み** |
| `ildsp_x64.dll` | 4,197,376 | x64 | 2,622 | 92／146 | 73／70／3 | 10 | 72,961 | 50,703／34／23,929 | 944 | 32／19 | **DSP 効果のクラス**（`Limiter`／`LimiterLite`／`PSOLA`／`Shimmer`／`Transmitter`／`StereoEnhancer` 等の `*_CreateInstance`）。サンプルの復号ではない。FLP 解析の閉包に `f_66ea10`（"ildsp" のローダー）が入る（`f_f51fd0`→`f_ea7ae0`→…→`f_66ea10`、直接 call）。NULL を許す分岐（loadlibrary-names-classification §9。決定 0113）。G9 |
| `LAMEenc_x64.dll` | 465,720 | x64 | 1,019 | 84／113 | 71／62／9 | 9 | 3,208 | 665／4／0 | 3 | 10／3 | MP3 書き出し（`be*`）。`cpuid`・`xgetbv`・`MessageBoxA`・`SystemFunction036`。G10 |
| `EasyFLAC_x64.dll` | 394,240 | x64 | 1,008 | 63／88 | 76／71／5 | 10 | 2,938 | 1,274／802／0 | 1 | 33／12 | FLAC の読み書き（`EasyFLAC_*`）。FLAC の読みが oggio 経由か EasyFLAC 経由かは**未確認** |

- 行なし import（すべて kernel32 の CRT 由来。`user32!MessageBoxA` と `advapi32!SystemFunction036` は LAME）: wavpack `GetFileInformationByHandle`／`PeekNamedPipe`／`TlsFree`／`HeapDestroy`／`GetConsoleCP`／`LCMapStringA`／`GetStringTypeA`／`GetLocaleInfoA`／`WriteConsoleA`／`CreateFileA`／`SetEndOfFile`／`CompareStringA`／`SetEnvironmentVariableA`。oggio `InitializeCriticalSectionEx`／`LCMapStringEx`／`GetLocaleInfoEx`／`CompareStringEx`／`IsProcessorFeaturePresent`／`TlsFree`／`GetFileInformationByHandle`／`PeekNamedPipe`／`ReadConsoleW`／`GetFileSizeEx`／`GetTimeFormatW`／`GetUserDefaultLCID`／`EnumSystemLocalesW`／`SetEndOfFile`。**CRT の起動コードが `GetProcAddress` で引くものかどうかは未確認**（決定 0092／0081 の先例 = ロケール系の応答を足す作業）。
- 規模の読み: wavpack は被覆 98 KB で 11〜14 倍の係数（m2-inventory §5.2）なら wasm 1.1〜1.4 MB、oggio は被覆 880 KB で 9.7〜12.4 MB。**dsp_ippv2 で出た問題（SSE3 `LDDQU` の未対応 975 箇所、locals 5 万超の関数 15 個など）が oggio で再現するかは未確認**。
- 実在モジュールの枠: `RM_MAX = 8`（決定 0106 の**選択**）で、非 main の実在モジュールは dsp_ippv2・Blend2D・QFC・ILTools・freetype の 5。wavpack + oggio で 7。**G9 のプラグインは別の方式が要る**（m2-inventory §5.5、§7）。
- 別の DLL として **`DelZip64.dll`（362,808 B。export に `DZ_Exec` など）**は `.zip` の展開用と読める（`f_11d58c0` の zip の枝。FLEngine の文字列に `DZ_Exec` がある〔m2-inventory §3.2〕）。.flp 単体には要らない。

## 6. 仮想ボリュームに要る供給物（項目 5）

### 6.1 現在の種との差（`lift/harness/seeds/vfs-seed-idle1.tsv`、218 ファイル + ディレクトリ行 19、5,638,150 B ≈ 種のパック 5,733,627 B）

| 段階 | 追加ファイル | 件数・B（無圧縮） | 備考 |
|---|---|---|---|
| G8a-0（`Empty.flp` を開く） | なし（`Data\System\Empty.flp` は種に有る） | 0 | 起動時に通る（§2.2） |
| G8a（`Basic.flp`） | `Basic.flp`（49,457）+ `Packs\Legacy\Drums\Dance\Basic 808 Clap.wav`（46,492）・`HiHat`（28,180）・`Kick`（74,468）・`Snare`（32,992） | 5 件・231,589 | PCM 16 ビット・ステレオ。ユーザー側 .flp の置き場は `C:\Users\user\Documents\Image-Line\FL Studio\Projects\`（エンジンが起動時に列挙。決定 0129 の記録） |
| **G8b／c（`Sampler.flp`）** | `Sampler.flp`（81,831）+ `Packs\Shapes\Misc\SHP_Sawing Synth.wav`（8,336） | **2 件・90,167** | 最小 |
| G8d（`Blocks.flp`） | `Blocks.flp`（57,737）+ `Packs\Legacy\Drums\Dance\DNC_Clap_5.wav`（14,448、PCM）・`DNC_Hat.wav`（4,856）・`DNC_Kick.wav`（5,254）・`DNC_Snare_4.wav`（6,088）（Vorbis-in-WAV）+ **`oggio_x64.dll`**（1,872,384） | 6 件・約 1.96 MB | DLL は種ではなく DLL パックへ（決定 0137） |
| G9 入口（`3OSC.flp`） | `3OSC.flp`（65,798）+ `Plugins\Fruity\Generators\3x Osc\3x Osc_x64.dll`（4,348,928）+ `engine_x64.dll`（221,696） | 3 件・4.64 MB | ローダーを `LoadLibraryW` の実在モジュール化する設計が先 |
| デモ曲 20 本そのまま | .flp 20（`Demo songs` 29.3 MB）+ サンプル 889 ファイル 約 102 MB（圧縮後の近似）+ プラグイン DLL 110 個 約 408 MB + oggio + wavpack | — | §6.2 |
| ブラウザ（Data） | `Data\Demo projects`（46、49.1 MB）、`Data\Templates`（35、30.8 MB）、`Data\System\Render tests`（82、7.7 MB） | .flp だけで 87.7 MB | |

### 6.2 見積もりと配信の論点（短く。決定 0134／0137 の知見と合わせる）

- **ベースライン**（実測、決定 0134／0137）: wasm 667,115,949 B（brotli q11 26,525,655 B、V8 の単一 wasm 1 GiB の 62.1%）、線形メモリの最終値 930 MiB（974,848,000 B）、種のパック 5,733,627 B（brotli 3.3 MB、FLSEEDP1 を MEMFS に書き、C 側の VFS が起動時に中身を読み込む）、DLL パック 6 個 70.6 MB（brotli 14.5 MB）、合計 brotli 約 34 MB。種の配信は「(a) パック」を採用、**遅延読み込みは範囲外（未実装）**。
- **ユーザーの .flp とサンプルは実行時に来る**ので、種のパックには入れられない。MEMFS と C 側の VFS（`fle_vfs.c`、ファイルは C ヒープのノート）の**二重保持**が起きる（`vfs_seed.h` は `FS.writeFile` したものを C の VFS に `WriteFile` で複写し、1 ファイル 64 MiB 以下の制限）。930 MiB の線形メモリ（`lift/harness/wasm/build.sh`: 初期 640 MiB、ゲストの VirtualAlloc アリーナ 512 MiB、`ALLOW_MEMORY_GROWTH`。**上限の設定は読んでいない〔未確認〕**）に 100 MB 級のデモを足すと余裕が減る（Chrome の renderer RSS は 0138 で 1.9〜2.0 GB）。
- 配信の候補（意見）: ① 最小デモ（約 90 KB）は種のパックに足すだけ（5.7→5.8 MB）。② デモ全体は R2 に Range で置き、**エンジンが `CreateFileW`／`FindFirstFileW` で開いた時に JSPI で取りに行く遅延読み込み**（0137 の (c)）か、ページが先に必要なファイルをまとめて取って `fle_app_put_file` で足す。③ プラグイン DLL（524 MB）は今の「1 つの wasm に全部」では 1 GiB を超える（m2-inventory §6）ので別モジュール化が先で、G8 の範囲外。
- **`Data\Patches` 全体**（圧縮後 Packs 283.0 MB、Misc 77.7 MB、Plugin presets 49.1 MB、Impulses 42.8 MB）は G8 には要らない。デモ曲が参照するのは 889 サンプル（約 102 MB）。

## 7. G8 の段階案（意見。決めない）

共通の注意: 受け入れ条件の「native と wasm の面の SHA 一致」「ring のブロックの SHA 一致」は既存の方式（決定 0136／0138／0139）。**実機の書き出し WAV と画面が無いので、「実物と同じ」は主張しない**。時間の見積もりは書かない（桁だけ）。

| 段階 | 内容 | 観測できる受け入れ条件 | 要る壁の見込み（API・DLL・モデル） | 規模の桁（読み） | 音声の速度問題との関係 |
|---|---|---|---|---|---|
| **G8a-0** | **観測だけ**: 起動時の `Empty.flp` の読み込みが実際に走ったかの確認 | `f_f54760`／`f_f51fd0` への入口数 ≥ 1、その戻りと `ReadFile` の合計バイト = 46,361（gdb か `FLE_FRUITY_…` の関数入口の記録。**新しい実装は不要**） | 無し | S | 無関係 |
| **G8a** | 内蔵だけの別のプロジェクト `Basic.flp`（PCM サンプル 4）を開く | ① `CreateFileW(…Basic.flp)` と 49,457 B の読み、② 面の SHA が `Empty` の面と異なり native と wasm で一致（チャンネルラックの 4 行 "808 Kick"／"808 Clap"／"808 HiHat"／"808 Snare"、テンポ表示 130.000。人が画像で確認）、③ 止まらず idle に戻る。サンプル 4 個の `CreateFileMappingW`／`MapViewOfFile` が通る | `SHAddToRecentDocs`（条件付き）、`GetFinalPathNameByHandleW`（遅延）、サンプル検索の `FindFirstFileW`、ダイアログ（"missing sample" 等）。**FLP の解析本体の壁は未知**（過去 16 壁連続、行なしの静的な数は当てにならない） | 入口の実装は S（`GetCommandLineW` の値を引数付きにする、VFS に足す）。壁は M〜L（数は見えない） | 影響が小さい（音を出さない） |
| **G8b** | サンプル 1 個＋ノートの最小プロジェクト `Sampler.flp` を開く | 上に加え、① サンプルの読み（`SHP_Sawing Synth.wav` 8,336 B）、② チャンネルラックの "Sampler" にサンプル名／波形、③（任意）ピアノロールを実入力で開くとノート 7 個が出る（面の差） | WAV の解析 `f_670fd0`、SRC／ピッチ（dsp_ippv2 の `ipps*` が呼ばれる可能性）、サンプルの `smpl` チャンク | S〜M | 無関係（再生しない） |
| **G8c** | **再生**して AudioWorklet（ring）に非ゼロのサンプル | ① native（仮想時計）で、Space／Play の後 N 仮想秒の間に **ring に書かれるブロックのうち max\|x\|>0 のものがある**。非ゼロの塊の開始が **0／0.25／…／1.5 s**（7 ノート、120 BPM）に概ね一致、ノート間は小さい／0、② wasm（Chrome）で ring の最初の N ブロックの SHA が native と一致、③ Worklet が非ゼロのブロックを**1 個以上**読む。**アンダーランは許す**（連続ゼロは G7 の字義の問題として別立て） | Play = `StartActionExecute` `f_f34ac0` → `f_11ca4e0` → `f_11c9e70` → `f_1253610` → `f_1253260`（TMixerThread_DS、DirectSoundCreate）の鎖（実測: 直接 call）。ミキサー本体 `f_d0ece0`（412 B）は `f_12aeec0` が直接呼ぶ。`f_d0ece0` の直接 call の閉包は 4,422 関数・import 266（行あり 248／行なし 18）、tier 2 で 5,628 関数・312（268／44。`msacm32` の `acm*` 12 名が加わる）。行なし 18 名はすべてプロジェクト読み込みの閉包（tier 1）の行なしに含まれる（新しい名前は増えない。静的な上限）。音源の呼び出しは `TFruityPlugHost`＋プラグインの vtable 経由（`Sampler` は内蔵）。ミキサーの 1 ブロックの計算量は**未測定**（空のプロジェクトで平均 6.5 ms／92.9 ms、決定 0139） | M | **本質**: 決定 0139 は実エンジンで 98.1% アンダーラン（60 s、仮想時計が 88 s で 424 ms）。原因は他スレッドの枠（メイン約 2.5 s + スレッド 2 約 0.5 s / 仮想 16 ms）。決定 0138 の定常は 1.2〜1.8 フレーム/s、`fle_mx_*` が CPU の 67.6%。進行中の fp1（決定 0143）の結果次第で「連続再生」の可否が変わる。**よって G8c の条件は『非ゼロ』であり『連続』ではない**（仮想時計の下では決定的に測れる） |
| **G8c-RT**（別立て） | 実時間で連続して鳴る | Worklet のアンダーラン 0 を一定時間 | 上に加え UI 描画との競合（スレッド 2 = dsp_ippv2、`WarpAffineQuad` が CPU の 98.9% — 依頼文の記載。本調査は再測定していない） | L（HLE を含む） | fp1 の成否に依存 |
| **G8d** | oggio（Vorbis）を要する内蔵だけの `Blocks.flp`（サンプル 3 が Vorbis-in-WAV） | 上と同じ形（クリップ 52 個・ノートなし） | **`oggio_x64.dll` の lift**（実在モジュール 6 個目。IPP＋AVX 系の未対応は到達しない読み）、`OggioLoadFile(W)` が呼ぶ CRT のファイル I/O が仮想ボリュームを通ること、`GetFileSizeEx` 等の CRT 行 | M〜L | 同上 |
| G8e（G9 へ） | プラグイン 1 個（`3OSC.flp`）、wavpack、デモ曲 | プラグインの GUI と音 | プラグイン DLL の実在モジュール化の一般化（§5）、`TPluginFormatFL` の経路、プラグイン DB | L〜XL | 同上 |

**推奨（意見）**: G8a-0 → G8a（`Basic.flp`）→ G8b → G8c の順（oggio を待たずに『開く → 鳴る』が通る）。G8d は oggio の lift の難しさを見る実験として並行で `fe-lift` を走らせる。**ゴールの字義（デモ曲）は G9 の領域**（88 種のプラグイン）で、G8 の「同梱デモ」は『内蔵だけ』に絞った上の 12 本の部分集合が現実的。

## 8. アプリ側（`app/`）に要るもの（項目 7、F5 の入口）

現状（実測: `app/src/engine/fruity.ts`、`app/README.md`）: エンジンは Worker で動き、種のパックを MEMFS（`/data/seed/…`）に書き、ハーネスが起動時に C 側の VFS へ複写する。入力は `fle_input` の生イベントのキューに入り、エンジンの idle で配られる。**実行中に VFS へファイルを足す export は無い**。

最小の経路の案:

1. ブラウザで `<input type=file>`／ドロップ → `ArrayBuffer`。ページ → Worker へ転送（`engine/protocol.ts` に `openProject` メッセージ）。
2. Worker: `_malloc` した領域へ書き、**新しい C の export `fle_app_put_file(path_utf8, ptr, len)`**（`vfs_seed.h` が使う `CreateDirectoryW`／`CreateFileW`／`WriteFile`…のゲスト向けスタブを実行中に呼ぶだけ。`lift/runtime` の既存関数で足りる読み。1 ファイル 64 MiB 以下の制限と二重保持に注意）で `C:\Users\user\Documents\Image-Line\FL Studio\Projects\<name>.flp` に置く。サンプルも同様（パスの対応方針は §10）。
3. 「開く」を伝える。選択肢:

| 案 | 内容 | 偽りの有無 | 規模（桁） | 壁の見込み |
|---|---|---|---|---|
| **A. 起動時のコマンドライン** | `GetCommandLineW` が `"C:\…\FL64.exe" "C:\…\x.flp"` を返す（新しい環境変数 `FLE_CMDLINE` 相当を、manifest の `engine.env`〔決定 0137 で既に有る〕から渡す。ファイルは起動前に VFS へ） | **無し**（FL64.exe を引数付きで起動するのと同じ。パスが仮想なだけ） | S（ランタイム 数十行 + 種／Worker） | 起動時にしか使えない。ウィザード（GDPR・版の選択）は先に出る。`f_11ef180` が `.FLP` を見た後の鎖と `f_117f780` の経路は未追跡 |
| **B. `WM_COPYDATA`（dwData=10）** | Worker が `fle_app_open(path)` を呼ぶ → C が UTF-16 のパスと `COPYDATASTRUCT`（`dwData=10`、`cbData=(len+1)*2`）をゲストのメモリに作り、`FindWindowW` と同じ窓表の探索で `TFruityLoopsMainForm` の窓を得て、**`SendMessageW(hwnd, WM_COPYDATA, 0, &cds)` と同じ経路**で配る（`fle_input.h` 64 行の `fle_win_input_send(api, hwnd, msg, wp, lp)` が入力で既に使っている窓への送信なので再利用できる読み）。`fle_input` のキューと同じ形（idle で）にできる | **小**: 送り手が「2 回目に起動された FL64.exe」の代わりに我々。`wParam`（送り手の hwnd）は 0 | S〜M（C 100〜200 行 + JS）。窓表に class `TFruityLoopsMainForm`（lid 13、スレッド 0、タイトル "FL Studio"→"FL Studio 2026"）が**実在する**（実測: `docs/investigations/idle1/logs/fruity-on/fon1i/rerun.out` 892 行、決定 0141）。ウィザードが開いている間は主窓が無効（入力のみ遮断。`SendMessage` は届く読み） | `f_92d0f0` のキューの消費側（どのスレッド・どのタイミングで `TLoadFlpAction.Execute` が走るか）は**未追跡**。起動中（ウィザード表示中）に届くと、モーダルの扱いが変わる可能性 |
| **C. File > Open（Ctrl+O／メニュー）** | キー入力で `FileOpenAction` → COM の `IFileOpenDialog` のモデルを作り、`Show` をブラウザの選択に写す | 中: ダイアログの見た目はブラウザのもの。エンジンから見ると正規の COM の結果 | M〜L（COM 約 20 メソッド + `IShellItem` + `SHCreateItemFromParsingName` + モーダルの待ち。`TFavFileOpenDialog` の拡張は未読） | `Ctrl+O` のキーが `FileOpenAction` に届くか（ショートカットの割り当ては未調査）。モーダルのネスト（G10） |
| **D. OLE のドロップ** | `RegisterDragDrop` で記録済みの `IDropTarget` に `DragEnter`／`DragOver`／`Drop` を `fle_call_guest` で呼ぶ。`IDataObject`（`QueryGetData`／`GetData(CF_HDROP)`／`EnumFormatEtc`）をランタイムが実装 | 小（ドロップの意味は同じ）。座標・キー状態の合成 | M（IDataObject 約 10 スロット + DROPFILES のメモリ + ポインタ位置の対応 + `f_124b8d0` の中の壁） | `CLSID_DragDropHelper` の `CoCreateInstance` は REGDB_E_CLASSNOTREG（既に許容） |
| E.（試験用）文字列 `0x16bcca8` へ書いて `PostMessageW(main, 0x566)` | ゲストのグローバルを直接書く | **大**（固定 VA の突っつき。OS の経路ではない） | S | 版ごとに VA が変わる。**本番には使わない**（診断用の最終手段） |

推奨（意見）: **F5 の最小 = A（起動時）+ B（実行中）**。D は「ブラウザにドロップ」という Web らしい操作の本命だが、B で同じ効果が得られるので後。C は G10。ユーザーの .flp が参照するサンプルは作者の機械のパスなので（§4.4）、アプリは同じフォルダーのサンプルを `Projects\<name>\` 等に置き、FL の「見つからないサンプルを探す」挙動に任せる案と、zip（`DelZip64` が要る）を JS で展開してから置く案がある（どちらも**未検証**）。

## 9. 限界・確認できなかったこと

- **実行していない**。`f_f51fd0` に実際に入ったか、`f_11d58c0` が最後まで通ったか（決定 0139 の gdb で `f_1253ae0` を呼ぶところまでは出ているが、`Project loaded...` は出力に無い）、各「行なし」を実際に踏むかは**未確認**。
- 呼び出しの到達は**直接 call のみ（tier 1）**＋ lea（tier 2）。`CreatePlugInstance`・ミキサー・oggio／wavpack のローダー・WM_COPYDATA の後段は仮想呼び出しで辿れない。
- **FLP のイベント名は第三者（PyFLP）の読み**。未知の ID が多く、`PluginWrapper`／`MixerParams`／`TrackData`／`PluginData` の中身は読んでいない（`Fruity Wrapper` の VST 部分だけ構造を読んだ）。サンプルの参照の抽出は「プラグイン状態の中の UTF-16／ASCII 文字列で、音声の拡張子で終わるか `%FLStudio` を含むもの」という**経験則**（FL の独自の保存形式の網羅ではない）。unresolved 46 / 3,519 参照は作者の機械の絶対パス。`%FLStudioData%` = `<install>\Data`、`%FLStudioFactoryData%` = `<install>` は**サンプルの参照が解決できたこと**から読んだ（FL の定義ではない）。
- インストーラは「必須の節」だけを線形に歩いた（決定 0129 と同じ限界。条件分岐・他の節は評価していない）。
- DLL の「難しさ」は import・セクション・命令の群からの読みで、`fe-lift` を走らせていない。AVX 系が実際に到達しないかは CPUID の応答（SSE2 まで）に依存。
- `Render tests` が回帰試験用という解釈、`Sampler.flp` の期待する音の時間構造（`smpl` チャンクの扱い、declick、resampler）は**実機の WAV が無く検証できない**。
- Windows の何版に対し `IFileOpenDialog` が選ばれるかの分岐、`Ctrl+O` の割り当て、Play の他の経路（`PlayBtn` のクリック処理、Space）は読んでいない。
- サンプルの参照にある `%FLStudioFactoryData%`／`%FLStudioData%` をエンジンがどのパスに展開するか（`TFLPathManager`、RTTI 0x9db4e7）は**読んでいない**。種のツリーは決定 0129 の `C:\Program Files\Image-Line\FL Studio 21\` 配下で、同じ接頭辞に `Data\Patches\…` を置けば解決する、という前提（未確認）。
- `GetFinalPathNameByHandleW` の遅延ロードの欠落時の挙動、`MachineGuid`／`GetComputerNameExW` の応答が FLP 解析に与える影響（`f_f51fd0` が読む）は未確認。レジストリの `MachineGuid` は種に無い（`grep` で 0）。
- 時間: 開始 10:25（JST）。本書の初稿は 11:25 まで、以後は検証と訂正。

## 10. PM の判断が要る点

1. **F5 の入口**: A（起動時のコマンドライン）+ B（`WM_COPYDATA`）で進めてよいか。A だけでまず G8a〜c を測る案もある。
2. **最小の G8 デモの定義**: `Render tests\Sampler.flp`（内蔵のみ、PCM）を「G8 の最小」とし、goal-ladder の「同梱デモ」を『内蔵だけの 12 本の部分集合』に読み替えるか（字義のデモ曲は G9 のプラグイン 88 種を要求する）。
3. **G8c の受け入れ**: 『非ゼロ（仮想時計で決定的）』を G8c、『実時間で連続』を別立て（G8c-RT）にしてよいか（fp1 待ち）。
4. **oggio の lift の優先度**: 87.8% のサンプルが要るが、最小の G8 には要らない。G8d を並行の実験にするか、G9 の後か。
5. **アプリ側の `fle_app_put_file` と `fle_app_open`** の追加（`lift/runtime`／`harness` への 2 関数）を許すか（他レーンが `lift/runtime` で作業中）。
6. **ユーザーの .flp のサンプルの方針**（作者のパスは解決しない。同じフォルダー／zip／警告を受け入れる）。
7. **供給の方式**: 最小デモ（約 90 KB）は種に足す。デモ全体は遅延読み込み（0137 の (c)）か先読みか。
8. **起動時の `Empty.flp` の読み込みが実際に走っているか**を G8a-0 として先に確かめてよいか（新しい実装なしの観測）。

出典なしの選択（本調査）:

| 名前 | 値・内容 | 理由 | 撤回条件 |
|---|---|---|---|
| `S-g8-min-demo` | 最小のデモ = `Render tests\Sampler.flp`、内蔵だけの定義 = 「プラグイン DLL もエフェクトも VST も無い」 | 165 本の全数照合で 12 本が該当し、再生ボタンだけで音が出るのは 2 本 | PM が別の定義にしたとき |
| `S-g8-nonzero` | G8c の合格 = ring に max\|x\|>0 のブロックがあり、塊がノート位置に概ね一致 | 実機の波形が無い。決定 0139 の実時間が崩れている | PM が別の条件にしたとき |
| `S-g8-entry` | 起動時 = コマンドライン、実行中 = `WM_COPYDATA` を推奨 | エンジン自身の外部入口で、偽りが最小 | 実走で壁が大きいとき |
| `S-g8-resolve` | サンプル参照の解決: `%FLStudioFactoryData%`／`%FLStudioData%`／`C:\…\Image-Line\FL Studio*\` の尾を install の相対パスに写す | 参照の 98.7% が解決でき、解釈が整合 | 実機の FL の定義が示されたとき |

## 11. 規則違反の自己申告

- **`timeout 1 true` を含むコマンドを 1 回書き**、ハーネスに拒否された（実行されていない）。sleep／nohup／末尾 `&`／`pgrep -f`／while ループは書いていない。
- **中間報告 1（10:46）に「約 1 時間」、報告 2（11:10）に「約 1.5 時間」と書いたが、実際の経過は 20 分強と 45 分だった**（所要時間の見積もりの誤り。報告 3 は 11:25、開始 10:25）。報告 2 に書いたサンプル形式の数字（PCM／float 51 本、wavpack 134 本、1,843 ファイル、「約 88%」）は、**サンプル参照の抽出の不備**（プラグイン状態の中の文字列に前置バイトが付いて解決できなかった、`%FLStudioData%` を解決していなかった）により不完全だった。本書の数字（2,008 ファイル、PCM 59 + float 10、wavpack 167、Vorbis 1,324 + OggS 439）が修正後。
- 作業ツリーは**新規の docs ファイルのみ**。git の変更系操作なし、commit なし。`lift/runtime`、`lift/crates`、`lift/out-lay1`、`lift/out-clip1`、`lift/out-fp1` には**書き込んでいない**（`lift/out-lay1`／`out-clip1`／`out-fp1` は開いてもいない）。読み取りで読んだもの: `lift/runtime/` の `fle_env.c`／`fle_vfs.c`／`fle_ole.c`／`fle_vmap.c`、`lift/harness/vfs_seed.h`、`lift/crates/fe-lift/src/imports_table.rs`、`lift/out-idle1/inputs/`、`lift/out-fruity6/inputs/FL64.exe`（読み取りは許可されている）。ビルド・FLEngine の実行はしていない。
- メモリ: python の最大 RSS は索引の作成 0.69 GB。数 GB は使っていない。`/tmp` 直下には書いていない（`/tmp/opencode/g8in/` のみ、最大 113 MB）。
- ネットワーク: PyFLP のソース 7 ファイル（curl 5 + webfetch 2）、flp-file の README（webfetch）、`src/flp-event.ts` の raw URL（404）、Image-Line のフォーラムのページ 1（webfetch。**webfetch が約 3 千行のページを返し、出力が長かった**。ページの末尾にエージェントへの指示に見える文は無かった）。資格情報・ブラウザのプロファイルには触れていない。
- ツールの出力・取得ページに依頼者の指示でない文: 取得したページ・ログの中に「別の仮説を試せ」「編集 N 回」の類の文は**見当たらなかった**（コミットメッセージの引用文に過去の同種の出来事の記述があるが、私への指示ではなく従っていない）。
- `__pycache__` は repo 内に作っていない（`tools/__pycache__` は調査前から存在）。

## 12. 成果物

- `docs/investigations/g8-project-load.md`（本書）。
- `docs/investigations/g8-project-load/`:
  - TSV: `demos.tsv`（165 本）、`plugins.tsv`（111 種・DLL・無圧縮サイズ）、`render-tests.tsv`（82 本）、`sample-formats-summary.tsv`、`samples-candidates.tsv`、`unlifted-dlls.tsv`、`open-path-none-apis.tsv`。
  - テキスト: `open-path-chains.txt`（呼び出しの鎖と根拠の区分）、`empty-flp-structure.txt`。
  - スクリプト（標準ライブラリ、capstone、pefile のみ。`PYTHONDONTWRITEBYTECODE=1`）: `g8_extract.py`（インストーラの読み取りと抽出）、`flp_events.py`（FLP のイベント読み）、`flp_survey.py`（チャンネル／ミキサー／サンプルの抽出）、`flp_tables.py`（165 本の表）、`sample_sniff.py`（先頭バイトの照合）、`g8_trim_tables.py`（表の整形）、`dll_profile.py`（DLL の静的プロファイル）、`g8_make_extra_tables.py`（`unlifted-dlls.tsv`／`open-path-none-apis.tsv`）、`empty_flp_report.py`（`empty-flp-structure.txt`）、`fle_query.py`／`fle_closure_table.py`／`fle_disasm.py`／`fle_dataref.py`／`fle_vmt.py`／`fle_method.py`／`fle_msgtab.py`／`fle_scan_store.py`／`fle_scan_imm.py`（FLEngine の静的解析。`goal-ladder/fle_index.py` の pickle が前提）。
- 再現（要約）: ① `python3 docs/investigations/goal-ladder/fle_index.py binaries/win64/FLEngine_x64.dll /tmp/opencode/g8in/fle.pkl`。② `python3 lift/harness/seeds/gen_install_tree.py INSTALLER --list > tree.tsv`。③ `g8_extract.py INSTALLER /tmp/opencode/g8in/flp m.tsv '\.flp$'`。④ `fle_closure_table.py /tmp/opencode/g8in/fle.pkl lift/crates/fe-lift/src/imports_table.rs /tmp/opencode/g8in/clos f51fd0=FLPparse 11d58c0=OpenProject_f11d58c0 …` → `g8_make_extra_tables.py DLLDIR imports_table.rs /tmp/opencode/g8in/clos-imports.tsv docs/investigations/g8-project-load`（DLLDIR は `g8_extract.py` で `Shared\…_x64.dll` を抽出した場所）。⑤ `flp_tables.py /tmp/opencode/g8in/flp tree.tsv OUT` → `sample_sniff.py INSTALLER OUT/samples.tsv OUT/sample_formats.tsv` → `g8_trim_tables.py OUT docs/investigations/m2-inventory/pe-inventory.tsv docs/investigations/g8-project-load`。
- 一時物 `/tmp/opencode/g8in/`（抽出した .flp・サンプル・DLL・索引の pickle など 113 MB）は完了時に `rm -rf` で削除し、`ls /tmp/opencode` に残っていないことを確認した。再現には上の手順で再抽出が要る。
