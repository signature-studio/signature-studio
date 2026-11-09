# 0178: File → Open... の COM モデル(IFileOpenDialog)で、実エンジンが FLP を開いた — Sampler.flp は開いて描画・サンプル読み込みまで、参照プロジェクトの FLP は読み込みを始めて最初のプラグイン(チャンネル 1 の Fruity Wrapper)の DLL で止まる。壁は台帳 17 件(修正 9、開いた 1、確定した次の壁 2〔Fruity Wrapper の DLL、.zip 経路の DelZip64.dll〕、探索 5〔MP3 は ACM、失敗すると ilvideo〕)。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0178、担当 dlg1、2026-11-03 13:37〜、予算 4 時間)。作業と測定は HEAD **b4f1d3c** の `git archive` した木(`lift/out-dlg1/src/`)で行い、`dlg1.diff` は **HEAD 912a2ca**(docs だけが 7757f60 から進んだ)の `git archive` に `patch -p1 --dry-run` が当たることを確かめた。生成 C の基準には、同じ生成 C(決定 0171 の層 1+2 の行を足した D')の leak1 の native -O0 オブジェクトをハードリンクで借りた(読み取りのみ)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝名前つき(`S-dlg1-…`)で `tables/s-choices.tsv` に代替案と撤回条件を書いた。**実機 Windows との照合は 0 件**。速度は主張しない(この機械は load 7〜37 で他の担当が走っていた。エンジンの起動から最初の idle まで 1 回 3〜6 分の壁時計)。

## 冒頭の三点(正直さ)

1. **実行した本物と範囲(実測)**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → native -O0 にした物。生成 C は決定 0171 の層 1+2 の行 + 本決定の行〔`harness/seeds/dlg1-imports.txt`〕を `fe-lift --extra-imports` で足した物。FLEngine の関数本体が変わる単位だけ再コンパイルし、他は leak1 の同じ生成 C のオブジェクト)を、G3 ハーネスの **fork プローブ**(最初の idle の同じ状態から 1 操作ごとに fork した子で実行)で動かした。操作は決定 0171 と同じ座標のクリック(File (18,20)、Open... (48,85))。**ダイアログの『選択』だけはハーネスが与えた**(環境変数 `FLE_FILEDLG_PICK_FILE` の一覧ファイル。利用者の入力の模擬で、エンジンの出力の代役ではない。`Show()` のたびに stderr に『THE SELECTION WAS GIVEN BY THE HARNESS』と出す)。ほかに代役・モック・画像の再生・事前生成は無い。画像(`tables/shots/sampler-open-main.png`)は実エンジンの面を BGRX から PNG にしただけ。**探索**(`FLE_LOADLIB_MISSING`、`dlg1-explore-imports.txt` の ACM の行)は『プラグイン DLL は見つからない』『ACM ドライバは無い』という**仮定を env と行で入れた探索**で、結果ではない(§12)。ブラウザ側は設計と `emcc -c` が通る下書き(`lift/harness/web/filedlg/`)だけで、**何も実行していない**。
2. **結論**: (a) 決定 0171 の U1 の原因は読みどおり: エンジンは `CoCreateInstance(CLSID_FileOpenDialog {DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7}, NULL, CLSCTX_INPROC_SERVER, IID_IFileOpenDialog {D57C7288-D4AD-4768-BE02-9D969532D960})` を呼ぶ(印字して確認。Learn / SDK ヘッダの値と一致)。(b) 本決定の COM モデル(`lift/runtime/fle_filedlg.c/.h`、vtable は応答表の行、opt-in)で、エンジンは **File → Open... で Sampler.flp を実際に開き**(主窓のタイトルが "Sampler.flp - FL Studio 2026"、Sampler 窓に `SHP_Sawing Synth.wav` の波形、exit 0)、**参照プロジェクトの FLP は読み込みが始まり**、2 つの CompareStringW の壁を越えて、**チャンネル 1「Serum」(Fruity Wrapper)のプラグイン DLL を探して見つからない所**(`Plugins\Fruity\Generators\Fruity Wrapper\{Fruityplug_x64.dll, Fruity Wrapper_x64.dll}`、続いて `LoadLibraryW("")`)で止まる。これは G8 の**確定した次の壁**(Fruity Wrapper と Serum2.vst3 が未 lift・未配置。G9/G11)。(c) `.zip` を選ぶ経路は、エンジン自身が展開する(`DelZip64.dll`)ので、3 件の壁を越えて `LoadLibraryW("...\Shared\DelZip64.dll")` で止まる(未 lift の同梱 DLL)。
3. **未確認・未着手**: 実機との照合(0 件)。ダイアログのイベント(OnFolderChange ほか)の順序、お気に入りメニューの選択、IFileDialog2 など実機のダイアログが持つが本モデルに無い IID(未知の IID は名前つき停止で、エンジンが QueryInterface したのは IFileDialogCustomize だけ)。MP3 の復号(ACM の先、ilvideo 〔FFmpeg〕。§12)。File → Save as の往復(時間切れで未着手)。ブラウザでの実行(§8 は設計だけ)。プラグインを『見つからない』とした探索の先(サンプルの復号〔oggio / wavpackdll〕には届いていない)。

## 1. 先に結論 — エンジンが FLP を開いて何まで進んだか

