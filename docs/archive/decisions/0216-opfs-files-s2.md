# 0216 段階 2(追記): エンジンの『開く/保存』を OPFS のファイルマネージャー(選択モード)で行った — 実エンジン SHIP2 が OPFS から FLP を開き、OPFS へ WAV を書き出した(WAV は 353,904 B・SHA-256 f5c6a08b… = ネイティブ〔決定 0188/0193〕とバイト一致)。仮の決定

状態: **仮の決定**。段階 1(コミット dddad9c)に続く段階 2。基準は HEAD d17b038(`patch -p1 --dry-run` clean)。成果は `opfs1-s2.diff`(MERGE-NOTES-S2.md)。段階 3(OPFS をゲストの実体に)は**未着手**。

区分の凡例は段階 1 と同じ: **実測**＝このセッションのコマンドの出力を見た事実、**読み**＝解釈で未確認、**未確認**、**出典なしの選択**＝`S-opfs1-…`(`tables/s-choices-s2.tsv`)。速度の数字はこの機械(16 コア、SwiftShader、他の担当が同時に走り load 30〜56)の壁時計で、依頼者の機械の値ではない。

## 冒頭の三点

1. **実行した本物と範囲(実測)**:
   - **実エンジン(出荷の既定の入口 SHIP2: `lift/out-websrv/assets/manifest.json`、`emscripten-fruity-v1`、`-DFLE_FILEDLG_BROWSER` 入り)**を、隔離 Chrome 156.0.8078.47(headless、この機械)の実 OPFS で動かした(`files-engine-dialog.mjs`、自分の `vite preview`、本番ビルド)。**模擬(テストの入力)**: エンジンのメニューとキー(CDP の `Input.dispatchMouseEvent/KeyEvent`: ブラウザ自身が作る trusted イベント。人の手の代わり)と、窓の中の選択(DOM の click/入力。人の代わり)。ファイルは窓自身の『Upload folder / Upload files』で OPFS に入れた。代役・再生は無し。
   - **File > Open**: エンジンの要求がページに届き(要求の約 6 ms 後ではなく、このテストでは click から約 2 s)、OPFS の窓が出て、6 つの種類(`COMDLG_FILTERSPEC`)と『Supported song files』の選択(filterIndex 6)を受け、`Sampler-project/Sampler.flp` を選んで Open → エンジンの題名が **'Sampler.flp - FL Studio 2026'**(約 1.6〜1.9 s)。答えは FLP(R)と隣のファイル(E: `SHP_Sawing Synth.wav`)、ゲストのパスは `C:\Users\user\Downloads\Sampler-project\…`。エンジンの『Recent projects』に `Sampler.flp` が載った(画面で確認)。
   - **Export(Ctrl+R)**: 保存の窓が出て、エンジンの提案名 `Sampler.wav`・9 種類(wav 選択)・開始フォルダ `/Sampler-project`(エンジンの `Downloads\Sampler-project` に対応)を受け、Save → エンジンの書き出し窓 → Start → **WAV が OPFS の `/Sampler-project/Sampler.wav` に 353,904 B・SHA-256 `f5c6a08b1dede642f91026e26939ca99cbea9e20f21bef0c2742c2d5dc3f6d8a` で書かれた**(決定 0193 のネイティブ/ブラウザの値と全桁一致。2.5〜2.8 s)。
   - **File > Save as**: 保存の窓(.flp の種類、窓の一覧は .flp だけ)。既存の名前 `Sampler.flp` を打つと『Replace the file?』(FOS_OVERWRITEPROMPT)、No でダイアログは開いたまま。新しい名前 `Copy of Sampler`(拡張子なし)で Save → 既定の拡張子 `.flp` が付き、**エンジンのトライアル版の『Buy FL Studio』窓(TInAppShopForm)が先に出て**、閉じるとエンジンが保存し、OPFS の `/Sampler-project/Copy of Sampler.flp`(111,204 B、`FLhd` で始まる)が書かれ、主窓の題名が 'Copy of Sampler.flp - FL Studio 2026' になった。
   - **File > Import > MIDI file**: 同じ形の Open の窓(MIDI の種類が選ばれて届く)。`mini.mid`(34 B、テストが作った最小の SMF)を選ぶと、答えは MIDI ファイル 1 つ(R)で、**エンジンが取り込み、パターン名 'mini'・チャンネル 1 が画面に出た**(目視。スクリーンショット `after-midi.png`)。
   - **File > Export > MIDI file**: 保存の窓(MIDI の種類選択)→ Save → エンジンの書き出し窓(出力形式 MID)→ Start でエンジンの**自分の『Warning』窓(TMsgForm、本文は描かれない)**が先に出て、Enter で閉じるとエンジンが書き、**OPFS の `Copy of Sampler.mid`(53 B、`MThd`)にページが書き戻した**。
   - **zip を選ぶ**: Open の窓で zip(種類『Zipped song file』)を選ぶと、ページが OPFS 内で展開し、中の `Z Sampler.flp` をエンジンが開いた(題名 'Z Sampler.flp - FL Studio 2026')。ここでエンジン自身の**『Confirm』箱(TMsgForm: Yes/No/Cancel。MIDI の取り込みで変更済みの project を置き換えるため)**が先に出た(文字は描かれない)ので、テストは『No』をクリックした。
   - **キャンセル**: ✕ ボタンと Esc でエンジンのダイアログが取り消しになり、エンジンは idle に戻り、キーボードがデスクトップに戻る。
   - **エンジンの要求を模したページテスト(エンジン無し)**: `selectScenario` 31 項目を **Chrome と Firefox 158** の両方で(要求の形は実エンジンが出した 2 件の JSON と同じ)。Chrome 全体 105/105、Firefox 全体 104/104(差は Chrome だけの本物のフォルダのドロップ 1 項目)。
   - **native -O0 の harness**(`wall_opfs1.rs` 4 件): `fle_filedlg.c`(`-DFLE_FILEDLG_BROWSER`)に足した 2 つの変更を、ページを C の stub で置き換えてテスト。**答えはテストの入力(模擬)**。変異検査: 元の runtime(`FLE_OPFS1_RT_DIR`)に向けると 3 件が落ちる。既存の `wall_fpick1`(6)・`wall_dlg1`(29)・`wall_exp1`(15)は変更後の runtime で全件通る。
