# Issue #1521: File メニューに「Manage files」を足す見込み(読みの結果)

- 範囲: 読みと、DLL の DFM・RTTI の解読。製品(`app/`・`lift/`)と E2E は変えていない。実行はしていない(Manage files の項目は無い)。測定の値は無い。
- 区分: 「読み」は読んだ物と位置。「推定」は読みから推したもので、確かめていない。「確認できない」は、読みにも記録にも無いもの。
- 読みの元: `lift/out-app/host/binaries/win64/FLEngine_x64.dll`(FL の DLL。git に無い。ビルドの入力)。道具: `pm-work/findings/manage-files-1521-dfm-read.py`(調査専用)。

## 1. 依頼者の質問への答え(読みから)

- 項目を足せるか: **可能(この作業で実現。§8)。** 前回の読み(§4)は、項目を作る番地を探す方針だった。今回は DFM の資源を読み込み時に複写する方式に変えた。
- 押下の処理はメニューの ID で分岐しているか: **分岐していない。** File の各項目は、自分の `OnClick`(メソッドの名前)か、`Action`(TAction の `OnExecute` の名前)を持つ(§2)。ID で switch する形ではない。項目ごとに処理が結ばれる。
- 項目の並びは実行時に変えられるか: **推定で可能。** 項目は VCL の `TMenuItem` の木(子の列)。子の列の変え方(Insert に当たる関数)の番地は、まだ特定していない(§4)。

## 2. File メニューの構造(読み)

- 本体は `FruityLoopsMainForm`(TFruityLoopsMainForm。DFM のファイル内 20671368)。`MainMenu`(TQuickPopupMenu)→ `FileMenu`(TQuickMenuItem、Caption `&File`)。全文: `pm-work/findings/manage-files-1521-file-menu-dfm.txt`。
- File の子(DFM の順。`Visible=False` 以外も含む):

| DFM 順 | 名前 | 内容 |
|---|---|---|
| 1 | NewMenu | Action `ShortcutsModule.FileNewAction` |
| 2 | TemplatesMenu | Caption `New from &template` |
| 3 | OpenMenu | Action `ShortcutsModule.FileOpenAction` |
| 4 | N1 | 区切り |
| 5〜10 | SaveMenu〜SaveNewTemplateMenu | Save 系の Action |
| 11 | DeleteUnusedProjectFiles1Menu | Action `MacroDeleteUnusedFilesAndSaveAction` |
| 12 | N14 | 区切り |
| 13 | ImportMenu | `&Import`(子 1 件) |
| 14 | ExportMenu | `&Export`(子 22 件) |
| 15 | LastFilesMenu | `-Recent projects`、**Visible=False** |
| 16 | ExitSepMenu | 区切り |
| 17 | **RevAutoSaveMenu** | Action **`ShortcutsModule.FileRevertToLastBackupAction`** |
| 18 | N91 | 区切り |
| 19 | **ExitMenu** | Caption `E&xit`、OnClick `ExitMenuClick` |