| 項目 | 到達(実測) | 次の壁 |
|---|---|---|
| File → Open... (Sampler.flp、内蔵のサンプラー 1 チャンネル + `SHP_Sawing Synth.wav`) | **開いた**。ダイアログの COM 呼び出し(§3)→ OnFileOk ×2 → FLP の読み込み(`f_11d58c0` → `f_f54760` → `f_f51fd0` → `f_f46490`)→ サンプルの読み込み → 主窓のタイトル `Sampler.flp - FL Studio 2026`、Sampler 窓に波形。exit 0、idle に戻る(仮想時計 3,632,583 ms)。決定 0144 の入口 B(WM_COPYDATA)と同じ読み込みの経路を、今度はエンジン自身のダイアログから | — |
| File → Open... (参照プロジェクトの FLP、`serina1-serum2.flp` 1,322,619 B、70 チャンネル) | FLP のヘッダ・イベントを読み、**チャンネル 0 の Sampler(内蔵)を作り、チャンネル 1「Serum」の `Fruity Wrapper` の DLL を探す所まで**。2 件の `CompareStringW`(§5)を越えた後 | **Fruity Wrapper**(`Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll`、5,998,080 B、Delphi、未 lift・VFS に無い)。その次は Serum 2 の VST3(G11) |
| File → Open... (参照プロジェクトの .zip) | エンジンが `.zip` を判定し、一時フォルダ(`C:\Windows\Image-Line\4096-{GUID}\Zip\`)を作り、zip ファイルを開いて情報を読み(壁 Z1〜Z3 を越えた)、`DelZip64.dll` を読み込もうとする | **`C:\Program Files\Image-Line\FL Studio 21\Shared\DelZip64.dll`**(362,808 B、MSVC、export `DZ_Exec` ほか、未 lift) |
| 探索(プラグイン DLL = 見つからない、ACM ドライバ = 無し。結果ではなく仮定による探索) | 3 つの Fruity Wrapper チャンネルを飛ばして最初の音声クリップ(`[fadr.com] Vocals - ... .mp3`)を元の絶対パス(無い)→ プロジェクトのフォルダの同名ファイル(ある)の順で探して開き、**ACM(`msacm32`)で MP3 を復号しようとし、ACM が失敗すると `ilvideo_x64.dll`(FFmpeg)に回る** | **`ffmpeg\\x64\\ilvideo_x64.dll`**(未 lift)。§12(b) |

## 2. 方法

- **ビルド**: `tools/cycle.sh N`(決定 0171 の `cycle.sh` の流儀): `gen_two.sh gen split merge` で FLEngine と 5 DLL を `--extra-imports harness/seeds/dlg1-imports.txt` で lift し(FLEngine 約 2 分、6 モジュール計 数分)、本体が変わった単位(`fle-NN`、ILTools、`common.c`。5 単位)だけ `clang -O0` でコンパイルし、runtime とハーネスは本木のものをコンパイルして、leak1 の native オブジェクト(同じ生成 C。`keep/` にハードリンク)とリンクした。行の追加は応答表を変えるので、**行が変わるたびに再生成**した(9 回の cycle)。行を変えない修正(runtime だけ)は再リンクだけ。
- **行(opt-in)**: `harness/seeds/dlg1-imports.txt` = 決定 0171 の `ia1-imports.txt`(層 1+2。**変更せず先頭**。擬似 VA が動かない)+ 本決定の行(`shell32.dll SHCreateItemFromParsingName`/`SHGetKnownFolderPath`、vtable のスロット 68 行〔`ShellObj::*` 3、`IFileOpenDialog::*` 26、`IShellItem::*` 5、`IShellItemArray::*` 7、`IFileDialogCustomize::*` 27〕〔決定 0098 の `IMalloc::*` と同じ『輸出ではなくスロット』の行〕、`mpr.dll WNetGetUniversalNameW`、`kernel32.dll GetFileInformationByHandle`/`SetEndOfFile`)。**行の無い構成(既定の生成 C)は `fle_filedlg_cocreate` が応答表に行が無いことを見て断り**、`CoCreateInstance` は従来どおり `REGDB_E_CLASSNOTREG` を返す。`dlg1-explore-imports.txt` は上に ACM の探索行(`acmDriverEnum`・`acmStreamOpen`・`acmFormatSuggest`・`acmDriverClose`)を足した別ファイルで、**探索専用**(§12)。
- **選択の与え方(模擬)**: `FLE_FILEDLG_PICK_FILE`(1 行 1 ゲストパスのホストのファイル)または `FLE_FILEDLG_PICK`。スクリプトの `setenv` は値が 63 文字までなので短いパスのファイルにした。何も無ければ『利用者がキャンセルした』(`HRESULT_FROM_WIN32(ERROR_CANCELLED)` = 0x800704C7)。
- **VFS の種**: `tools/mk-vfs-seed.py`。参照プロジェクトの zip を**読み取りだけ**で `lift/out-dlg1/proj/` に展開し(元の `~/ダウンロード/serina1-serum2.zip` は変更していない。.zip 経路の実験のため、同じ zip の**ハードリンク**を `lift/out-dlg1/proj-serina1-serum2.zip` に作り、終了時に外した)、`C:\Users\user\Downloads\serina1-serum2\` に FLP と音声 48 ファイル、`C:\Users\user\Downloads\serina1-serum2.zip` に zip そのものを置く種の一覧(`tables/vfs-seed-dlg1-all.tsv`)を作った。日本語名の 2 ファイルは §7 の拡張で種付けできる。

## 3. エンジンの呼び出し列(File → Open...、実測。`FLE_FILEDLG_TRACE=1`)

```
CoCreateInstance clsid={DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7} iid={D57C7288-D4AD-4768-BE02-9D969532D960} unkouter=0 clsctx=0x1 (CLSID_FileOpenDialog)
SHCreateItemFromParsingName "C:\Users\user\Documents\Image-Line\FL Studio\Projects"      -> SetFolder
SetFileTypes[0] "FL Studio song file (*.flp)" *.flp
SetFileTypes[1] "Zipped song file (*.zip)" *.zip
SetFileTypes[2] "MIDI file (*.mid;*.midi)"   [3] "FL Studio Mobile song file (*.flm)"   [4] "BeatCreator groove file (*.zgr)"
SetFileTypes[5] "Supported song files (*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr)"
SetFileTypeIndex 6      SetOptions 0x20001000 (FOS_DEFAULTNOMINIMODE | FOS_FILEMUSTEXIST)
QueryInterface {E6FDD21A-163F-4975-9C8C-A69F1BA37034}  (IFileDialogCustomize)
  AddMenu(1900, "Favorites")     AddControlItem(1900, 1901, "Add this folder to favorites")
Advise ×2 (cookies 1, 2)
Show(owner = 主窓 0x7f990038)     [ここでハーネスが選択を与える]
OnFileOk ×2 -> 0x80004001 (E_NOTIMPL)      Unadvise ×2
SHGetKnownFolderPath {FDD39AD0-238F-46AF-ADB4-6C85480369C7} (Documents) ×2
```

(第 1 報の探索モードでは IFileDialogCustomize に E_NOINTERFACE を返してもエンジンは先へ進んだ。それは `AddMenu` の呼び出しを隠していたので、副オブジェクトとして本実装した。) 選択がハーネスから与えられると、`f_f1f020` が `PostMessageW(main, 0x566)` → `f_11d14a0` → `f_11d58c0`(拡張子で分岐)へ進む(決定 0144 の経路。`ed5d40` からの shadow stack は `logs/` の `.out`)。

## 4. COM モデルの仕様(`lift/runtime/fle_filedlg.c`)

- **オブジェクト**: ホストの calloc ブロック。先頭 8 バイトが静的な vtable(応答表の VA の配列)。`ShellObj::QueryInterface/AddRef/Release` の 3 行を 4 種類のオブジェクト(ダイアログ、IShellItem、IShellItemArray、IFileDialogCustomize の副オブジェクト)で共有する。副オブジェクトは多重継承の実機と同じく別の `this` と vtable を持ち、同じポインタを毎回返し、参照はダイアログが数える。
- **vtable の順序**(Learn はメソッドをアルファベット順にしか載せない。出典は SDK のインターフェース定義 `shobjidl.idl`〔Wine の写し。読んだだけで実行していない〕と、mingw-w64 の `shobjidl.h` の `*Vtbl` 構造体〔4 つの順序をこちらで独立に確認: IFileOpenDialog 29 スロット、IShellItem 8、IShellItemArray 10、IFileDialogCustomize 30。`tables/vtbl-order-check.txt`〕。GUID と値は mingw-w64 の `shobjidl.h`・`knownfolders.h`): IFileOpenDialog = IUnknown 0–2、`Show` 3、IFileDialog 4–26(SetFileTypes, SetFileTypeIndex, GetFileTypeIndex, Advise, Unadvise, SetOptions, GetOptions, SetDefaultFolder, SetFolder, GetFolder, GetCurrentSelection, SetFileName, GetFileName, SetTitle, SetOkButtonLabel, SetFileNameLabel, GetResult, AddPlace, SetDefaultExtension, Close, SetClientGuid, ClearClientData, SetFilter)、GetResults 27、GetSelectedItems 28。`tests/wall_dlg1.rs` が順序を、SDK の順序とは別に書いた表で確かめる。
- **Learn の根拠**: IModalWindow::Show(S_OK / `HRESULT_FROM_WIN32(ERROR_CANCELLED)`)、IFileDialog の各メソッド(SetFileTypes: 2 回目と FOS_PICKFOLDERS は E_UNEXPECTED、NULL は E_INVALIDARG。インデックスは 1 始まり。GetResult/GetResults は Show が成功した後と OnFileOk の間だけ、複数選択の GetResult は失敗。Advise のクッキー)、FILEOPENDIALOGOPTIONS(FOS_PATHMUSTEXIST / FOS_FILEMUSTEXIST が既定)、IFileDialogEvents::OnFileOk(S_OK 受理、S_FALSE 拒否)、IShellItem::GetDisplayName/GetAttributes/Compare、IShellItemArray::GetItemAt(範囲外は E_FAIL)、SHCreateItemFromParsingName、SHGetKnownFolderPath(末尾の バックスラッシュ なし、CoTaskMemFree)、SIGDN、IFileDialogCustomize の各メソッド(既定の状態は『有効・表示』)、WNetGetUniversalNameW(ERROR_NOT_CONNECTED)。**ページが空欄の選択は S-dlg1-…**(§9)。
- **Show()**: 設定(オプション、フィルター、タイトル、フォルダ、カスタマイズのコントロール)を `FLE_FILEDLG_TRACE` で印字し、ピッカー(`fle_filedlg_set_picker` で差し替え。既定はハーネスの環境変数)に渡す。選択が無ければキャンセル。FOS_FILEMUSTEXIST で VFS に無いパスは**名前つき停止**(実機はダイアログが閉じない = ハーネスの誤り)。選択後に結果を保持し、登録されたイベント受けの `OnFileOk(pfd)`(vtable スロット 3)を `fle_call_guest` で呼ぶ。
- **CoCreateInstance への接続**: `fle_ole.c` の `fle_imp_CoCreateInstance` に弱い参照の呼び出しを 1 か所足しただけ(`fle_filedlg.c` をリンクしない既存テストは無変更)。

## 5. 壁の台帳(`tables/walls.tsv`、17 行。呼び元は shadow stack の実測、意味は読み)

| id | 経路 | 停止(実測) | 呼び元と読み | 必要な物 | 最小の実装 | 状態 |
|---|---|---|---|---|---|---|
| D1 | Open | `CoCreateInstance` が REGDB_E_CLASSNOTREG | エンジンの TFileOpenDialog(Delphi の `TCustomFileDialog.Execute` と読む)、`f_f1f020` | IFileOpenDialog | §4 | 修正(opt-in の行) |
| D2 | Open | `QueryInterface(IFileDialogCustomize)` | 直後に `AddMenu(1900,"Favorites")` + `AddControlItem(1900,1901,...)` | 副オブジェクト 27 メソッド | 状態を持つ実装 | 修正 |
| D3 | Open | OnFileOk のハンドラが E_NOTIMPL を返す(×2) | エンジンの TFileDialogEvents | — | E_NOTIMPL = 未処理として続行(S-dlg1-onfileok-notimpl。実機の挙動の推測) | 修正(選択) |
| D4 | Open(Sampler.flp) | 停止なし | `f_11d14a0` → `f_11d58c0` → `f_f54760` → `f_f51fd0` → `f_f46490` | — | — | **開いた** |
| F1 | 参照 FLP | `CompareStringW(0x409, NORM_IGNORECASE, "\Backup\", "-serum2\")` | `f_44ec40`(SameText 相当。`sub 2`)← `f_44f040` ← `f_f45ae0`/`f_f45b70` ← `f_f46490`。結果は `test eax,eax; sete`/`test al,al; je`: **等号だけ** | ハイフン対記号の順序 | §6 | 修正(opt-in) |
| F2 | 参照 FLP | `CompareStringW(0x800, SORT_DIGITSASNUMBERS, "オートメーション", "UNSORTED")` | `f_103ba10`(`f_f4db1f` から)。`sub 2; test; setl` の探索ループ = **符号を使う**(§6) | かな対 ASCII の順序 | §6 | 修正(opt-in、出典なし) |
| F3 | 参照 FLP | チャンネル 1 の `Fruity Wrapper` の DLL が無く `LoadLibraryW("")` で停止 | `f_44db40` ← `f_132c680` ← `f_132ee00` ← `f_132f3f0` ← `f_12ba440` ← `f_12bd600` ← `f_f46490` | Fruity Wrapper_x64.dll(未 lift)、Serum2.vst3 | G9/G11 | **確定した次の壁** |
| E1〜E5 | 探索 | `acmDriverEnum` → `acmStreamOpen` → `acmFormatSuggest` → `acmDriverClose`(各 1 壁。ACM が全部失敗すると **`ilvideo_x64.dll` の読み込み(E5)**に回る) | `f_a5dc40`・`f_6729d0`・`f_673f40`・`f_a5f380` ← `f_a5ee30`(.mp3 のサンプル読み込み)← `f_a65690` ← `f_fee6d0` ← `f_fff070` ← `f_f3b150` ← `f_f3b8b0` ← `f_f46490` | §12 | 探索の行 | 探索 |
| Z1 | .zip | `mpr!WNetGetUniversalNameW` に行なし(0x423dd0) | エンジンの zip の枝(`f_11d58c0`) | ローカルドライブに ERROR_NOT_CONNECTED | `fle_wall_dlg1.c` + 行 | 修正 |
| Z2 | .zip | `ReadFile(0x52c)`: 『ハンドルは VFS が発行した物でない』 | zip の読み手が `CreateFileW` のハンドル(0x7ffa0000052c)を 32 ビットの欄に入れて下位半分を渡す | Learn『Interprocess communication』: 64 ビット Windows のハンドルは下位 32 ビットだけが有効 | `fle_vfs.c`: 生きたファイルハンドルの下位半分を受ける(S-dlg1-handle32。他の値は従来どおり停止) | 修正 |
| Z3 | .zip | `GetFileInformationByHandle` に行なし(0x423510) | `f_99a380` ← `f_9ab1b0` ← … ← `f_9c7f20` ← `f_11d58c0` | BY_HANDLE_FILE_INFORMATION | `fle_wall_dlg1.c` + `fle_vfs_file_stat()` + 行。`SetEndOfFile` も同じ形 | 修正(選択 S-dlg1-fileinfo) |
| Z4 | .zip | `LoadLibraryW("C:\Program Files\Image-Line\FL Studio 21\Shared\DelZip64.dll")` | `f_9b58a0` ← `f_9b55a0` ← `f_9b4f50` ← … ← `f_9c7f20` | DelZip64.dll(362,808 B、MSVC 11.0、export `DZ_Exec`・`DZ_Banner` ほか 7、import は kernel32/user32/shell32、.pdata 784 関数、未 lift) | DelZip64.dll を lift する、または**ページが zip を展開して .flp を渡す** | **確定した次の壁(.zip 経路)** |
| V1 | 種付け | vfs_seed.h が ASCII だけ(日本語名の音声 2 ファイル) | ハーネス | UTF-8 の名前 | §7 | 修正 |

## 6. CompareStringW の出典・範囲・符号(PM の指示 1)

- **出典(取得、Learn)**: 『Handling Sorting in Your Applications』(word sort が既定。『ハイフンとアポストロフィを除く句読点と英数字以外は、どの英数字より前』。『ハイフンとアポストロフィは別扱い(coop と co-op が並ぶように)』。SORT_STRINGSORT は両者を普通の記号として英数字の前に置く。**word sort の結果の表**: billet, bills, bill's, cannot, cant, can't, con, coop, co-op。string sort: bill's, billet, bills, can't, cannot, cant, co-op, con, coop)、CompareStringEx(NORM_IGNORECASE ほか)、同ページ(SORT_DIGITSASNUMBERS)。Wine は出典にしない(決定 0042)。
- **等号が実機と一致する範囲(読み)**: 『CompareString は**言語的な等しさ**を調べる』(同ページ)。NORM_IGNORECASE は大文字小文字(第 3 次差)を無視する。ASCII 同士で NORM_IGNORECASE 付きの『大文字小文字だけが違う』は等しい(CSTR_EQUAL)、それ以外で文字が違えば不等(`coop` と `co-op` も不等: 表で並びが違う)。これは既存の `fle_locale.c` と本決定の拡張のどちらでも同じ。**等号が実機と食い違い得るのは、重みの無いコード点(ZWJ など)と正規化の等価(à と a+◌̀)を含む文字列**(同ページ)で、本モデルは ASCII とかなだけを扱う。
- **符号を使う箇所(実測の命令列)**: F1(`f_44ec40` → `f_44f040` → `f_f45ae0`)は**等号だけ**。F2(`f_103ba10`)は**符号を使う**: `f_f46490` の中の FLP のイベント処理から、チャンネルフィルターの名前を**並べ替えた一覧に挿入する二分探索**(`CompareStringW(LOCALE_SYSTEM_DEFAULT 0x800, SORT_DIGITSASNUMBERS, 挿入する名前, 一覧の名前) - 2 < 0` の繰り返し)と読む。**効く画面 = チャンネルラックの『フィルター』タブ(チャンネルフィルター)の並び順**。参照プロジェクトのフィルター名『オートメーション』が既存の『UNSORTED』などより前か後かが、本モデル(かなは ASCII より後)と実機で違い得る。それ以外の動作(チャンネルの割り当て等)には効かないと読むが、確かめていない。
- **実装(`fle_locale.c`、opt-in = 環境変数 `FLE_CMP_EXTEND`、既定の動作は無変更)**: (1) S-dlg1-cmp-hyphen-wordsort: 最初の差の位置でどちらかがハイフン/アポストロフィのとき、その位置以降を『それらを除いて』同じ関数で比べ直し、等しければハイフン等の多い方を大きいとする(表の `bills < bill's`、`cant < can't`、`coop < co-op`、`con < co-op`、`can't > cannot` をテストで確認)。F1 の `\Backup\` 対 `-serum2\` は `\`(記号)対 `s`(文字)で記号が前になり CSTR_LESS_THAN。(2) S-dlg1-cmp-nonascii(**出典なし**): U+007F より上の単位は ASCII の単位より大きい、同士はコード点順(かな対 ASCII の既存の停止を越えるため)。同数のハイフンで位置が違うだけの文字列(`a-b` と `ab-`)は名前つき停止のまま。

## 7. 日本語名のファイルの種付け(PM の指示 2)

`harness/vfs_seed.h` の種の一覧を UTF-8 に拡張した: ゲストのパスは UTF-8 で、検証した上で UTF-16 に変換して `CreateFileW` に渡す(U+10000 以上は 4 バイト = 2 単位)。長さの上限(259)は**単位数**、コンポーネントの上限(255)も単位数。無効な UTF-8(孤立した継続バイト、過長形、サロゲート)は『is not valid UTF-8』で名前つき拒否。**ASCII の挙動はバイト同一**(制御文字・0x7f・`< > : " / | ? *` は従来の文言で拒否、259 超は従来の文言): 既存の `vfs_seed_selftest` の出力 60 行が、HEAD の素と本木で完全に一致(`logs/vfs-seed-selftest-*.txt`)。`wall_dlg1.rs` に UTF-8 の種付け(かな + 絵文字の名前を作って UTF-16 の名前で見つかる)、無効 UTF-8 の 3 種、長さの境界(259 単位は受理、260 は旧文言で拒否)、ASCII の拒否文が不変であることのテストを足した。生成 C に影響しない(ハーネスのヘッダだけ)。runtime の `fle_vfs.c` は UTF-16 の名前を元から扱える(大文字小文字の畳み込みは ASCII だけ)。

## 8. ブラウザ接続の設計(下書き。何も実行していない)

- **JSPI で Show() を一時停止**: `fle_filedlg_host_pick(request_json)`(`lift/harness/web/filedlg/fle_filedlg_lib.js`: `fle_idle_lib.js` と同じ形の async import〔WebAssembly.Suspending〕)が Promise を返し、Worker の `Module.fleFileDlg(...)` がページへ `filedlg-request`(オプション、フィルター 6 つ、フォルダ、カスタマイズのコントロール〔Favorites メニュー〕)を post し、ページの返事で `Module.fleFileDlgDeliver(ゲストパスの配列 | null)` が Promise を解く。C 側は `fle_filedlg_web.c`(`fle_filedlg_set_picker` に登録するピッカー)。`emcc -c` で `fle_filedlg.c`・`fle_ole.c`・`fle_filedlg_web.c` がコンパイルできることだけ確かめた。
- **ユーザー操作の要件**: `showOpenFilePicker()` は Window のメソッドで Worker には無い(MDN による読み)ので、ページが呼ぶ。**一時的なユーザー有効化**が要る: File → Open... を選んだクリック(またはキー)が Worker に転送されてから `Show()` に届くまで(ネイティブの実測で仮想時計の数百 ms 以内)に、ページは有効化が残っている間に呼ぶ必要がある。有効化が無い(例: 遅延)ときは SecurityError なので、ページに『ファイルを選ぶ』ボタンを出し、そのクリックで呼ぶ(設計)。
- **仮想ボリュームへの取り込み**: 選んだ `File` を `arrayBuffer()` で読み、Worker へ転送(参照プロジェクトの zip は 53 MB。ページのメモリ上限・転送は未測定)、VFS に `C:\Users\user\Downloads\<名前>` として書く(`vfs_seed.h` と同じくゲスト向けの `CreateFileW`/`WriteFile` のスタブを呼ぶ形。取り込み関数は未実装)。`Show()` はそのゲストパスを返す。**.zip**: エンジンの展開は `DelZip64.dll`(未 lift)に頼るので、**ページ側で展開して(`DecompressionStream` や fflate)FLP と音声を同じフォルダに置き、FLP のパスを返す**のが現実的(読み。日本語名の音声があるので §7 の UTF-16 名が要る)。サンプルは FLP に保存された元の絶対パスに無ければ、**FLP と同じフォルダの同名ファイル**を探す(実測: 元の `C:\Users\cabbage\AppData\Local\Temp\Image-Line\…\xxx.mp3` が無く、`C:\Users\user\Downloads\serina1-serum2\xxx.mp3` で見つかった)ので、フォルダごと(または zip)が要る。単独の `.flp` を選ぶと音声が見つからない。
- **メモリ**: 参照プロジェクトは音声 62.7 MB。エンジンの wasm メモリ 2 GiB の上限(決定 0170)の中で、zip(53 MB)+ 展開した音声 + エンジンが読む音声が同時に載る。未測定。

## 9. 出典なしの選択(`tables/s-choices.tsv` に代替案と撤回条件。25 件)

S-dlg1-pick-env、-qi、-opt-default、-fti、-advise-ref、-onfileok-refuse、-fail-code、-getfilename、-sigdn、-sfgao、-parse-notfound(`SHCreateItemFromParsingName` の存在しないパスは 0x80070002 — **実機の挙動の推測**)、-kf、-clsid-open-only、-optin、-vfs-place、-custom、-custom-sel、-onfileok-notimpl(**実機の挙動の推測**)、-wnet-value(2250 は winerror.h)、-cmp-hyphen-wordsort(Learn の表の読み)、-cmp-nonascii(**出典なし**)、-loadlib-missing(探索専用・推測)、-handle32(Learn 出典あり。生きたファイルハンドルに限る)、-fileinfo、-acm-none(探索専用)。

## 10. 検証(実測。load 7〜25)

1. **単体テスト**: `lift/crates/fe-lift/tests/wall_dlg1.rs` **28 件**。vtable の順序、CoCreateInstance(行なしは従来の REGDB_E_CLASSNOTREG、集約は CLASS_E_NOAGGREGATION)、参照カウント、オプション・フィルター・インデックスのエラー表、Show のキャンセル/選択/複数/FILEMUSTEXIST/PICKFOLDERS、OnFileOk(S_OK・S_FALSE・E_NOTIMPL・その他は停止)、IFileDialogCustomize(副オブジェクトの順序・同一性・状態・停止)、環境のピッカー(`FLE_FILEDLG_PICK` と一覧ファイル)、SHCreateItemFromParsingName/SHGetKnownFolderPath、WNetGetUniversalNameW、UTF-8 の種付け、32 ビットのハンドル、GetFileInformationByHandle/SetEndOfFile、CompareStringW の opt-in(既定は従来の停止)、探索の ACM、行の一覧。期待値は Learn の文と SDK の順序から手で書いた。
2. **全体テスト**(`bash lift/harness/test.sh full --jobs 3 --fresh`、`nice`、専用の target、`FLE_INPUT_ROOT` = 本木): HEAD の素(7757f60 の `git archive`。912a2ca との差は docs だけ): **results=161 passed=2164 failed=0**(`logs/fulltest-base.txt`)。本差分を HEAD 912a2ca の木に当てた物: **results=162 passed=2192 failed=0**(全 2,192 テストを実際に実行、cached=0、壁時計 300 s。`logs/fulltest-final.txt`)= **+1 実行ファイル(`wall_dlg1`)・+28 テスト**で新規テストの数とちょうど一致。SKIPPED の行は 15 で素と同じ(git archive の木には製品の入力ファイルが無いため。ia1 の注意に従い symlink で本木の `lift/out-*` を通さなかった)。PM の基準 157 results / 2,157 passed / SKIPPED 1 は b4f1d3c の時点で入力のある本木での値なので、数は直接は比べられない(同じ条件の素との差が正しい比較)。
3. **FLEngine 既定の生成 C**(HEAD の `fe-lift lift FLEngine_x64.dll X --pdata`): sha256 **cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50**、215,811,678 B(`logs/gen-default.sha256`)= 不変(`lift/crates` は無変更)。
4. **a4 FRUITY 無効**(`a4-fruity.sh off`、本差分適用後の HEAD 912a2ca の木): `init_count=942/942`、`run_rank=895/895`、`functions_distinct=5310`、`function_entries=1009731` = **基準と一致**(`logs/a4off-final.txt`。gen 14 s・build は JOBS=1。最初の試行は build 中に外から Terminated されたので〔メモリ圧と読む〕、並列を減らして取り直した: `logs/a4off-final-attempt1-terminated.txt`)。
5. **16 スナップ(`cmp-short.g3`、native -O0)**: (a) **行なしの通常構成 + 本木の runtime**(leak1 の同じ生成 C のオブジェクト + 本木の runtime をリンク): `g3-snap` 16・`g3-surface` 34・`g3-wintable` 16・`g3-progress` 16 = **82 行が `docs/investigations/ia1/tables/cmp-short-baseline-82.txt` と完全に一致**(`logs/cmpA-diff.txt` は空)。(b) **ia1 の行(層 1+2)のみ + 本木の runtime**: 82 行が ia1 の `cmp-short-rows-baseline.txt` と**完全に一致**(`tables/cmp-short-82-ia1-rows-current-runtime.txt`)。(c) **本決定の行(`dlg1-imports.txt`)を足すと 82 行のうち 35 行が変わる**(`tables/cmp-short-82-dlg1-rows-build.txt`。wiz0 で clock_ms 3601776〔ia1 の行は 3601774〕、entries 56,446,089〔56,446,126〕、面 lid 8 の非黒 52,035〔52,040〕、その SHA a5c2d803…〔650569af…〕。つまり決定 0171 の層 2 と同じ種類の『起動の基準が変わる行』)。**原因を絞った(bisect、実測)**: ia1 の行 + `shell32.dll SHCreateItemFromParsingName` と `shell32.dll SHGetKnownFolderPath` の 2 行だけを足した構成(`tables/dlg1-bisect1-imports.txt`)の 82 行が、本決定の全行を足した構成と**完全に同一**。vtable の行・`mpr`・`GetFileInformationByHandle`・`SetEndOfFile` の行は起動に効かない。この 2 行のうちどちらか(または両方)が起動中に `GetProcAddress` で引かれ(FLEngine の文字列表に隣り合って 2 名とも有る: 0x11eb506、0x11eb51e)、行があるとゲストが『あり』の経路に入ると読む(層 2 の `DwmIsCompositionEnabled` と同じ仕組み。2 行の分離は未実施)。実機は Vista 以降なら両方あるので『あり』のほうが実機に近い、と読むが、実機との比較は 0 件。**= PM の判断点: 行を採用すると起動の基準表が変わる**(新しい基準として本決定の 82 行を別名で出した。既存の表は上書きしない)。
6. **`dlg1.diff` の『-』行 14 本を全部読んだ**: `vfs_seed.h` の意図した置換 13 本(UTF-8 化。`VS_GUEST_MAX` の注釈、`guest` の寸法、長さ・コンポーネント長・文字の検査、`vs_wide`)と、`fle_load.c` の 1 本(`if (n == 0) fle_stop(...)` を、同じ停止を残したまま opt-in の探索分岐を前に置く形に直した)。他の既存ファイル(`fle_ole.c`、`fle_vfs.c/.h`、`fle_locale.c`)は追加だけ。他の決定の追加の巻き戻しは無い。

## 11. 未着手・未確認・PM の判断点

- **判断点**: (1) 行の採用: `dlg1-imports.txt` を最終バンドルの `EXTRA_IMPORTS` に使うか(ia1 の層 1+2 を含む。層 2 の判断は決定 0171 のまま)。**採用すると起動の基準が変わる**(§10 項目 5: `SHCreateItemFromParsingName`/`SHGetKnownFolderPath` の 2 行が起動中に引かれる。82 行のうち 35 行が変わる。新しい基準 `tables/cmp-short-82-dlg1-rows-build.txt`)。ファイルダイアログだけが目的で起動の基準を変えたくない場合の代替は、この 2 行を外して `SHCreateItemFromParsingName` を別の形で応えること(未実施。エンジンが File → Open... で呼ぶので外すとダイアログが成立しない)。(2) `fle_locale.c`・`fle_load.c`・`fle_vfs.c` の変更: `FLE_CMP_EXTEND`(CompareStringW、既定は従来の停止)と `FLE_LOADLIB_MISSING`(探索専用)は環境変数による opt-in。**`fle_load.c` の空名の停止 1 行は、判断点として: 既定は同じ停止のまま、環境変数がある時だけ『見つからない』を返す形にした**。`fle_vfs.c` の 32 ビットハンドルの受け入れは opt-in にしていない(従来は停止だった値だけが対象。Learn の根拠あり)。(3) かな対 ASCII の順序(S-dlg1-cmp-nonascii)は出典なし(チャンネルフィルターの並びに効く)。(4) 次の担当への仕様: §12。
- **未確認**: 実機との比較。OnFileOk の E_NOTIMPL の扱い、SHCreateItemFromParsingName の不存在パス、ほか S- の全部。File → Save as の往復。ブラウザでの実行。探索の先(サンプルの復号)。

## 12. 次の壁の整理(次の担当への仕様)

### (a) Fruity Wrapper(F3。G9/G11 の壁。次の担当 wrp1 へ。静的な調査で、lift はしていない)

- **位置**: インストール内 `Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll`(5,998,080 B、sha256 `372bbc84e85f427d2419d04d548a9b763b96b85975b056f31d807fe480dba472`)と `Plugins\Fruity\Effects\Fruity Wrapper\Fruity Wrapper_x64.dll`(5,997,056 B、`31f012f6d741…`)。抽出済みの実ファイル: `lift/out-plug1/dll/Plugins/Fruity/{Generators,Effects}/Fruity Wrapper/Fruity Wrapper_x64.dll`(git 外)。`docs/investigations/m2-inventory/pe-inventory.tsv` に行がある。
- **形式(実測、pefile)**: PE32+ x64 DLL、**Delphi**(リンカ 8.0)、ImageBase 0x400000(FLEngine と同じ。同時にロードするには `fe-pe rebase` が要る)、`.text` 4.8 MB、`.reloc` 250 KB あり、`.pdata` 19,826 関数、export 3 つ: `CreatePlugInstance`・`__dbk_fcall_wrapper`・`dbkFCallWrapperAddr`(FLEngine の Delphi 流儀と同じ形)。
- **import(実測)**: 16 DLL・584 個: user32 208、kernel32 150、gdi32 113、comctl32 30、oleaut32 18、ole32 18、shell32 16、advapi32 11、winspool.drv 5、winmm 4、comdlg32 2、version 3、imagehlp 3、shfolder 1、msacm32 1、api-ms-win-crt-string 1。遅延ロード 12 DLL: kernel32 7、user32 3、wtsapi32 2、msimg32 1、advapi32 1、rpcrt4 1、windowscodecs 1、shell32 2、uxtheme 5、imm32 7、dwmapi 2、shcore 2。
- **エンジンの探し方(実測、`runs/x9`)**: プラグイン名はイベント 201(内部名 `Fruity Wrapper`)、表示名はイベント 203(`Serum`、`Serum #2`、`Serum 2`、…。FLP の最初のプラグイン生成イベントはチャンネル 0 の Sampler〔内部名が空〕、2 番目がチャンネル 1)。`...\Fruity Wrapper\Fruityplug_x64.dll`(存在しない名前)→ `...\Fruity Wrapper_x64.dll` を `GetFileAttributesW`、`Documents\Image-Line\FL Studio\Presets\Plugin database\Installed\Fruity Wrapper.NFO` と `.Plugins.ini` を調べ、`LoadLibraryW("")` に至る。
- **lift の見込み(読み)**: FLEngine と同じ Delphi 製で、同じ lifter の流儀。大きさは FLEngine(22 MB)の約 1/4。動かすには、`CreatePlugInstance` へ渡すエンジン側のホストオブジェクト(決定 0116 の fake host の延長)と、Fruity Wrapper が VST3 を読む経路(`LoadLibraryW(Serum2.vst3)`。VST3 の lift は決定 0156・0177 に実績)が要る。参照プロジェクトは VST/VST3 のプラグインが 21 インスタンス(Serum 2 ×6 ほか。`reference-project.md`)で、いずれも Fruity Wrapper が入れ物になる。

### (b) MP3 の ACM 経路(E1〜E5。サンプルの復号。探索で読んだ)

- **エンジンが ACM に入る条件**: 実測 = 最初の音声クリップ(`.mp3`)で入った。`.MP3` の UTF-16 定数がサンプル読み込みの近傍にある(0xa65208、0xa6540c、0xa6b6dc。`f_a65690` の周辺)が、**拡張子だけで決めているか、内容(ヘッダ)も見るかは未確認**。WAV の PCM は ACM に入らない見込み(Sampler.flp の WAV は入らず読めた。実測)。
- **呼び出し順(実測)**: `acmDriverEnum`(0 件を返す)→ `acmStreamOpen`(ACMERR_NOTPOSSIBLE)→ `acmFormatSuggest`(同)→ `acmDriverClose`(MMSYSERR_INVALHANDLE)。
- **ACM が使えないときの動き(実測)**: **ファイルを諦めるのではなく、別の復号器に回る** = `C:\Program Files\Image-Line\FL Studio 21\Shared\ffmpeg\x64\ilvideo_x64.dll`、`...\FL Studio 21\ffmpeg\x64\ilvideo_x64.dll` を探し(無い)、相対名 `ffmpeg\x64\ilvideo_x64.dll` を `LoadLibraryW`(名前つき停止)。= **未 lift の同梱 DLL `ILVideo_x64.dll`(Delphi、1,329,152 B、26 export、`ILVideo_GetAudioDuration`・`ILVideo_ListCodecs` ほか)と FFmpeg 一式**(`Shared\ffmpeg\x64\`: avcodec-58 47 MB、avformat-58 12 MB、avfilter-7 11.5 MB、avdevice-58 1.9 MB、avutil-56 0.86 MB、swresample-3 0.6 MB、swscale-5 0.57 MB。GCC/MinGW 2.38。計約 75 MB。`m2-inventory/pe-inventory.tsv`)。`bundled-dll-inventory.md` の『パス付きの名前』表 7 番の予測と一致。
- **実 Windows の MP3 ACM コーデック(`system32\l3codecp.acm`)の有無**: `ls ~/ダウンロード` に無い。`find ~ -iname 'l3codec*'` と `find /` で見つかったのは `lift/out-ctype/wineprefix/drive_c/windows/{system32,syswow64}/l3codeca.acm`(1,104,248 B)・`l3codecx.ax` だけで、**Wine 自身の組み込み実装**(`strings` に "Wine builtin DLL"・"Wine MPEG3 decoder"。Microsoft の物ではない。決定 0042 により出典にしない)。= Microsoft の `l3codecp.acm` は依頼者の Windows の system32 にあるはずで、**『依頼者側の供給』候補**。
- **代用の可否は PM の判断**: 実機は ACM が先で ilvideo(FFmpeg)は後なので、実機と同じ復号ビット列を得るには Microsoft のコーデックが要る。FFmpeg の MP3 復号と同一かは未確認。代用は進捗に数えない(探索の仮定として labelled)。

### (c) DelZip64.dll(Z4。.zip を直接選ぶ経路の壁)

- **位置**: `C:\Program Files\Image-Line\FL Studio 21\Shared\DelZip64.dll`(エンジンが渡す絶対パス。実測)。362,808 B、sha256 `39c9763d9631e9749ff64fd72e599313ffbd644aefd23de7f5c62faa7c475fac`、x64 DLL、**MSVC 11.0**、`.pdata` 784 関数、import は kernel32・user32・shell32 の 3 DLL(112 個)、export 7(`DZ_Abort`・`DZ_Banner`・`DZ_Exec`・`DZ_Name` ほか)、『Freeware Zip/Unzip』(DelphiZip)(`m2-inventory/pe-inventory.tsv`)。抽出済みの実ファイルは手元に無い(インストーラの中。`g8-project-load.md` は『`.zip` の展開用』と予測、本決定の実測で確定)。
- **lift の見込み(読み)**: 小さい(ILTools_x64 級)、import が 3 DLL で少なく、MSVC 製で最も lift しやすい部類。VFS に置いて `fle_realmod` の実在モジュールとして載せる(決定 0082)。
- **代替**: ブラウザのページが zip を展開して FLP と音声を渡すなら(§8)、DelZip64.dll は要らない(zip 内の日本語名は §7 の UTF-16 名で扱える)。エンジンの unzip を実機と同じ経路にするなら lift する。

### 次の担当への優先(読み)

(1) ブラウザの取り込み(§8)— 参照プロジェクトを開く入口の実体。(2) Fruity Wrapper の lift + VST3 の取り込み — これが無いと FLP のチャンネル 1 から先が進まない(探索の結果、プラグインが無いまま先へ進むと MP3 で ilvideo に当たる)。(3) サンプルの復号: PCM の WAV は読める(Sampler.flp)。MP3 は ACM(依頼者の供給)または ilvideo + FFmpeg。Vorbis は oggio_x64、WavPack は wavpackdll_x64(いずれも未 lift)。

## 13. 自己申告

- 存在しない `bash` ツールを 1 回呼んだ(shell ツールを使うべきだった)。
- 同時プロセス: 一時的に 3〜5 本(エンジン 1〜2 + 生成/コンパイル 2 + 全体テスト 3 jobs)。load は 7〜37。
- `lift/out-dlg1/proj-serina1-serum2.zip` は `~/ダウンロード/serina1-serum2.zip` の**ハードリンク**(内容は変更していない。終了時に外した)。他の `lift/out-*`(leak1 の native オブジェクト、ia1 の docs)は読み取りだけ(オブジェクトはハードリンクで借りた)。`/tmp/opencode/dlg1/` にピッカーの一覧ファイル(スクリプトの `setenv` が 63 文字までのため)と取得した Learn のページを置いた。
- 途中の測定のうち、`FLE_CMP_SYMBOL_ORDINAL`(コード点順)の版は Learn の表を読んで word sort の規則に置き換えた(第 2 報の『コードポイント順の opt-in』は撤回)。

## 14. 再現

`tools/`(cycle.sh、run-script.sh、serve.sh、send.sh、follow-run.sh、build-rt.sh、build-h.sh、link-native.sh、mk-vfs-seed.py、va2func.py、dlg1_wraps.c、bgrx2png.py、fulltest.sh、mkdiff.sh)と `tables/*.g3`(スクリプト)、`tables/vfs-seed-dlg1-*.tsv`。`MERGE-NOTES.md` に当て方。

## PM の判断と取り込み(2026-11-03、追記)

取り込み: `dlg1.diff`(15 ファイル、基準 HEAD 912a2ca)。PM の検証: `patch -p1 --dry-run` は offset・fuzz・reject なしで当たる。`-` 行 14 本(第 3 報の「13 本」は担当の数え間違いで、最終報告で訂正)は `vfs_seed.h` の意図した置換 13 本と `fle_load.c` の空名の 1 行(`FLE_LOADLIB_MISSING` があるときだけ『見つからない』を返す探索専用の形)だけで、他の決定の変更の巻き戻しは無い。FLEngine の既定の生成 C の sha256 は `cd9107ce…`(215,811,678 B)で不変。全体テスト **162 results / 2,192 passed / 0 failed**(+1 実行ファイル、+28 テスト = 担当の値と一致)、a4(FRUITY 無効)942/942・895/895・functions 5,310・entries 1,009,731 で不変。PM は FLP を開く操作を再実行していない(数字は担当の実測)。進捗(G 段階)には数えない: ブラウザでの実行は未実施で、選択はハーネスが与えた。

判断点への回答:

1. **行の採用**: 出荷するバンドルには `dlg1-imports.txt` の行も入れる(File → Open... が成立しないと G8 に進めない。`SHCreateItemFromParsingName` を外すとダイアログが成立しない)。起動の基準は `tables/cmp-short-82-dlg1-rows-build.txt`(別名の新しい基準。既存の基準表は上書きしない)。層の構成は 層 1(ia1 の起動基準を変えない行)→ 層 2(ia1 の遅延ロードの行。DwmIsCompositionEnabled で基準が変わる)→ 層 3(dlg1 の行。SHCreateItemFromParsingName と SHGetKnownFolderPath の GetProcAddress で基準が変わる)。`EXTRA_IMPORTS` は 1 つのファイルなので、出荷用は 3 層を結合した 1 ファイルを作る(fin1 の最終バンドルの作業で行う)。2 行のどちらが起動を変えるかの分離は未実施。
2. **`fle_load.c` の空名の停止**: 既定は同じ停止のまま。探索専用で、採用は想定しない。
3. **`fle_vfs.c` の 32 ビットハンドルの受け入れ**: 従来は停止だった値だけが対象(生きたファイルハンドル)。根拠は Learn『Interprocess communication』。このまま。
4. **かな対 ASCII の順序**: 出典なし(S-dlg1-cmp-nonascii)のまま。チャンネルフィルターの並びに効く。実機の画面との照合が来るまで未確認。
5. **MP3**: ACM の先が `l3codecp.acm`(依頼者の Windows の物)で、リポジトリには無い。Wine の `l3codeca.acm` は出典にしない。代用の復号器でファイルを読ませる探索は可だが、**進捗には数えず、探索の仮定として labelled**。HLE の定義(結果ビット同一)で代用してよいかは、ACM の MP3 復号の仕様(出力の PCM の規則)を調べてから決める。依頼者側の供給候補: `C:\Windows\System32\l3codecp.acm` と `msacm32.dll` が使うドライバの登録情報。
6. **S- の選択 25 件**: `tables/s-choices.tsv`。実機の挙動の推測を含むので、全て仮。

次の担当(wrp1、決定 0183 予定): §12(a) の Fruity Wrapper(Delphi、5,998,080 B、import 584)を lift してエンジンに載せ、ラッパーが VST3 の `GetPluginFactory` を呼ぶところまで進める。