2. **結論**: (a) エンジンの Open / Save / Import / Export の Show() を、`<input type=file>` の箱でなく OPFS の選択モードの窓が答えるようにできた。**ブラウザの選択窓を開かないので、transient activation の 5 秒の制約は無い**(窓の中のクリックは OPFS の読み書きに活性化を要しない)。Open は『選んだ FLP とその隣のファイル』をエンジンの仮想ボリュームへコピーして渡し、Save は『エンジンが書き終えたファイル』を選んだ OPFS のパスへ書き戻す。(b) **ゲストのパス ↔ OPFS のパスは暫定の対応**(`C:\Users\user\Downloads\<OPFS のパス>`: `tables/guest-path-map-provisional.tsv`)で、段階 3 が本当の割り当てを決める。(c) zip はページ側の展開(0x7075 対応)で開く。エンジン自身の DelZip64 経路(dec1)は HEAD にまだ無く、比較表 `tables/zip-route-comparison.tsv` は『ページ側=実測、エンジン側=未測定』。(d) **エンジンの GetFileTypeIndex は、人が窓で選んだ種類を返さない**(答えのプロトコルに欄が無く、未知の行は C 側が取り消す)。`TYPE <n>` 行を足す runtime の変更(+ JS ライブラリ)と、フォルダのダイアログ(FOS_PICKFOLDERS)を C 側が取り消さずにページに聞く変更を作り、native の harness で確かめた。**SHIP2 のバンドルには入っていない**(バンドルの再生成は ship2 の担当の領域)。ページ側は古いバンドルに対して安全。(e) 個人情報の無い**使用状況の数**(Files を開いた回数、アップロードの種類と件数、ドロップの種類、失敗コード、persist の結果、エンジンのダイアログの件数)をビーコンの任意ブロック `files` に足した(§6)。
3. **未確認・未着手**: **依頼者の Firefox 159 / Windows 10 での実エンジンの Open/Save(0 件)**(Firefox 158 headless Linux では Open まで実エンジンで確認、Save/Export は Firefox では未実行)。実際の人の操作。フォルダ選択(FOS_PICKFOLDERS)の**実エンジンでの確認**(バンドルの再生成が要る)。GetFileTypeIndex の中継の実エンジンでの確認(同上)。`Export > Project data files...` など、エンジンが選ばれたフォルダへ複数のファイルを書く機能(書き戻しは単一の Save 対象だけ)。ページの窓を使わずに OPFS を実体にする(段階 3)。数十 MB のサンプルを持つ project の Open(上限 256 MiB/ファイル・384 MiB/合計・512 ファイル。超えると project だけを渡して注意を出す)。