- 「Revert to last backup」の Action(`ShortcutsModule` の DFM。ファイル内 21950364 から。行 48): Caption `Revert to last backup`、OnExecute `FileRevertToLastBackupActionExecute`、OnUpdate `FileRevertToLastBackupActionUpdate`(Category `File`)。
- **項目の位置(読み)**: Revert(17)の次が区切り N91(18)、その次が Exit(19)。Revert と Exit の間に入れるなら、「Revert の後・N91 の前」か「N91 の後・Exit の前」の 2 つ。
- **確認できない**: `menus.tsv` の番号(#8 = Revert、#9 = Export 相当、#10 = Exit)と DFM の順は合わない。DFM の順では Export(14)が Revert(17)の前にある。drop-down の表示は `Action` の Visible・Enabled(OnUpdate で決まる)で項目が隠れる可能性がある(推定。確かめていない)。`File` の drop-down の項目数 10 件(menus.tsv)と、DFM の表示の子の数(19 件から Visible=False の 1 件を除く 18 件)も合わない。番号の対応は、実行で確かめるまで決めない。

## 3. 項目の名前(手順 2)

- 名前の出どころは DFM の Caption(静的な読み)。Revert は Action の Caption `Revert to last backup`、Exit は `E&xit`(`&` はアクセラレータ。表示は Exit)。
- 実行時の読み: `TMenuItem` の Caption は、guest のメモリの文字列(Delphi の文字列)。読むには番地とオフセットが要る。未実装。OCR は使わない方針どおり、この方法を使う。
- 「Manage files」は DLL に無い(`grep -a 'Manage files'` は 0 件。`Manage` を含む文字列は別の名前だけ: `Manage shortcuts`・`Manage plugins` など)。項目を足すには、文字列を guest のメモリに作る(または DFM の値を足す)必要がある。

## 4. 方式の候補と判定(手順 1)

RTTI から分かった番地(`manage-files-1521-dfm-read.py rtti`の出力。ファイル内位置ではなく VA。ImageBase 0x400000):

| 対象 | 値 | 根拠 |
|---|---|---|
| TMenuItem の VMT | 0x838c10 | TMenuItem の TypeInfo(0x839920)の ClassType |
| TQuickMenuItem の VMT | 0x725a58 | TQuickMenuItem の TypeInfo(0x725b70)。親は TMenuItem |
| TMenuItem.Caption の設定 | 0x8403e0 | 公開プロパティ `Caption` の SetProc(直接の番地) |
| TMenuItem.OnClick の格納 | 「フィールド」オフセット 0x100 | SetProc が `0xFF..0100`(Delphi の表記: フィールドのオフセット) |

- **(a) 項目の並びに、TQuickMenuItem を 1 件挿す(実行時に VCL の関数で作る)**: 選ぶ。理由: DFM を書き換えると、押下の処理の名前(`OnClick` や `OnExecute` の名前)を、所有者(TFruityLoopsMainForm か ShortcutsModule)の公開メソッドとして足す必要がある。この DLL に無い名前なので、新しいコードが要る。実行時なら、既存の VCL の関数を呼ぶ形で済む見込み。`fle_call_guest`(`lift/runtime/fle_callback.h`)で guest の関数を呼ぶ仕組みは既にある。
  - 残りの番地: (1) TQuickMenuItem の生成: 仮想コンストラクタ(VMT の枠)の番地。(2) 子の列への挿入: `TMenuItem` の `Items` の列を変える関数(公開されていないので RTTI に無い)。(3) FileMenu の実体: `FruityLoopsMainForm` の公開フィールドの番地(VMT の FieldTable から、オフセットは読める見込み)。(4) 押下の処理: `OnClick` に入れる guest の関数(既存の番地、または harness の関数)。
  - **確認できない**: (2) の番地と、列の内部の形(Delphi の `TList` の作り)。
- **(b) 項目の組み立て・押下の関数に harness の hook を入れる**: 見込みは薄い。`lift/harness/hooks/flengine-hook-points.txt` の名前付きの hook は 3 件で、メニューの関数ではない(この読みでは、メニューの関数の番地を特定していない)。`lift --hook-points` で番地を名前付けするには、番地を先に特定する必要がある。
- **(c) 項目の配列を、実行時に書き換える**: (a) と同じ。列の内部の形が要る。
- **(d) ページ側に File の見出しの下の項目を出す**: 採らない(エンジンの項目ではないので、依頼の「メニューに項目を足す」に当たらない)。
- **判定**(前回): (a) を方式に決める。ただし、番地の特定(§4 の残り)が済んでいないため、前回は項目を足していない。今回の結果は §8(方式は DFM の複写と HLE に変えた)。

## 5. 押下の先(窓を出す入口)

- ページ側の入口: `app/src/files/mount.ts` の `mountFiles`(窓を作る)と `FilesHandle.openWindow(path?)`(窓を出す)。今は `mountFiles` を呼ぶ箇所が無い(#1481 で外れた)。
- 押下の処理をページの `openWindow` に結ぶには、エンジンから呼べる入口(host の呼び出し)が要る。今は無い。#1519(File→Open の OPFS の窓)と同じ入口を使うのが自然(推定)。
- 代替案(推定): Manage files の項目の `Action` を、既存の `ShortcutsModule.FileOpenAction`(Open)にすれば、#1519 の後は Open の経路がそのまま OPFS の窓になる。ただし、表示の名前と意味が Open と同じになるので、依頼者の「Manage files」とは別物。採らない。

## 6. 次にやること(差し戻しの理由に対する道筋)

1. §4 の番地を特定する: TQuickMenuItem の生成(VMT の仮想コンストラクタの枠)、TMenuItem の子の挿入の関数、FruityLoopsMainForm の FileMenu の公開フィールドのオフセット。
2. harness に、FileMenu の子の列へ TQuickMenuItem を 1 件挿す処理を入れる(`fle_call_guest` を使う)。Caption は `Manage files`(文字列を guest に作る)。OnClick は harness の関数へ向ける(guest から呼べる入口が要る)。
3. 押下で、ページの OPFS の窓を出す入口を呼ぶ(#1519 の入口)。
4. 項目の名前の読み: TMenuItem の Caption を guest のメモリから読む harness の関数を作り、E2E の `menus/manage-files-order` で使う(§3)。
5. 上の 2 と 3 の後に、`pm-work/plan.md` の手順 5〜7(E2E の一覧の作り直し、赤の確認、シナリオの追加)へ進む。

## 6A. この作業での結果(受け入れの実行。製品は変えていない)

- 判定: **差し戻し**(項目を足す実装は無い。実装に要る番地と押下の処理は、§4 のとおり未特定)。
- 前回の失敗の原因: 受け入れの `node tools/e2e/run.mjs --only menus/...` が、既定の集合(`smoke`)に `menus/...` を探して「is not a scenario of the set」で落ちた。`run.mjs` の `loadSet` を直した: `--only` が領域の ID(`<領域>/<名前>`)だけのとき、その領域を読む(`lib/plan.mjs` の `areasOfOnly`。試験は `tools/e2e/test/scenario.test.mjs`。main の版と同じ動きのため、前回の重複の追加は外した)。
- 追加したシナリオ: `tools/e2e/scenarios/menus/manage-files-order.mjs`・`manage-files-opens-window.mjs`(`_generate.mjs` の一覧に入れた。生成し直しても消えない)。判定は `lib/menus.mjs` の `manageFilesOrder`・`manageFilesOpens`。
- 赤の出力(今の本番。runner の値。DPR 1 と DPR 2 の両方で同じ結果):
  - `menus/manage-files-order`: `"Manage files" in the engine's memory: no; "Revert to last backup": yes` / `place: not read (no reader of the File drop-down's items in their order yet)` / `ok=false`
  - `menus/manage-files-opens-window`: `"Manage files" in the engine's memory: no` / `no item named Manage files: nothing is chosen, no window is looked for` / `ok=false`
- 読みの結果: 「Revert to last backup」の文字列はエンジンのメモリに在る(`guestText`。読める)。「Manage files」は無い。
- 項目の数の食い違い: 窓の見える項目は 10 件(E0)。DFM の表示の子は 18 件(Visible=False の 1 件を除く)。項目の一覧は DFM のとおりではない(VCL が Action の Visible・Enabled で項目を隠す、と推定。確かめていない)。項目の番号 `#n` と DFM の順は一致しない。

## 6B. 差し戻しの理由と次の作業

1. 項目を組み立てる関数・項目を挿す関数(`TMenuItem.Insert` 相当)・項目の生成(`TQuickMenuItem.Create` 相当)の番地が未特定。RTTI と DFM からは番地が出ない(VCL の非公開のメソッド)。
   - この作業の調べ(調査専用の Python。記録には残さない): TQuickMenuItem の VMT(0x725a58)を RIP 相対の `lea` で指す命令は、`.text` に 0 件。VMT の値を持つ 8 バイトは 2 件だけ(vmtSelfPtr(VMT-0xC8)と、TypeInfo 0x725b70 の ClassType)。TMenuItem の VMT(0x838c10)も同じ。つまり、項目は FL のコードが直接の番地で作るのではなく、DFM の読み(TReader)が名前から VMT を引いて作る、と見るのが妥当(推定)。項目を直接作る番地を探す方針は当たらない。
2. 押下の処理(`OnClick` に入れる、guest から呼べる入口)が無い。ページの OPFS の窓を出す入口へ、engine から呼ぶ経路も無い。
3. 項目の一覧の読み(項目の並びを、窓の見え方の順に取る)が無い。`guestText` は文字列の有無だけ。並びの読みは、項目の Caption とリストの順を guest のメモリで辿る処理が要る。
4. 項目の番号: Revert と Exit の間に入れると、Exit より後の `#n` が 1 つずれる見込み(Revert が #8・Exit が #10 という推定に基づく。確かめていない)。`menus.tsv` と `_generate.mjs` の作り直し、既存のシナリオの番号の付け替えが要る。

## 7. 道具と再現

- `python3 pm-work/findings/manage-files-1521-dfm-read.py lift/out-app/host/binaries/win64/FLEngine_x64.dll dfm 20671368`: File の木(`manage-files-1521-file-menu-dfm.txt` の元)。
- `... rtti 0x725b70`(TQuickMenuItem の公開プロパティ)・`... rtti 0x839920`(TMenuItem)・`... find NAME`(TypeInfo の位置)。
- この読みの道具は調査専用で、製品と本番の処理には使わない(AGENTS.md の Python の決まり)。

## 8. この作業の結果(Issue #1521 の 2 回目以降。受理)

- 方式: DFM の複写(`lift/runtime/fle_res.c`)+ HLE(`lift/harness/hooks/flengine-hle-entries.txt`、`lift/runtime/fle_hle.c`)+ 既存のピッカーの経路(`lift/runtime/fle_filedlg.c`、`app/src/files/engine-dialog.ts`)。
  - DFM: 資源 RT_RCDATA `TFRUITYLOOPSMAINFORM`(RVA 0x1745f88、大きさ 150246)を、読み込みの時に複写する。`RevAutoSaveMenu` の後・`N91`(区切り)の前に、`TQuickMenuItem` `ManageFilesMenu`(Caption `Manage files`、OnClick `FileMenuClick`、Tag 0x4D46、Flags は空の集合)を挿す。アンカーが合わない時は止める。
  - 押下: `FileMenuClick` の番地は、`TFruityLoopsMainForm` のメソッド表(VMT-0x98 が指す表。`ExitMenuClick` 0x11d2940 と同じ表)にある。VA 0x121f770 / RVA 0xe1f770。HLE の guard は、送り手(rdx)の TComponent の Tag(+0x18。`Tag` の setter がこの場所に書く)が 0x4D46 の時だけ `fle_filedlg_manage_open()` を呼び、1 を返す。それ以外は本体を実行する。
  - 窓: `fle_filedlg_manage_open()` は既存のピッカー(`fle_filedlg_host_pick`)へ `{"manage":true,...}` を出し、結果は使わない。ページは `EngineFiles.select` の manage 分岐で、選ぶものの無い OPFS の窓(`Explorer`、普通のモード)を `#filedlg` に出す。閉じる(`#fx-close` または Esc)で要求が終わり、エンジン側の待ちが解ける。`mountFiles` は使わなかった(ヘッダに Files のボタンが増え、別の overlay になるため)。
- 項目の名前: ゲストのメモリの文字列を `guestText`(ASCII と UTF-16LE)で読む。`"Manage files"` は在る(E2E の notes: yes)。
- 位置(読み): DFM では `Revert to last backup` の後・区切り(N91)の前・`Exit` の前。ドロップダウンでは、灰色の `Revert to last backup` の下、区切り線の上、`Exit` の直上(上キーを 2 回押した画面で確認)。
- 番号の注意: 下キーの数は状態で変わる(灰色の項目は下キーで飛ばされる。最近のプロジェクトの項目と、プロジェクト関係の項目が増える)。この実行では `Import` が 10 番目。E2E は、上キーを 2 回押して `Manage files` を選ぶ(`tools/e2e/lib/menus.mjs` の `chooseManageFiles`)。`menus.tsv` の `#10`(Manage files)・`#11`(Exit)は、クラッシュ調査の状態に基づく推定で、今の状態とは一致しない。
- 窓の class: `fx-window`(`#filedlg` の中。`#fx-input-files` あり)。
- 直す前の赤: §6A(前回の記録。DPR 1 と DPR 2 で同じ)。
- 残り(未確認・範囲外):
  - 複写は DLL の資源の RVA と大きさに固定している。別の DLL では複写が掛からず、項目は出ない(E2E が検出する)。
  - Revert の有効・灰色の切り替え(バックアップの有無)は、この作業では確かめていない。
  - 窓の入口は `mountFiles` ではなく、#1519 の経路(選択窓の `EngineFiles`)の普通のモード。
  - 押下の番地(0xe1f770)は、この DLL のビルドに固定。