## 1. エンジンの要求と窓の対応(表: `tables/dialog-request-mapping.tsv`)

エンジンは `IFileOpenDialog`/`IFileSaveDialog` の `SetFileTypes`(`COMDLG_FILTERSPEC`)・`SetFileTypeIndex`・`SetFileName`・`SetDefaultExtension`・`SetFolder`/`SetDefaultFolder`・`SetTitle`・`SetOkButtonLabel`・`SetOptions` を呼び、`Show()` で止まる。C 側(`fle_filedlg.c browser_picker`)が JSON(`save`・`options`・`filterIndex`・`title`・`okLabel`・`fileName`・`defaultExt`・`folder`・`filters`)にしてページへ渡す。窓での扱い:

| エンジンの値 | 窓 |
|---|---|
| `filters`(名前, `*.a;*.b`) | 種類のドロップダウン。選んだ種類に合うファイルとフォルダだけを一覧に出す(`*.*` は全部)。人が種類を替えられる |
| `filterIndex`(1 始まり) | ドロップダウンの初期位置。**人が替えた位置は `typeIndex`(+1)として答えに載せる** |
| `fileName` | 保存: 名前欄の初期値(拡張子が無ければ `defaultExt`、無ければ種類の最初の拡張子を足す)。開く: 無視(欄は選択に追従) |
| `defaultExt`(`.wav` のようにドット付きで来る) | 保存で、ドットの無い名前に足す |
| `folder`(`SetFolder`、無ければ `SetDefaultFolder`) | 注記として表示。`C:\Users\user\Downloads\<x>` なら OPFS の `/<x>` で窓を始める。それ以外(`Documents\Image-Line\…`)は対応する OPFS が無いので、最後に使ったフォルダか `/` |
| `title`・`okLabel` | 窓の題『FL Studio asks: <title>』(既定 Open / Save as / Choose a folder)・OK ボタンの文字 |
| `options` | `FOS_ALLOWMULTISELECT` → 複数選択。`FOS_OVERWRITEPROMPT` → 『Replace the file?』。`FOS_PICKFOLDERS` → フォルダ選択の窓(**ただし C 側が取り消す: 下の発見 2**)。`FOS_FILEMUSTEXIST` → 打った名前が無ければ窓に残る |

実エンジンの 2 件の要求(Open: `options 0x20001000`・6 種類・`folder …\Projects`・`defaultExt .flp`、Export: `0x20000002`・9 種類・`fileName Sampler.wav`・`folder …\Downloads\Sampler-project`)を、`selectScenario` が同じ形で再現する。Save as(`fileName Sampler.flp`、`filterIndex 1`)と Import MIDI(`filterIndex 3`、**`fileName` と `defaultExt` が直前の Save as の値のまま残る**: FL は MIDI の Open で名前を初期化しない)も実測した。

## 2. 設計(`app/src/files/`)

- `select.ts`(純関数): `configOf(request, startDir)`(窓の設定)、`filtersOf`/`matchesFilter`/`saveExtension`、`guestPathOfOpfs`/`opfsPathOfGuestFolder`(暫定の対応)、`planOpen`(選んだ FLP と同じフォルダの隣のファイル。他の `.flp` と下位フォルダは含めない。512 ファイル/384 MiB/256 MiB の上限を超えるなら project だけを渡して注意を出す)、`planFolder`、`pickProject`(zip の中のいちばん浅い `.flp`)。
- `explorer.ts`: 選択モード(`select` オプション): 一覧は種類で絞る、名前欄は選択に追従、OK(Open/Save/Choose)、Enter、ダブルクリック、Esc/✕/Cancel でキャンセル。保存は拡張子の補完・フォルダ名を打つとそのフォルダを開く・Windows で不正な名前を理由つきで拒否・既存の名前は `FOS_OVERWRITEPROMPT` のとき確認。開くは打った名前を大文字小文字無視で探す・複数選択は `FOS_ALLOWMULTISELECT` のときだけ。
- `engine-dialog.ts`(`EngineFiles`): `FileDialogView` の `select` フックとして窓を作り、答えを作る。**開く**: 選んだ `File`(OPFS の `getFile()`)と隣のファイルを `FileDlgItem`(R/E、ゲストのパス)にして返す。zip を選んだら OPFS 内に展開して(進捗と中止つき)中の `.flp` を R にする。**保存**: ゲストのパス(S)を返し、`pendingSaves` に覚え、エンジンが書き終えた `saved-file` を選んだ OPFS のパスに書き戻す。書き戻せなければ従来の『Files the engine wrote』に Download が残る。**フォルダ**: D + 中のファイル(E)。答えを作れないとき(ファイルが消えた等)は窓に理由を出して開いたままにする(エンジンは待ち続ける)。
- `ui/filedlg.ts`: `FileDlgHost.select` が与えられていれば箱を作らず窓を作る(約 +30 行)。`main.ts` は `EngineFiles` を作って渡し、`savedFile` で `onSavedFile` を呼ぶ(`?filedlg=box` で従来の箱)。
- **runtime**: `fle_filedlg.c`(`#ifdef FLE_FILEDLG_BROWSER` の中だけ): 答えの `TYPE\t<n>` 行を `GetFileTypeIndex` に反映(範囲外は無視)、`D` ロール(フォルダの `fle_vfs_make_dirs`)、`FOS_PICKFOLDERS` を取り消さずにページに聞く(`R` の答えは取り消し)。`fle_filedlg_lib.js`: `typeIndex` と `D` の転送。**エンジンを止める経路は足していない**(ページの誤りは取り消し)。

## 3. テスト

- vitest: `files-select.test.ts`(13)、`files-usage.test.ts`(30)、`files-summary.test.ts`(3)、`files-paths.test.ts` に 1 件。**全体 1,409 passed + 7 skipped**(HEAD の同じ木で段階 2 の 4 ファイルを除いた値は 1,362 + 7 skipped: +47)。型検査(4 つの tsconfig)rc=0。
- ページ e2e(エンジン無し): Chrome **105/105**、Firefox 158 **104/104**(段階 1 の 60/59 + 選択モード 31 + 使用状況 11 + 保存状態の警告 3)。
- 実エンジン e2e(`files-engine-dialog.mjs`、Chrome、SHIP2、全ステップ open→save→saveas→savemidi→midi→zip→cancel): **50/50**(`logs/final-s2-engine.txt`、`tables/engine-dialog-run.tsv`)。
- native -O0: `cargo test -p fe-lift --test wall_opfs1`(4 件)+ 既存 `wall_fpick1`・`wall_dlg1`・`wall_exp1` 計 50 件が通る。変異検査(元の runtime に向ける)で 3 件が落ちる。
- Playwright は使っていない(CDP の直接呼び出しと geckodriver の WebDriver)。

## 4. 発見

- **F1(GetFileTypeIndex)**: 人が窓で選んだ種類はエンジンに届かない(SHIP2 以前の橋)。`typeIndex` と `TYPE` 行で直した(runtime 変更。SHIP2 には未収録)。
- **F2(フォルダ選択)**: `FOS_PICKFOLDERS` は C 側がページに聞かず取り消す。`Export > Project data files…` などはこれに当たる読み。D ロールで直した(同上)。
- **F3(エンジン自身の message box の本文が描かれない)**: `Confirm`(Save changes?)・`Warning`・`Buy FL Studio`(TInAppShopForm)は、アイコンとボタンだけで本文が空。人は何を聞かれているか読めない。ページ側ではなく lift したエンジンの文字描画(読み)。txt1(決定 0219)が調査中と PM から連絡あり。
- **F4(トライアル)**: File>Save as は『Buy FL Studio』が先に出て、閉じると保存される(OPFS に書かれたのは確認済み)。保存した project の題名も追従。
- **F5(Export > MIDI file)**: 保存の窓 → エンジンの書き出し窓(出力形式 MID)→ Start で、トライアルの Buy 窓と同じく**エンジン自身の Warning 窓が先に出る**(本文は描かれない: F3)。Enter で閉じるとエンジンが書き、OPFS の `/Sampler-project/Copy of Sampler.mid`(53 B、`MThd`)にページが書き戻した。
- **F6(Save changes?)**: 変更済み project を置き換えるとき、エンジンは Open の前に自分の Confirm を出す(本物の FL の動作)。テストは『No』を座標でクリックする(`N` キーは効かなかった: 実測)。
- **F7(Recent projects)**: 窓で開いた project はエンジンの『Recent projects』に載る(画面で確認)。
- **F8(Import MIDI の要求の名前)**: 直前の Save as の `fileName`/`defaultExt` が残って届く(§1)。窓は開くモードでは名前を使わないので影響なし。

## 5. Firefox

- ページテスト(選択モード 31 項目、保存状態の警告、使用状況)は Firefox 158 headless で通る。実フォルダ入力・実フォルダのドロップ・`persist()` の応答は WebDriver では作れない(模擬または未確認)。
- **実エンジンを Firefox 158(headless、geckodriver)で動かした**(`files-engine-firefox.mjs`、**5/5**): JSPI(`WebAssembly.Suspending`/`promising`)があり、実エンジン(SHIP2)が 14.0 s(壁時計、load 25。P1 の値ではない)で最初の idle に達した。WebDriver のポインタ操作(ブラウザが作る trusted イベント。人の手の代わり)で File>Open をクリック → **OPFS の窓が出て**(6 種類)、`Sampler-project/Sampler.flp` を選ぶと題名が **'Sampler.flp - FL Studio 2026'**(2.2 s)。つまり『File>Open で選択画面が出ない』は、依頼者と同じ種類のブラウザ(Firefox)でも、この経路では出る。**依頼者の Firefox 159 / Windows 10 ではない**。
- 依頼者の Firefox 159 / Windows 10 でこれらを確認する手段として、使用状況ビーコン(§6)を足した。

## 6. 使用状況ビーコン(PM の依頼 (4)。**実装した**)

- ビーコン版 3 のまま、任意の `files` ブロック(決定 0209 の `engine.exit.message` と同じ流儀: 古いサーバー/読み手は無視できる)。`src/files/usage.ts`(数える側)、`src/perf/beacon.ts`(+6 行)、`shared/perf.ts`(`beaconProblems` が `files` を検査: 失敗コードは固定の 17 個以外を指摘、負数・非整数・未知の counter を指摘)、`scripts/perf-log-summary.mjs`(『File manager』節: ブラウザ別の実フォルダ選択/実フォルダのドロップ/persist の結果/失敗コード)。
- 載せるもの(全部が整数・真偽・`null` と固定コード): Files を開いた回数、chooser の種類(files/folder/zip)・drop の種類(files/folders/zips)、書いた/飛ばした/失敗したファイル数と MiB、展開した zip、ダウンロード、操作(mkdir/rename/delete/copy/move)、`persist()` の asked/granted/denied/pending(3 秒答えが無い=Firefox の確認待ち)、`estimate()` の MiB、エンジンのダイアログの件数(open/save/folder、answered/cancelled、zipUnpacked、savedBack)、失敗コード。
- **載せないもの**: ファイル名・フォルダ名・パス・内容・個別の大きさ・時刻・エラー文。単体テスト(`files-usage.test.ts` の PRIVACY)と e2e(『テストで使った名前が JSON に無い』『文字列は失敗コードだけ』)で検査。ページのログ(`addLog`)にもファイル名は出さない(act1 の R3)。
- Files を一度も使わなければブロックは付かない。`?beacon=0` は従来どおり何も送らない。

## 7. 出典のない選択(S- の一覧。全文は `tables/s-choices-s2.tsv`)

S-opfs1-guest-map、S-opfs1-siblings、S-opfs1-siblings-limit、S-opfs1-zip-open、S-opfs1-start-dir、S-opfs1-type-switch、S-opfs1-overwrite、S-opfs1-typeindex、S-opfs1-folder-files、S-opfs1-no-box-fallback、S-opfs1-usage-block、S-opfs1-usage-3s、S-opfs1-engine-confirm-no。

## 8. act1 の R1〜R6(`lift/out-git/act1-results/MERGE-NOTES.md`)への対応

| # | 状況 |
|---|---|
| R1 保存(`licensevalidation.txt`、`*.txt`、`Documents`、`0x20000802`) | 窓は提案名・フィルター・開始(対応する OPFS が無いので注記と最後のフォルダ)・上書き確認を**エンジンの値のまま**使う(実エンジンの Export/Save as/MIDI export の 3 種で確認、`licensevalidation.txt` 自体は未実行)。保存されたファイルは OPFS に書き戻され、Files 窓の『Download』または『Files the engine wrote』から取り出せる |
| R2 開く(`*.reg`、拡張子 `reg`、`Downloads`、`0x20001000`) | `Downloads` は OPFS のルートに対応するので窓は `/` で始まる。ローカルからは窓の Upload ボタン(files/folder/zip)で入れてから選ぶ。大文字小文字無視(`FLRegkey.Reg`)。FILEMUSTEXIST: 存在しない名前は窓に残る。取り消しはエンジンに『取り消し』を返す |
| R3 検証ファイルをサーバー等に載せない | ビーコンの `files` ブロックに名前・パスなし。ページのログにファイル名を書かない(書き戻しのログも名前なし)。ファイルは OPFS から外へ出ない(Download はブラウザ内の Blob URL) |
| R4 レジストリの永続 | **段階 3 の設計メモ(§9)**。OPFS のルートと Worker の書き込み経路(`createSyncAccessHandle`)は段階 3 の測定で選ぶ |
| R5 `/fl/state/`・`/fl/license/` を黙って消さない | **実装した**: それらと、それを含むフォルダ(`/fl`、`/`)の削除・改名・切り取りは警告つきの確認(`protectedHit`、単体テスト 1 件、e2e 3 件)。`persist()` の要求ボタンは窓にある(自動の要求は未実装: Firefox は確認を出すので、利用者の操作に結びつけるのがよいという読み) |
| R6 上書き確認 | 保存で既存の名前なら『Replace the file?』(エンジンの `FOS_OVERWRITEPROMPT` に従う)。『以前の検証ファイル』という文言は付けていない |

## 9. 段階 3 への入力(未着手。設計メモ)

- 段階 2 でゲストが見るファイルは**コピー**(MEMFS)。段階 3 の目的は、コピーなしで OPFS を読むこと、設定/書き出しを残すこと、act1 の R4(レジストリの永続)に OPFS のルートと同期書き込みの経路を与えること。
- 比較する候補(MDN BCD: Chrome 102+・Firefox 111+ の両方に `createSyncAccessHandle` がある): (1) JSPI の中断で非同期の `open` を待つ(エンジンは既に JSPI。`fle_idle_wait` と `fle_filedlg_host_pick` が同じ仕組み)、(2) ヘルパー Worker + SharedArrayBuffer + `Atomics.wait` の同期橋、(3) 列挙(FindFirstFile)時に先読みして同期ハンドルを開く。コスト・Firefox での JSPI の有無・書き込みの永続(flush/close)を測って選ぶ。
- **ロックの競合**: 同期ハンドルを開いたファイルは、ページの窓から削除・上書き・改名ができない(`NoModificationAllowedError`)。窓は『使用中』と言うだけ(`describeError`)。段階 3 では窓がそのファイルを使用中と示す手段が要る。
- メモリ: 数十 MB のサンプルを MEMFS に全コピーしない(今は上限 256 MiB/ファイル・384 MiB/合計)。

## 10. 自己申告

- sleep を 1 回呼んで拒否された(2 回目)。他の担当の実行中のプロセスは触っていない。自分の古い実行を PID で止めたこと数回。
- テストの最初の版が誤った結論を出した: 『Save as はトライアルでは保存されない』と書いたが、実際は『Buy FL Studio』窓を閉じると保存される(テストが閉じていなかった)。直して F4 に書いた。
- 実エンジン e2e の失敗の多くはテストの書き方(競合・座標・メニューの状態)で、ページの不具合ではなかった。ページの不具合として直したのは: 窓の初期ナビゲーションとの競合で `engine.explorer` が早く現れる件(最初の一覧が出てから公開するように変更)。
- 段階 1 の `files-opfs.mjs` が書いたテスト用データの置き場(リポジトリ直下)を、`APP_WORK` の下に変えた(PM の指摘)。

---

## PM の判断と取り込み(2026-11-04)

- **取り込み**: `opfs1-s2.diff`(26 ファイル、`-` 行 41 本はすべて 1 行の置換で PM も全部読んだ)を HEAD 55e4e83 に当てた。clean(offset・fuzz なし)。PM 実測: 型検査 rc=0、vitest 1,409 passed + 7 skipped、Worker 43、e2e 1,109/1,109(1 回目は Rust の全体テストと同時で perf-beacon の 7 件がテスト間の遅れたビーコンの競合で落ち、perf-beacon 単独は 46/46、負荷の少ない状態での全体の再実行は 1,109/1,109。製品の不具合ではなくテスト分離の競合と読む)、Rust 全体テスト 184 results / 2,311 passed / 0 failed(基準 183 / 2,307 + wall_opfs1 4)。FLEngine の既定の生成 C は fe-lift が無変更のため `cd9107ce…` のまま。a4 は runtime の `fle_filedlg.c`(`#ifdef FLE_FILEDLG_BROWSER` の中だけ)以外に触れないので再実行していない。
- **既定の見え方が変わる**: エンジンの File>Open / Save / Import / Export の選択が、`<input type=file>` の箱でなく OPFS の窓になる(依頼者の『File>Open の選択画面が出ない』『プロジェクトは zip を読む必要がある』への答え)。`?filedlg=box` で従来の箱に戻せる。OPFS が使えないときは窓が『使えない』と言って取り消す(箱へは落ちない。必要になれば `engine-dialog.ts` の失敗側で箱へ戻す)。**現在の既定の入口(SHIP2)の wasm には `-DFLE_FILEDLG_BROWSER` の TYPE 行/フォルダの修正(runtime)が入っていないので、実エンジンでの確認は ship3 で行う**(担当の 50/50 は SHIP2 の wasm にページ側だけの変更を当てた結果)。
- **ここまでの限界(再掲)**: 依頼者のブラウザ(Windows の Firefox 159)での確認は 0 件。選択・キー・クリックは CDP / WebDriver の模擬入力。Save as / Export>MIDI は、エンジン自身の Buy / Warning 窓(本文が描かれない。txt1 が原因を追っている)を閉じると書かれる。ライセンスの回避はしない。
- **段階 3(同期アクセスの実測、act1 の R4 = レジストリ/ライセンスの永続化を含む)**は担当が続ける。act1 の R5(`/fl/state`・`/fl/license` の保護)は実装済み。
