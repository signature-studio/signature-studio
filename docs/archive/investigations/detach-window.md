# 追加要求: Web での「デタッチ窓」(FL の Detached = 独立したポップアップ窓) — 静的分析・設計制約・ブラウザ実測

作成: 2026-10-30〜31(作業開始時の HEAD 167a1db、書き終わりの HEAD abb4151 = 決定 0121 まで進んでいた。ブランチ main。窓表 `fle_win.c` の記述は 167a1db 時点で読み、書き終わりに差分(`msgonly` の追加、+39 行)を確認した)。**調査のみ(静的分析と小さなブラウザ実験)。何も決めない**(優先度・延期・前倒しは PM が決める)。FL 本体の実行、ビルド(cargo/emcc/clang)、実装はしていない。実機 Windows での FL のデタッチの観測は**無い**。FLEngine は `binaries/win64/FLEngine_x64.dll`(SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` を確認)を読み取りだけで使った。

## 0. 先頭の要約

| 項目 | 結果 |
|---|---|
| デタッチの実体(静的の読み) | **メニュー項目 `&Detached`(`TQuickMenuItem`、AutoCheck)→ `*DetachMenuClick` → 共通のトグル `f_1245f30`(読み) → VCL の仮想 `SetParent`(vtable+0x138 = `f_85c580`)で `Parent := nil` / `Parent := WorkPanel`**。`SetWindowPos(HWND_TOPMOST)` も `WS_EX_TOPMOST` も `SetParent` API もトグルの中では使わない。**VCL の流儀で窓ハンドルを破棄して作り直す**(`DestroyWindow` → のち `CreateWindowExW`。`SetParent` API の呼び元は 2 関数だけで、フォームの Parent 変更とは別)。確度: 呼び出し鎖は読みで固い、VCL との同一視は記憶による(Embarcadero のソースは未参照)ので中〜高 |
| 位置関係の保持 | **OS の機能ではなくエンジンが座標を変換**する: トグル時に `WorkPanel` のクライアント原点(`ClientToScreen`)を足し引きして、画面座標 ↔ WorkPanel 相対座標を入れ替える(`f_1245f30`、`f_132ec10`)。デタッチ後は top-level の画面座標(読み。**メイン窓が後で動いたとき追従するかは未確認**) |
| 「常に最前面」 | トグルは `FormStyle`/`WS_EX_TOPMOST` に触れない(読み)。デタッチ窓は **`WS_POPUP` の top-level + オーナー窓あり**(`CreateParams` = `f_85f250` が `FParent`/`ParentWindow` が無いとき `WS_POPUP`(0x80000000)を書く命令列と、`PopupMode`/`PopupParent` を読む分岐があることまでを読んだ。`pmExplicit` が 4 クラスの DFM 値。**`Params.WndParent` が実際に非 NULL になることは実行していないので未確認**。読み)。Win32 では「オーナー窓の上に常にいる」(Learn Owned Windows)ので、**TOPMOST 無しで FL の上に出る**と読める。`SetFormStyle(fsStayOnTop)`(`f_85e110`)は別の機能で、呼び元 7 か所(うち 4 か所が fsStayOnTop を渡す) |
| スナップの無効化 | エンジンの `WM_WINDOWPOSCHANGING` ハンドラ(`f_8076c0`、`f_8058b0`)と VCL の `ScreenSnap`(`f_8620b0`)による**エンジン側の処理で、OS の機能ではない**。`ScreenSnap` は **`Parent == nil` のフォームだけ**に効く(`f_8620b0`、読み)。「デタッチ中は他窓とスナップしない」という挙動の分岐は**静的には特定できていない(未確認)** |
| 窓モデルに今入れておくべき属性 | hWndParent(親/オーナーの別を style で解釈)、親相対/画面座標の矩形、窓ごとの Z 順(兄弟リスト + TOPMOST 群 + `hWndInsertAfter`)、`WS_EX_TOPMOST` を `SetWindowPos` が更新、**窓の破棄と再作成(新しい HWND)**、`DestroyWindow` の子/オーナー連鎖、`SetPropW`、`WM_WINDOWPOSCHANGING` で編集できる `WINDOWPOS`、**top-level 窓ごとの描画面**(§3) |
| ブラウザ方式の実測(headless Chrome 156、3 回の中央値) | ポップアップ描画 **約 60 fps(中継 59.9〜62.5、MessagePort 直結 59.9〜62.5。生成側が 16 ms 周期の代役なので能力値ではない)**、Worker→ポップアップの到達 **p50 0.2〜0.3 ms**(0.1 ms 前後は計測ノイズの範囲)、入力往復(mousemove)**p50 0.5〜1.1 ms / p99 0.9〜1.8 ms**(外れ値 約 10 ms あり)。**Document PiP: 可**(headless、合成クリック後)。ただし 1 タブに 1 窓、位置を指定できない |
| 方式の要点 | `window.open` は**ユーザー操作 1 回につき 1 窓、有効約 5 秒**(4.5 秒で成功、5.5 秒で拒否)→ **ユーザー操作の無い時点(プロジェクト読み込み時のデタッチ復元)や、1 回の操作で複数窓を開く「全プラグインをデタッチ」は、`window.open` では拒否される見込み**(拒否は活性なしと同じ条件で実測。FL のこれらの操作が実際にどの経路で `SetParent` に届くかは未確認)。ポップアップはオープナーと **同一レンダラプロセスで動き、main thread を塞ぐと表示も止まる挙動**(オープナーの main thread を 300 ms 塞ぐとポップアップへの描画間隔が最大 317 ms。ただし `canvas` 経路〔Worker が直接描く〕は塞がれても 60 fps のまま)。`Atomics.wait` で戻らない Worker は、**ポップアップから渡した OffscreenCanvas にも表示されない**(3/3 回) |

## 1. 凡例・環境・手順

- **実測** = このセッションでコマンドを実行して得た値(`docs/investigations/detach-window/` の JSON/TSV、`lift/harness/web/detach/`)。**読み** = 逆アセンブル、RTTI、DFM、既存文書、Learn/MDN の読みからの解釈。**未確認** = 調べていない、または調べたが決められない。
- Windows の挙動は Microsoft Learn(2026-10-30 取得。URL・取得時刻・HTML の SHA-256 は `detach-window/sources.tsv`)。ブラウザ API は MDN と Chrome の公式資料(同ファイル)と実測。
- **環境**: Chrome 156.0.8078.47(headless=new、`--no-sandbox`)、専用の一時プロファイル `/tmp/opencode/detach/chrome-profile`、localhost のみの静的サーバー。**headless には window manager が無い**: 窓の位置・最前面・窓間のフォーカスは実物ではない(§4.5)。**この機械の Chrome は既定のブラウザコンテキストで `http://127.0.0.1` の読み込みが完了しなかった**(要求がサーバーに届く前にキャンセルされる。起動フラグの組み合わせを約 40 通り(数えた概数)、net-log、生ソケットで調べたが原因は見つからなかった)。**全測定は `Target.createBrowserContext` で作った隔離コンテキストで行った**(§8)。
- 静的分析の道具: `docs/investigations/goal-ladder/fle_index.py`(capstone と pefile で `.pdata` 57,816 関数の索引。約 8 秒)、`detach-window/tools/`(DFM の先頭プロパティ、Delphi の VMT、逆アセンブル、集計。標準ライブラリ + capstone/pefile)。機械語の引用は本文に出さない(関数 VA と読みの要約のみ)。
- 手順と成果物の対応: `lift/harness/web/detach/README.md`(再現コマンド)、`detach-window/summarize.py`(JSON → `measure.tsv`、`measure-median.tsv`、`other.tsv`)。

## 2. 静的分析: FL のデタッチの Win32 上の実体(読み。実行していない)

FLEngine は Delphi(VCL)製(`Embarcadero Delphi for Win64 compiler version 36.0`。決定の既存の読み)。クラス名は RTTI、フォームの定義は `.rsrc` の RT_RCDATA(TPF0、128 件。うち `T*` のフォーム 104 件)、メソッドは VMT と RTTI のメソッド表から引いた。関数の命名は `f_<va>`(`docs/investigations/goal-ladder` と同じ)。

### 2.1 入口: メニュー項目とクリックハンドラ(読み。DFM と RTTI は実測の数え上げ)

`.rsrc` の `TFRUITYLOOPSMAINFORM`、`TFXFORM` 内に `AutoCheck = True`、`Caption = '&Detached'`(`&D` が (D) のアクセラレータ)の `TQuickMenuItem` が 4 つ:

| メニュー項目(所属するポップアップメニュー) | 呼ばれるハンドラ | 本体 | 対象フォーム(読み) |
|---|---|---|---|
| `EEDetachMenu`(`EEMenu`) | `EEDetachMenuClick` = `f_12461b0` | `f_1245f30(*[15e4558])` | `TEventEditForm`(`PopupMenu = FruityLoopsMainForm.EEMenu`。キャプション `Piano roll -`。`EEMenu` の項目にプレイリスト系(`EEDropAudioToPLMenu`、`EEResizeAllPLTracksMenu` ほか)があるので、**プレイリストとピアノロールはこのフォームの別パネル(`TEventPanel_AU`/`_PR`)と読む。プレイリスト専用のフォームクラスは VMT 一覧に無い**。未確認) |
| `SSDetachMenu`(`SSMenu`) | `SSDetachMenuClick` = `f_120c860` | `f_1245f30(*[15e14a0])` | `TStepSeqForm`(キャプション `Channel rack`) |
| `FXDetachMenu`(`TFXForm` 内) | `FXDetachMenuClick` = `f_129c910` | `f_1245f30([16bd450])` | `TFXForm`(名前から FX/ミキサー側の窓と読む。未確認) |
| `PDetachMenu`(`PluginMenu`) | `PDetachMenuClick` = `f_1245ef0` | 現在のプラグインのフォーム(`[[15e2c38]+0xb8]->[+8]`)の `FParent != 0` を渡して `f_132ec10` | `TPluginForm`(プラグインのエディタ窓) |

- 共通のトグルは **`f_1245f30`**(呼び元 5: 上の 3 ハンドラ、`f_132ec10`、`f_11fab40`)。`f_132ec10`(呼び元 5: `f_1245ef0`、`f_1213580` ほか)はプラグイン単位で、エディタ窓がまだ開いていなければ**保存してある位置とフラグ(`[+0x34] |= 4`)だけを更新**し、開いていれば `f_1245f30` を呼ぶ。
- マクロ「全プラグインのエディタを切り替え」: `MacroSwitchEditorsToDetachedActionExecute` = `f_f1de70`(`Attached` 版は `f_f1de50`)→ `f_1213580(app, detach?)` → 全プラグイン(と 10 個の別リスト)に `f_132ec10`。メニュー `SSDetachAllPluginsMenu`、`SSAttachAllPluginsMenu`、`DetachAllPluginsMenu` はこのアクションに結ばれる。
- 設定: `Detach all new plugins`(`DetachPlugsBox`、ヒント「Detach all newly added plugins by default」、ハンドラ `f_135dd50`)、`Open detached plugins with the FL Studio window`(`DetachedPlugsOnMainScreenBox`、ヒント「Open detached plugins on the same screen as the FL Studio window」)、`Don't limit windows to screen`(`WindowLimitBox`、「Allows you to drag windows outside the screen」)。UI 文字列に `This plugin's editor window is detached.`(ラベル `lblMessage3`)、`Hide while dragging (when detached)`(ブラウザ窓。ヒント「Hide the (detached) browser window while dragging an item」)。**ブラウザ窓 `TBrowserForm` には `*DetachMenu` の項目が無い**(DFM の走査)。
- **RT_STRING(`LoadStringW` の ID)に Detach/Snap/Always on top は無い**(RT_STRING 40 ブロック、非空 625 件を走査。該当は VCL の `Dock zone ...` と `Floating point ...` だけ)。UI 文字列は `.data` の UTF-8/ASCII の文字列表(`Always on top` は 0x1067e75 にあるが、結ばれたコントロールは特定していない。未確認)。

### 2.2 (a)(b) トグルの中身: Parent の付け替えと座標変換(読み)

`f_1245f30(form)` の流れ(命令の引用はしない):

1. フォームが最大化(`[+0x4c2] == 2`)なら `f_85eb80(form, 0)`(`WindowState := wsNormal`、`ShowWindow`)で戻す。サイズ(`f_85bd40`/`f_85bd70`)を取る。
2. **`[form+0x78]`(`TControl.FParent`。RTTI の `Parent` の getter がこの位置)が 0(= 今は top-level)なら「アタッチ」**: `WorkPanel`(メインフォームの published フィールド、オフセット 0xb18)に対し、フォームの画面位置を `f_609b20`(`ScreenToClient` 相当: 仮想メソッド `ClientOrigin`(vtable+0xe0 = `f_617b20`、中身は `ClientToScreen`)を引く)で WorkPanel 相対に変え、WorkPanel の矩形(`f_609710`)に収まるように丸め、**`vtable+0x138`(`SetParent`)を `WorkPanel` で呼び**、**`vtable+0x188`(`SetBounds`)**で位置とサイズを戻す。
3. **`FParent != 0`(今は子)なら「デタッチ」**: `f_609970`(`ClientToScreen` 相当)で位置を画面座標に変え、**`vtable+0x138`(`SetParent`)を `nil` で呼び**、`LockWindowUpdate(NULL)` を呼んでから `SetBounds`(画面座標)。
4. 最後に、最初に最大化から戻していれば `f_85eb80(form, 2)` で最大化へ戻す。

`f_132ec10`(エディタ窓がまだ無い場合)も同じ座標の入れ替えをデータだけに行う: **デタッチ指定なら保存位置(`[+0x20]`、`[+0x24]`)に WorkPanel の画面原点を足し、アタッチ指定なら引き、結果がメイン窓の範囲外なら既定の (12, 96) に戻す**。→ **「位置関係の保持」はエンジンが保存位置を座標系ごと変換する実装**で、Windows が窓の相対位置を覚えているのではない。デタッチ後に FL のメイン窓を動かしたとき、デタッチ窓が追従するかは読んでいない(未確認。トグルは一度きりの変換で、追従の仕組みは見つかっていない)。

**仮想メソッド `SetParent`(vtable+0x138)の型**: RTTI の `Parent` の setter は `0xfe00…0138`(仮想、スロット 0x138)。`TCustomForm` 以下の実装は **`f_85c580`**(`TControl`/`TWinControl` は `f_61af00`)。`f_85c580` の読み:

- `FParent` が変わるときだけ働き、**元の `FParent == nil`(top-level から子になる)場合は、先に `UpdateRecreatingFlag(True)`(`f_61ac40`、`[+0xa4] |= 0x2000`)→ 仮想 `DestroyHandle`(vtable+0x200)で HWND を壊す**。次に継承の `f_61af00`(`TWinControl.SetParent`)→ `f_60a200`(`TControl.SetParent`: 旧親から `RemoveControl` = `f_611320`、新親へ `InsertControl` = `f_6111b0`)。`Parent == nil` になった場合は `f_6126e0`(`UpdateControlState`)。最後に `UpdateRecreatingFlag(False)`。
- **`RemoveControl`(`f_611320`)は子が `TWinControl` なら `f_6110a0` の後に仮想 `DestroyHandle`(vtable+0x200)を呼ぶ**。したがって**子から top-level へ戻る向きでも HWND は壊れる**。
- `DestroyHandle`(TCustomForm の実装 `f_85ff40`)→ `EnumThreadWindows`/`EnumChildWindows`(子窓を先に)→ `f_860100`(vtable+0x208)→ `f_612070` → **`DestroyWindow`**(`DestroyWindow` の呼び元は全体で 8 関数)。
- 窓は次にハンドルが要るとき(`CreateHandle` = vtable+0x1d0 → `CreateWindowHandle` = vtable+0x1e8、`TWinControl` 版 `f_611c70` が **`CreateWindowExW`**)に作り直す。`CreateParams`(`f_85f250`)が新しいスタイルと**`WndParent`**を決める(§2.5)。

結論: **このエンジンでも「`Parent` を変えると窓ハンドルが再作成される(RecreateWnd 型)」は成り立つ**(読み。確度 中〜高)。`SetParent`(Win32)の直接の呼び元は **2 関数**: `f_612820`(`ParentWindow` プロパティの setter。呼び元 8)と `f_c83ac0`(`AllocateHWnd` の窓を **`HWND_MESSAGE`(-3)** の親にする)で、後者はデタッチと無関係。`SetWindowLongPtrW(GWLP_HWNDPARENT = -8)` で書く呼び出しは**無い**(`Set` の index は -4、-12、-16、-20、-21、0 の 6 種。`Get(-8)` は 3 関数。`tools` の集計)。

### 2.3 (e) デタッチ窓は独立した top-level 窓か、オーナーを持つか(読み)

- フォームの `CreateParams`(`f_85f250`。`TCustomForm.CreateParams`)が、`FParent == nil` かつ `[+0x354]`(`ParentWindow`)== 0 のとき `WS_POPUP`(`or 0x80000000` を書く)を付け、**`PopupMode`(`[+0x610]`、`pmNone`/`pmAuto`/`pmExplicit`)で `WndParent`(= `Params.WndParent`、CreateWindowEx の `hWndParent`)を決める**。DFM の 104 フォームの `PopupMode`: **`pmExplicit` 16、`pmAuto` 5、既定(`pmNone`)83**。デタッチされるフォーム(`TEventEditForm`、`TStepSeqForm`、`TPluginForm`、`TFXForm`)は **すべて `pmExplicit`**(`forms-top.tsv`)。
- `pmExplicit` のとき `WndParent` は `PopupParent`(`[+0x614]`。`f_85e990` が setter)の窓。`f_85f250` は `PopupParent` が無いとき `Application` の窓(`[[15f7190]+0x2b0]` = TApplication の隠し窓。`CreateWindowExW("TApplication", 0x84ca0000)`。決定 0119 の壁 10)を `WndParent` にする分岐(0x85f3f9〜)を持つ。デタッチ窓のオーナーが**メインフォームの HWND か TApplication の隠し窓か**は、どちらの分岐に入るか(`PopupParent` の値)を実行して見ないと決まらない(未確認)。**いずれにせよ `WndParent != NULL` の `WS_POPUP`、すなわちオーナー付きの top-level**(`GetWindowLongPtrW(GWL_EXSTYLE)` を引いて **`WS_EX_TOOLWINDOW`(0x80)の祖先を飛ばす**走査 `f_85f1a0` がある。`GetParent` を辿るので、`GetParent` が**オーナーを返す** Win32 の規則(Learn GetParent: 「top-level で `WS_POPUP` ならオーナーのハンドル」)に依存する)。
- Learn 「Owned Windows」(Window Features): 「An owned window is always above its owner in the z-order」「The system automatically destroys an owned window when its owner is destroyed」「An owned window is hidden when its owner is minimized」。→ **TOPMOST 無しで FL のメイン窓の上に出続けるのは、オーナー関係の効果と読める**(実機で未観測)。「FL の外に出せる(常に最前面)」の「常に最前面」は、**他アプリの窓より前**ではなく**FL(オーナー)より前**の意味の可能性が高い(未確認。依頼文は「常に最前面/FL 外にウインドウを出せる」と併記)。
- **`WS_EX_TOPMOST`(0x8)を拡張スタイルに足す箇所は、フォームの `CreateParams`(`f_85f250`、`f_85da10`、`f_6118a0`)に無い**(命令の走査)。TOPMOST は **`SetWindowPos(hWndInsertAfter = HWND_TOPMOST)`** で付ける設計に見える: `f_85e110`(`SetFormStyle`)と `f_8684f0`(`RestoreTopMosts`)、`f_8682c0`(`NormalizeTopMosts`)が `SetWindowPos`(`uFlags = 0x213` = `SWP_NOSIZE|SWP_NOMOVE|SWP_NOACTIVATE|SWP_NOOWNERZORDER`。`hWndInsertAfter` は 表 `[1408bb8]` = {-2 (HWND_NOTOPMOST), -1 (HWND_TOPMOST)})を呼ぶ。`TApplication` が `FTopMostList`/`FTopMostLevel`(VCL の `TApplication` の既知のメンバーと一致)を持つ。`SetFormStyle` の呼び元 6 関数のうち、`fsStayOnTop`(3)を渡すのは 4 地点(`f_864830`、`f_bcb1f0`、`f_bcbab0`、`f_e29920`)、`fsNormal`(0)は 3 地点。**デタッチのトグルはこの関数を呼ばない**(`f_1245f30` の直接の呼び出しに `SetFormStyle` も `SetWindowPos` も無い。呼ぶのは仮想 `SetBounds`(vtable+0x188 = `f_616c10`、中で `SetWindowPos`/`SetWindowPlacement`)と `LockWindowUpdate`、`ShowWindow`(`f_85eb80` 経由)だけ)。
- 設定項目に `Keep window on top`、`Always on top`、`Keep the Plugin Manager window on top of other applications` の UI 文字列(`.data` の文字列表)がある。`TPluginListForm`(プラグインマネージャ)や補助窓(`TToolbarForm`、`TFLHintBarForm`、`TNotificationForm`、`TPlugListForm`、`TQuickPopupMenuWindow` ほか 16 フォーム)が DFM で `FormStyle = fsStayOnTop`。**これらは「デタッチ」とは別の機構**(読み)。

### 2.4 (c) スナップとその無効化の手がかり(読み。決められない点を明記)

- スナップは **FL のエンジン側の処理**(`WM_WINDOWPOSCHANGING` = `f_8076c0`〔`TCustomWPForm`〕、`f_8058b0`〔`TWPControl`〕、`WM_SIZING` = `f_8083c0`〔→ `f_807b00`〕、VCL の `ScreenSnap` = `f_8620b0`〔`TCustomForm`〕。`handlers.tsv`)。DFM の `ScreenSnap = True` のフォームは 34(`SnapBuffer = 8` は `TFXForm` とメインフォームだけ。残りは既定値)。`TWPControlOptions` の集合に `wpSnapChildren`、`wpSnapHResize`、`wpSnapVResize`(RTTI の名前列)。
- **`f_8620b0`(VCL 流の画面端スナップ)は `ScreenSnap` バイト(`[+0x63c]`)を見たあと `FParent`(`[+0x78]`)を見て、`FParent == nil` の top-level のときだけ働く**(読み。モニターの作業領域との差を `SnapBuffer`(`[+0x640]`)と比べる `f_862080`)。`f_8076c0` は途中で `ScreenSnap` を 0 にして継承を呼び、戻す(0x807921〜0x807941)。**「デタッチ窓は他窓とスナップしない」に対応する分岐(例: `FParent != nil` のとき `f_8076c0` が別の経路を取る)は、静的には見つけていない**。→ **スナップの無効化の実装は未確認**。FL のデタッチ中のスナップ無効の挙動は依頼者の観察で、静的解析で裏づけられていない。
- Windows 自体の「Aero Snap」は `SetWindowPos`/`WM_WINDOWPOSCHANGING` の外で動く OS の機能で、`DwmGetWindowAttribute` は FLEngine の import に**無い**(DWMAPI の import は `DwmIsCompositionEnabled` と `DwmExtendFrameIntoClientArea`)。→ エンジンが Aero Snap を制御する手がかりは**無い**(未確認ではなく、API の不在という実測)。

### 2.5 (d) デタッチ可能な窓の種類(読み。「全窓が対応か」は未検証のまま)

- **同じ基底の機構**: デタッチされる 4 クラス(`TStepSeqForm`、`TEventEditForm`、`TFXForm`、`TPluginForm`)は **すべて `TChildVectorForm` の子孫**で、`TChildVectorForm < TVectorForm < TUnpaintedWPForm < TCustomWPForm < TForm < TCustomForm`(`form-classes.tsv`)。`TChildVectorForm` を継承するのは他に `TGraphEditorForm`、`TNotificationForm`、`TPlugListForm`、`TBaseGlassForm`(→`TQuickPopupMenuWindow`)の計 9 クラス。**`SetParent`(`f_85c580`)、`CreateParams`、`SetBounds` は共通**(`TCustomForm` 以下)、デタッチのトグル `f_1245f30` はフォームの型を選ばない(`TCustomForm` として扱う)。→ **デタッチは基底クラスの機構で、メニュー項目が付いたフォームにだけ入口がある**。
- **入口があるのは 4 クラス(上の表)**。`TBrowserForm`、`TPluginListForm`、`TToolbarForm`、`TMIDIForm`(設定)ほかの `TVectorForm` 直系は `*DetachMenu` を持たない(DFM の走査)。ブラウザ窓は「(detached)」の語を持つ設定があるので、別の仕組みでデタッチ相当を持つ可能性がある(未確認)。
- アタッチ時の親は `WorkPanel`(メインフォームの `TWPControl`、`Align = alClient`)。メインフォームは `bsNone`(自前の枠)で、`WorkPanel` が子フォームの置き場(読み)。

### 2.6 FLEngine の窓関連 API の使用箇所(実測。直接の呼び元の関数数。`api-usage.tsv`)

`SetParent` 2、`GetParent` 18、`GetAncestor` 2、`GetWindow` 10、`IsChild` 4、`SetWindowPos` 46、`GetWindowRect` 27、`GetClientRect` 10、`ClientToScreen` 7、`ScreenToClient` 2、`MapWindowPoints` 17、`AdjustWindowRectEx` 2、`GetWindowPlacement` 16、`SetWindowPlacement` 6、`LockWindowUpdate` 7、`ShowWindow` 18、`MonitorFromWindow` 4、`MonitorFromRect` 2、`GetMonitorInfoW` 6、`EnumDisplayMonitors` 2、`GetWindowLongPtrW` 46(index: -4 6 地点、-6 1、-8 3、-12 3、-16 25、-20 26、-21 2)、`SetWindowLongPtrW` 30(-4 13、-12 3、-16 7、-20 18、-21 1、0 1)、`SetPropW`/`GetPropW`/`RemovePropW` 4/3/3、`DestroyWindow` 8、`CreateWindowExW` 8、`EnumThreadWindows` 5、`EnumChildWindows` 3、`BeginDeferWindowPos` 3、`SetForegroundWindow` 4、`SetActiveWindow` 18、`GetActiveWindow` 7、`GetCapture` 54。**`DwmGetWindowAttribute`、`SetLayeredWindowAttributes`、`UpdateLayeredWindow` は import に無い**(`goal-ladder.md` §3.5 の「`UpdateLayeredWindow`」はクラス名由来の推測と見られる。本調査の実測は import の不在)。`GetWindowRect` と `ClientToScreen`/`MapWindowPoints` がデタッチ窓以外の大半(ドラッグ、ヒント、ポップアップメニュー)で使われる。**窓の API は「デタッチ用」ではなく UI 全体の基盤**。

### 2.7 静的分析のまとめ(確度)

| 確認したいこと | 結果 | 確度 |
|---|---|---|
| (a) どの機構か | `Parent` の付け替え(VCL の `SetParent`)。HWND の破棄と再作成を伴う(`DestroyHandle` → `DestroyWindow`、のち `CreateWindowExW`)。`SetParent`(Win32)/`GWLP_HWNDPARENT`/`SetWindowPos(HWND_TOPMOST)`/`WS_EX_TOPMOST` は**トグルの中では使われない** | 呼び出し鎖は読みで固い。VCL との同一視は記憶に依る(中〜高) |
| (b) 位置の保持 | エンジンが WorkPanel の画面原点で座標系を入れ替える(`ClientToScreen`/`ScreenToClient` 相当)。追従の仕組みは未確認 | 読み(中) |
| (c) スナップ | エンジン側(`WM_WINDOWPOSCHANGING`/`WM_SIZING`/`ScreenSnap`)。`Parent == nil` のときだけ効く層がある。デタッチ中の無効化の分岐は未特定 | 読み(低〜中) |
| (d) 種類 | `TChildVectorForm` 系の共通機構。入口があるのは 4 クラス | 読み(中) |
| (e) top-level か | `WS_POPUP`(+ `WndParent != NULL`)= オーナー付き top-level。オーナー相手(メインフォームか TApplication の窓か)は未確認 | 読み(中) |
| 実機の観測 | **無い** | — |

## 3. 仮想窓モデルへの要求(設計制約): 決定 0071/0119(と 0121)の窓表に「今から」入れておくもの

### 3.1 現状(`lift/runtime/fle_win.c`、`fle_win.h`。読み)

窓表の 1 項目 = `{wndproc, hinst, style, exstyle, atom, live, x, y, cx, cy(作成時の矩形・画面座標・親なし), text, wts, icon_big/small, has_menu/menu_items, userdata, msgonly(0121)}`。ハンドルは `0x7F990000 + 4*(i+1)`、**再利用しない、削除しない**(`DestroyWindow` は未実装)。`CreateWindowExW` は 3 つの形(隠しツール窓、TApplication、メッセージ専用窓)だけ。**`hWndParent` は NULL 以外を名前付きで止める**(`HWND_MESSAGE` の形だけ例外)。`GetWindowLongPtrW(GWLP_HWNDPARENT)` は「親なし = 0」の固定。`SetWindowLongPtrW` は WNDPROC/EXSTYLE/USERDATA だけ。`ShowWindow`、`SetWindowPos`、`GetWindowRect`、`GetClientRect`、`ClientToScreen`、`MapWindowPoints`、`SetParent`、`GetParent`、`GetAncestor`、`GetWindow`、`DestroyWindow`、メッセージキューは**無い**(決定 0119 の「実装していないもの」)。→ **今の表は「親・オーナー・矩形の更新・Z 順・破棄」を持たない**。デタッチ以前に G2 のメインウィンドウ(`TApplication` の可視化)、G6 の入力(ヒットテストに窓の矩形が要る)、メニューやヒント窓(ポップアップ。`WS_POPUP` + オーナー)が同じ属性を要求する。

### 3.2 要求(一次の属性 = 今決めるもの / 後で足してよいもの)

| # | 属性・API | 一次の属性か | 今入れる理由(作り直しを避ける) | 後でよい部分 | Learn の根拠(取得した文言) |
|---|---|---|---|---|---|
| R1 | **`parent` と `owner` を別の欄で持つ**(`hWndParent` の引数は style で解釈: `WS_CHILD` なら親、`WS_POPUP`/overlapped なら owner) | **一次** | `CreateWindowExW` の第 8 引数を 1 本の「親」に入れると、`GetParent` の戻り(top-level の `WS_POPUP` ではオーナー)と `GetAncestor(GA_PARENT)`(オーナーを含まない)、`GWLP_HWNDPARENT`(オーナーの設定)が表せない。**デタッチ窓は `WS_POPUP` + オーナー**(§2.3)なので、これを 1 欄にすると、デタッチ(親 → オーナー)の遷移が欄の意味の変更になる | `SetParent` の実装、親子の `WM_PARENTNOTIFY` | GetParent: 「If the window is a child window, the return value is a handle to the parent window. If the window is a top-level window with the WS_POPUP style, the return value is a handle to the owner window.」 / GetAncestor: GA_PARENT 「Retrieves the parent window. This does not include the owner, as it does with the GetParent function.」/ SetWindowLongPtrW: 「GWLP_HWNDPARENT is used to change the owner of a top-level window, not the parent of a child window. … A window can have either a parent or an owner, or neither, but never both simultaneously.」 |
| R2 | **矩形は「親相対」で持ち、画面座標は親鎖から計算する**(top-level は画面座標) | **一次** | デタッチ = 子 → top-level で**座標系が変わる**。画面座標だけで持つと、メイン窓が動いたとき子が動かない(子は親に追従する)。親相対だけで持つと top-level の `GetWindowRect` が引けない。FL のトグルは `ClientToScreen`/`ScreenToClient` で座標系を入れ替える(§2.2) | クライアント矩形の計算(NC 領域の幅。`AdjustWindowRectEx`) | GetWindowRect: 「The dimensions are given in screen coordinates that are relative to the upper-left corner of the screen.」 / Window Features: 「The coordinates of a top-level window are relative to the upper left corner of the screen; the coordinates of a child window are relative to the upper left corner of the parent window.」/ ClientToScreen: 「The screen coordinates are relative to the upper-left corner of the screen.」/ MapWindowPoints: 「If this parameter is NULL or HWND_DESKTOP, the points are presumed to be in screen coordinates.」 |
| R3 | **窓の Z 順 = 兄弟リスト(親ごと)+ top-level リスト(TOPMOST 群が先)** と `hWndInsertAfter`(`HWND_TOP`/`BOTTOM`/`TOPMOST`/`NOTOPMOST`/窓) | **一次** | FL は `SetWindowPos(0x213, HWND_TOPMOST/NOTOPMOST)` と `EnumWindows`/`GetWindow(GW_HWNDNEXT)` を使う(§2.3、`NormalizeTopMosts`)。**Z 順は表の添字(作った順)と別に持つ**。作った順だけだと、TOPMOST への変更やオーナーの上への配置を後から入れるとき、窓表の順序を全関数が前提にしている(添字 = ハンドル = Z 順)ため作り直しになる | 実際のブラウザ上の重なり(§4)、`DeferWindowPos` | SetWindowPos: HWND_TOPMOST 「Places the window above all non-topmost windows. The window maintains its topmost position even when it is deactivated.」/ Window Features: 「The system maintains the z-order in a single list. … A topmost window has the WS_EX_TOPMOST style. All topmost windows appear in the z-order before any non-topmost windows. A child window is grouped with its parent in z-order.」 |
| R4 | **`WS_EX_TOPMOST` は exstyle のビットだが、`SetWindowPos` が更新する**(`SetWindowLongPtrW(GWL_EXSTYLE)` で直接は更新させない) | **一次**(更新規則) | 現状の `SetWindowLongPtrW(GWL_EXSTYLE)` は書いた値を即座に反映する(決定 0119 S9)。TOPMOST だけは `SetWindowPos` でしか変わらない | `SetWindowPos` の他フラグ | ExStyles: 「WS_EX_TOPMOST … To add or remove this style, use the SetWindowPos function.」/ SetWindowPos: 「When a non-topmost window is made topmost, its owned windows are also made topmost. Its owners, however, are not changed.」「A non-topmost window can own a topmost window, but the reverse cannot occur.」 |
| R5 | **窓の破棄と再作成を表す: 破棄した窓の HWND は再利用しない(今のまま)が、「同じ論理窓(Delphi のフォーム)に新しい HWND」を扱えること** | **一次** | **VCL の `SetParent` は `DestroyWindow` → `CreateWindowExW` で HWND を変える**(§2.2)。デタッチのたびに HWND が変わるので、(a) 古い HWND での `IsWindow`/`SendMessage` は失敗すること、(b) HWND を持つ canvas(ブラウザ側の描画面)を**窓の論理 id に結び**、HWND に直結しないこと。窓表の添字(= ハンドル)に描画面を結ぶと、デタッチ(再作成)で描画面が孤児になる。現状の「削除しない」は作り直し時にそのまま使える(再利用しない方針と合う) | `DestroyWindow` の連鎖(子・オーナー)、`WM_DESTROY`/`WM_NCDESTROY` の送出 | DestroyWindow: 「If the specified window is a parent or owner window, DestroyWindow automatically destroys the associated child or owned windows when it destroys the parent or owner window. The function first destroys child or owned windows, and then it destroys the parent or owner window.」/ SetParent: 「If the window identified by the hWndChild parameter is visible, the system performs the appropriate redrawing and repainting.」(= 作り直しなしでも変わる窓がある、という Windows 自体の仕様。FL は作り直す) |
| R6 | **`SetParent` が `WS_CHILD`/`WS_POPUP` を変えない** | 二次(`SetParent` を実装するとき) | FL の VCL は自分で style を作り直す(`CreateParams`)。`SetParent`(Win32)を呼ぶのは `ParentWindow` setter と `AllocateHWnd` の `HWND_MESSAGE` だけ(§2.2)。**今は `HWND_MESSAGE` の形が既に入っている(0121)** | `SetParent` 全般 | SetParent: 「For compatibility reasons, SetParent does not modify the WS_CHILD or WS_POPUP window styles of the window whose parent is being changed.」 |
| R7 | **`SetPropW`/`GetPropW`/`RemovePropW` の窓ごとの表** | 二次 | VCL は HWND に `Self` のポインタを `SetPropW` で結ぶ(`f_612190`、2 つのアトム)。窓の作成時に呼ばれる(静的な呼び元 4/3/3 関数)。**これが無いと HWND → フォームの対応が引けず、デタッチ(再作成)の後も通る** | | SetPropW: 「Before a window is destroyed (that is, before it returns from processing the WM_NCDESTROY message), an application must remove all entries it has added to the property list.」 |
| R8 | **`WM_WINDOWPOSCHANGING`/`WM_WINDOWPOSCHANGED`(`WINDOWPOS`)を `SetWindowPos` が送り、ゲストが `WINDOWPOS` を書き換えられる** | **一次**(メッセージの順序の規則) | FL のスナップは `WM_WINDOWPOSCHANGING` のハンドラ(`f_8076c0`)が `WINDOWPOS` の x/y/cx/cy を書き換える(§2.4)。**送らない設計(決定 0071/0119 の「送らない」)のままだと、スナップも「デタッチ中はスナップしない」も実現できない**。送るか否かは `SetWindowPos` の実装時でよいが、**`WINDOWPOS` の構造体の形と「戻りで反映する」を前提にしておく** | `WM_GETMINMAXINFO`、`WM_SIZE`、`WM_MOVE`(lParam = **クライアント原点**を screen 座標(top-level)/親クライアント座標(子)で) | WM_WINDOWPOSCHANGING: 「While this message is being processed, modifying any of the values in WINDOWPOS affects the window's new size, position, or place in the Z order.」/ WM_MOVE: 「The parameters are given in screen coordinates for overlapped and pop-up windows and in parent-client coordinates for child windows.」 |
| R9 | **画面(モニタ)の情報は窓表と別に持つ(既に `fle_monitor.c`: 1 モニタ、原点 (0,0)、`rcWork = rcMonitor`)が、デタッチ窓がモニタの外に出る・複数モニタの形を壊さない** | 二次 | FL は `MonitorFromWindow`、`MonitorFromRect`、`GetMonitorInfoW`、`EnumDisplayMonitors` を使い(§2.6)、設定 `Open detached plugins on the same screen as the FL Studio window`、`Don't limit windows to screen` がある。**仮想画面を 1 枚に固定すると、デタッチ窓がブラウザの別画面(§4.4)に出ても、エンジンには 1 枚の仮想画面の座標として見せる必要がある**(ブラウザの `screenX/screenY` はブラウザ窓の外の座標) | 複数モニタ | Window Management API(§4.4。ブラウザ側の話)。Win32 側は GetMonitorInfoW の `rcMonitor` 「expressed in virtual-screen coordinates」(決定 0121 が既に引用) |
| R10 | **描画面(canvas)を窓表の項目に直結せず、「top-level 窓(= ポップアップの候補)ごと」に持つ**(子窓は親の描画面に描く) | **一次** | §3.3 | ブラウザ側の方式選択(popup / PiP / 浮動パネル) | (この項は Learn の根拠ではなく、ブラウザの制約: §4.2) |

「一次」= 後から付け足すと窓表の構造(1 項目の欄の意味、添字とハンドルの対応、Z 順の持ち方)を変える。「後でよい」= API を足すだけ。**上の一次(R1〜R5、R8 の形、R10)を 1 項目の欄として今決めておけば、デタッチ自体(ポップアップ)を延期しても窓表の作り直しにならない**。窓表の添字 = ハンドル = 作成順 = Z 順 = 描画面 の 4 つ組を前提にしたコードを増やさないこと、が要約。

### 3.3 描画面を「top-level 窓」に結ぶ理由(読み + §4 の実測)

- ゲストの窓は `WorkPanel` の子窓(アタッチ中)か top-level(デタッチ後)。**描画(`WM_PAINT`、DIB → 窓の DC)の宛先は「その窓が属する top-level の描画面」**にすると、アタッチ中は主 canvas、デタッチ後は別の canvas(ポップアップ/PiP)に出せる。子窓ごとに canvas を持つ(G2〜G4 の素朴な実装)と、**子が top-level に変わるとき canvas を作り直す**。
- ブラウザ側の描画面は**窓の再作成で入れ替わる**。`OffscreenCanvas` を Worker に渡す経路(§4.2)では、**ポップアップが閉じる/アタッチに戻るとき canvas を Worker から取り戻せない**(転送は一方向。実測は §4.3 の close)。**画素バッファ(DIB)は wasm のメモリに残し、canvas は表示の出口にする**設計ならば(`putImageData`/`ImageBitmap` 送出)、出口の差し替えだけで済む。

## 4. ブラウザでのポップアップの実現性(読み + headless Chrome での実測)

### 4.1 実験の構成(`lift/harness/web/detach/`、素の HTML/JS 約 800 行、62 KB。ビルド無し)

- `index.html` + `main.js` = **オープナー**(エンジンの Worker を持つ。実エンジンでは wasm の Worker と AudioWorklet もここ)。`popup.html` + `popup.js` = **デタッチ窓**(canvas 1 枚、mousemove/keydown を送る)。`engine-worker.js` = **エンジンの代役**(FL のコードではない。512×384 のフレームを 16 ms 周期で描き、入力イベントに ack を返す)。`run-detach.mjs` = CDP の駆動(`../cdp.mjs` を読み取りで使用)。
- 経路 3 種(`via`):**`main`** = Worker → オープナー(`postMessage` で `ImageBitmap` を transfer)→ ポップアップ(`window.postMessage` で transfer)→ `bitmaprenderer`。**`port`** = オープナーが `MessageChannel` を作り、片方の端を Worker、もう片方をポップアップに渡す(以後オープナーを経由しない)。**`canvas`** = ポップアップが自分の `<canvas>` を `transferControlToOffscreen()` し、`OffscreenCanvas` を `window.opener.postMessage` で transfer、オープナーが Worker へ中継、Worker が直接 2D 描画。
- Worker のループ 2 種(`mode`): **`loop`** = フレーム間にイベントループへ戻る(JSPI で中断する実エンジンを外から見た形。`browser-event-loop.md` の M-B)、**`block`** = 開始後に `Atomics.wait` で戻らない(M-A の形)。`block` の入力は `SharedArrayBuffer` のリング + 1 本のドアベル(futex 語)で受け、`/coi/`(COOP `same-origin` + COEP `require-corp`。ポップアップにも同じヘッダ)のページで測った。
- 各測定 = 1 つの Chrome、窓 1 つ、5 秒、mousemove 300 回(CDP の `Input.dispatchMouseEvent` で約 4 ms 間隔。**合成入力**であり実マウスではない)。**3 回**(r1〜r3)の中央値を表にする。生データ `m-*.json`、集計 `measure.tsv`、`measure-median.tsv`。時刻は `performance.timeOrigin + performance.now()`(同一マシンの文書間で比較可能。5 µs 精度は `/coi/` のとき、`/plain/` は 100 µs 刻みなので表の 0.1 ms 台の差は刻みの影響を受ける。`browser-event-loop.md` §3)。

### 4.2 方式の比較

| 観点 | (A) `window.open` の独立ポップアップ + 窓ごとに canvas | (B) Document Picture-in-Picture | (C) 同一ページ内の浮動パネル | (D) Window Management API(複数画面への配置) |
|---|---|---|---|---|
| OS の別窓か | **可**(実測: 別のブラウザ窓。headless でも別ターゲットとして現れる) | **可**。ただし「ブラウザの PiP 窓」で、**1 タブ 1 窓**(MDN)、**位置をサイトが指定できない**(MDN: 「The Picture-in-Picture window position cannot be set by the website.」) | **不可**(同一ドキュメント内の `div`。ブラウザ窓の外に出せない)。読み(測定せず) | (A) への追加の機能。`getScreenDetails()` + `window.open` の `left/top` |
| 常に最前面 | **不可**(読み。headless では観測不能) | **可**(MDN: 「always-on-top window」。headless では観測不能。未確認) | 不可(ページ内の z-index) | (A) と同じ |
| ユーザー操作の要件 | `window.open` は **transient activation が要り、1 回で消費される**(実測: 無操作で拒否、1 回の合成クリックで 1 窓だけ成功、同じ活性での 2 窓目は拒否) | `requestWindow()` は transient activation 必須(実測: 無操作で `NotAllowedError`、クリック後は成功) | 不要 | `getScreenDetails()` は**権限**(実測: headless で既定は `NotAllowedError: Permission denied`、CDP で権限付与後は成功、画面数 1) |
| 同時に開ける数 | 活性 1 回につき 1(実測: 3 窓は 3 回のクリックで開いた) | **1**(実測: 活性を与えて 2 つ目を要求すると解決するが、**1 つ目は 600 ms 以内に閉じて `pagehide` が出る**) | 制限なし | (A) と同じ |
| ポップアップブロック | 活性が約 5 秒で失効する(下の 4.3)。FL のデタッチ操作が**メニュー操作 → ゲストのメッセージ処理 → ホスト呼び出し**の鎖で 5 秒以内ならよいが、**起動時の「デタッチ状態の復元」「全プラグインをデタッチ」(複数窓を 1 操作で)は拒否される** | 同上(窓は 1 つ) | なし | — |
| 同一オリジン条件 | オープナーと同一オリジンの URL を開く。**COOP/COEP(`SharedArrayBuffer` のため)は同じヘッダをポップアップにも付ける必要**(実測 4.3) | 空の同一オリジン窓(MDN: 「similar to a blank same-origin window opened via Window.open()」)。ポップアップ側の文書は自分で作る。`Window` オブジェクトを直接触れる | — | — |
| 描画の出口 | popup の `<canvas>`。経路は 4.3 | PiP の `document` に canvas を作る。**同一レルム**で JS を実行できるため、オープナーの main thread から直接 `ImageBitmap` を渡せる | 主 canvas の上に重ねる | (A) と同じ |
| 閉じたとき | `pagehide`/`beforeunload` がポップアップ側で出る。オープナー側は `popup.closed` を見る(4.3)。**自動で「アタッチに戻す」はされない**ので、エンジン側が検知して戻す | `pagehide` が出る(実測)。MDN: 「The Picture-in-Picture window never outlives the opening window」 | — | — |
| ハイ DPI | 窓ごとに `devicePixelRatio` が違いうる(実測は CDP のエミュレーションのみ: ポップアップだけ 2、オープナー 1) | 同上(未測定) | ページと同じ | 画面ごとの `devicePixelRatio` を `ScreenDetailed` で読める(MDN。実測は 1 画面だけ) |
| 規模(桁) | 小〜中(窓表 + 描画面 + 入力の送り先) | 小((A) の差し替え) | 小 | 小((A) に画面選択を足す) |

### 4.3 実測(headless Chrome 156、隔離コンテキスト。表は 3 回の中央値)

**(1) 経路と Worker のループ形ごとの描画・入力(512×384、5 秒、mousemove 300 回)**

| 経路 | Worker のループ | ポップアップが受けたフレーム / Worker が送った数(3 回) | 表示間隔から求めた fps | スクリーンショットから求めた fps | Worker→ポップアップ到達 p50 / p99 (ms) | 入力往復 p50 / p95 / p99 (ms)(最大) | 入力 受理/送信 |
|---|---|---|---:|---:|---|---|---|
| `main`(オープナー経由) | `loop`(戻る) | 629〜630 / 同数 | 59.9 | 56.6 | 0.30 / 0.70 | 1.10 / 1.50 / 1.80(11.0) | 301/301 ×3 |
| `main` | `block`(`Atomics.wait`)、入力は SAB リング | 657〜659 / 同数 | 62.5 | 60.8 | 0.31 / 1.04 | 0.68 / 0.88 / 1.12(10.7) | 301/301 ×3 |
| `port`(MessageChannel 直結) | `loop` | 627〜630 / 同数(±1) | 59.9 | 56.4 | 0.20 / 0.40 | 0.50 / 0.70 / 0.90(10.6) | 301/301 ×3 |
| `port` | `block`、入力は SAB リング | 657 / 同数(±1) | 62.5 | 60.1 | 0.20 / 0.72 | 0.52 / 0.80 / 1.43(10.7) | 301/301 ×3 |
| `port` | `block`、入力を **port のメッセージ**で送る | 657〜659 | 62.5 | 60.1 | 0.16 / 0.55 | **届かない(0/301 ×2)** | 0/301 |
| `canvas`(ポップアップの OffscreenCanvas に Worker が直接描く) | `loop` | (ポップアップ側で数えられない)/ Worker 643〜648 | — | **61.2**(表示されている) | — | 0.70 / 1.0 / 1.4(10.4) | 301/301 ×3 |
| `canvas` | `block` | Worker 650〜654 | — | **表示されない**(ポップアップは黒のまま。3/3 回、6 枚ずつのスクリーンショットで画素 0) | — | 0.67 / 0.88 / 1.09(10.8) | 301/301 ×3 |

- 値の読み方: **フレームは落ちていない**(ポップアップが受けた数 = Worker が送った数、差は最大 1)。**生成側が 16 ms 周期の代役なので fps は伝送の能力値ではない**(能力を測るには周期を詰めたテストが要る。未実施)。表示間隔 p99 は 17〜25 ms。到達 p50 の 0.1 ms 台は `/plain/` のタイマー刻み(100 µs)の影響を受ける(`/coi/` は 5 µs)。
- 入力往復の外れ値は最大 約 10 ms(各回に 1 件前後)。ほかの 299 件前後は 1〜2 ms 以内。`browser-event-loop.md` §3 の「ページ ↔ Worker 約 0.1 ms」に比べて、**ポップアップ → オープナー(main thread)→ Worker の 2 ホップのぶん 0.5〜1.1 ms**。
- **ポップアップの `MessagePort` に渡した入力は、`Atomics.wait` で戻らない Worker には届かない**(0/301 ×2。ポートのメッセージはイベントループが回らないと処理されない)。**入力を直接 Worker に届ける方法は SAB リング + 1 本のドアベル(futex 語)**で、ポップアップが `Atomics.notify` する(実測: 301/301)。**これは COOP/COEP(`crossOriginIsolated`)を要する**。リングを書く側ごとに別リング(単一生産者)にした。
- 窓をエンジン起動後に登録(`late=1`): `main`/`block` 656〜661 フレーム、`port`/`loop` 631 フレーム。**実行中にデタッチした窓でも落ちない**。
- `window.open` → ポップアップの `hello` 到達 **中央値 43.8 ms**(最小 36.6、最大 87.3、n = 20)。

**(2) Document PiP(同じ代役、400×300)**: `loop` で 568 フレーム(表示間隔 59.9 fps、スクリーンショット 57.5 fps)、Worker→PiP p50 0.20 ms / p99 0.30 ms、入力往復 p50 0.50 ms / p99 0.90 ms(255/255)。`block`(SAB リング)で 591 フレーム、62.5 fps、p50 0.18 ms / p99 0.65 ms、往復 p50 0.52 / p99 1.40 ms(255/255)。**ポップアップと同じ性能**で、PiP の JS は**オープナーと同一レルム(`opener` あり、同一オリジン文書)で動く**ので、`postMessage` の中継が要らない。**headless で動いた**(合成クリック後)。位置と最前面は観測できない。

**(3) ポップアップと main thread の関係**: オープナーの main thread を 300 ms ずつ塞ぐ(500 ms 周期、約 4 秒)と、**`main` 経路だけでなく `port` 経路(Worker → ポップアップの直結)でもポップアップの表示間隔が最大 317 ms になる**(`main` 17 ms/317 ms、`port` 17 ms/317 ms(p50/最大))。CDP のスクリーンキャストで表示を数えると、**`main` 経路は塞がれている間 22.8 fps(アイドル 59.7)に落ち、`canvas` 経路(Worker が直接描く)は 60 fps のまま**。レンダラのプロセス数は窓を開く前後で 7 → 7(**同一プロセス**)。→ **同一オリジンの `window.open` のポップアップは、オープナーと同じレンダラプロセスで動き(プロセス数 7 → 7。ただし数えたのは `--type=renderer` のプロセス数で、どのプロセスがどの窓を持つかは見ていない)、`main`/`port` 経路では main thread が塞がると表示も止まる**(実測は上の表示の停止。「main thread を共有する」は読み)。**オープナーの main thread で重い処理(`ImageBitmap` の生成、音声のメッセージ処理)をするとポップアップも止まる**。`canvas` 経路は main thread に依存しない(ただし `block` ループでは表示されない)。

**(4) COOP**(`crossOriginIsolated` 付きのオープナーから開く。実測): ポップアップにも **同じ COOP `same-origin` + COEP `require-corp`** を付けると、`window.opener` が残り、ポップアップも `crossOriginIsolated`、`SharedArrayBuffer` が使える。**ポップアップの文書に COOP を付けない**(`unsafe-none`)と、**別の browsing context group になり、`window.opener` が切れ、オープナー側の `popup.closed` が `true`、`popup.document` は `SecurityError`**(`postMessage` も届かない)。`same-origin-allow-popups`(COEP 付き)は `crossOriginIsolated` にならない(この構成では `SharedArrayBuffer` が使えない)。→ **エンジンのホストページのヘッダを `same-origin` + `require-corp` にし、ポップアップにも同じヘッダを付ける**。`file:` や別オリジンのポップアップは使えない。

**(5) 閉じたとき**: `popup.close()` でオープナーの `popup.closed` は直ちに `true`、ポップアップの `pagehide` と `beforeunload` から `closed` メッセージが 2 通届く(8 ms 差)。「ユーザーが閉じた」相当(CDP の `Target.closeTarget`)でも `popup.closed` が `true`、`closed` メッセージ 1 通。**閉じた後も Worker は port/relay にフレームを送り続ける**(ポップアップが無いので捨てられる。`close.json`: 84〜96 フレーム)。→ **エンジン側が `closed` を受けて窓を「アタッチに戻す」(`Parent := WorkPanel` 相当)処理を呼ぶ**必要があり、ブラウザは何もしない。`beforeunload` は信頼できる通知ではない(ブラウザのクラッシュ・強制終了では出ない。読み)ので、オープナーが `popup.closed` を**定期的に**見る保険も要る(未測定)。

**(6) 位置・大きさ・DPR(headless の窓ジオメトリは模擬で、window manager は無い)**: `window.open(.., 'left=123,top=77,width=512,height=384')` → ポップアップの `screenX/screenY` = 123/77、`outerWidth/outerHeight` = 512/418(内側 512×383)。オープナーから `moveTo(300, 200)` → `screenX/screenY` = **288/182**(差の原因は未確認)、ポップアップ自身の `moveTo(40, 30)` → 40/30、オープナーから `resizeTo(640, 480)` → 640×480。`getScreenDetails` で作業領域の座標を取って `left/top` に使うと指定どおり(200/100)。**ブラウザには「窓が動いた」イベントが無い**(MDN の `Window` のイベント一覧に `move` 系が無い。31 個を確認)ので、**ポップアップの位置の変化は `screenX/screenY` のポーリング**でしか追えない(ポップアップ側の `mousemove` の `event.screenX − event.clientX` でクライアント原点は求まる。headless では `window.screenX + (outer−inner の左右の差/2)` と一致: 150/125)。**FL の「メイン窓との位置関係の保持」は、ブラウザの窓位置(`screenX`)を窓表の画面座標に写す変換層を自前で持つ**必要がある(ブラウザの `screenX` はブラウザ窓の外の OS 座標で、ゲストの仮想画面ではない)。`devicePixelRatio` は窓ごとに違いうる(CDP のエミュレーションでポップアップだけ 2)。実 2 画面は未測定。

**(7) ユーザー操作の有効時間**: クリックから `window.open` までの待ち 0、1000、3000、4500 ms は成功(`navigator.userActivation.isActive = true`)、**5500、7000 ms は拒否**(`isActive = false`)。Worker からオープナーへ要求を中継しても同じ(10、1000、4000 ms 後は成功、5500 ms 後は拒否。Worker → main の遅れは約 20〜36 ms)。→ **約 5 秒(4.5〜5.5 秒の間)**。MDN は「Transient activation expires after a timeout」と数値を書かないので、実測の値。

**(8) 複数窓と音声**: 3 窓を 3 回のクリックで開き、1 つのエンジン Worker から全窓へ(`main` 経路 225/225/225 フレーム、`port` 219〜220、`canvas` 経路 Worker 241)。**オープナーの `AudioContext`(48 kHz)は窓を開いている間も閉じた後も `running` で、`currentTime` が実時間と一致**(4.67 s / 4.67 s、窓を閉じた後 6.21 s / 6.20 s)。ただし**出力デバイスが無く(`--mute-audio`)、アンダーランの測定ではない**。AudioWorklet はオープナーに居るので、ポップアップの開閉の影響は受けない(読み。上の `currentTime` の一致は整合する結果)。

### 4.4 「常に最前面」と Window Management API

- **PiP 以外に常に最前面を実現する手段は無い**(読み。MDN の `window.open` の `popup` 機能に最前面の指定は無い。Document PiP の説明が「The additional app window doesn't always stay on top, and can therefore get hidden by other windows」と `window.open` との違いに挙げる)。→ 「FL の外に出して常に前面」を完全に再現するなら PiP(1 窓のみ)、複数窓なら `window.open` の窓は**ほかのアプリの後ろに隠れる**。FL のデタッチ窓が実機で TOPMOST でないなら(§2.3 の読み)、この差は小さい。
- Window Management API(Chrome のみ、実験的。MDN: 「Limited availability」「Experimental」): `getScreenDetails()` で画面の矩形・`devicePixelRatio`・`isPrimary` が取れ、`window.open`/`moveTo` で任意の画面に置ける(実測は 1 画面で、`left/top` が指定どおり)。**権限ダイアログが出る**(headless では `NotAllowedError: Permission denied`)。ゲストの設定「Open detached plugins on the same screen as the FL Studio window」(§2.1)に対応するには、ポップアップを**オープナーのいる画面**に開く(`ScreenDetails.currentScreen`)。複数画面での実測は無い。

### 4.5 headless で分からないこと

window manager が無いため、**最前面(z 順)、実窓の位置の動き(ユーザーのドラッグ)、窓間のフォーカス移動、キーボードフォーカスの奪い合い、ブラウザ窓の装飾(タイトルバー)の有無による座標の差、複数画面、実マウスの粒度**は観測していない。**headed の Chrome は使っていない**(画面の無い環境)。PiP と `window.open` は headless でも開けた(§4.3)。合成クリック(`Input.dispatchMouseEvent`)が実ユーザー操作と同じ transient activation になるかは、`navigator.userActivation.isActive` が `true` になったこと(実測)から「ブラウザは活性とみなした」までしか言えず、実ユーザーの挙動(権限・ポップアップブロックの UI の違い)は未確認。Firefox/Safari は未測定(MDN: Document PiP と Window Management API は Chrome 系のみの機能。「Limited availability」)。

## 5. JSPI/ブロッキングループとの整合(読み。§4.3 の実測を引く)

複数の browsing context からの入力を、1 つのエンジンのメッセージキューに入れるときの注意:

1. **ポップアップの DOM イベントはオープナーの main thread を経由する**(ポップアップの JS はオープナーと同一プロセス・main thread を共有する挙動)。入力の経路は (i) ポップアップ → `opener.postMessage` → オープナー → Worker、(ii) ポップアップ → 直接 SAB リング(`/coi/` のとき)、(iii) ポップアップ → `MessagePort` → Worker。**JSPI(`loop` の形)なら (iii) が使える**(実測: 301/301)が、**`Atomics.wait` の `block` 形では (iii) は処理されない**(0/301)。JSPI の中断中は Worker のイベントループが回るので `onmessage` は届くが、**ゲストの途中状態(`R[]`)の上でハンドラが動かないよう、ハンドラはキューへの書き込みだけに限る**(`browser-event-loop.md` §4.3)。
2. **順序**: 窓ごとの入力列は FIFO(単一のリング/ポートなら保たれる)。**窓をまたぐ順序**は、経路が違うと(ポップアップ A は SAB、B は port)保証されない。`WM_MOUSEMOVE` の合体や `WM_KEYDOWN` の順序は**単一のキューに時刻つきで積む**設計にする(`browser-event-loop.md` §3.3 の「押下を捨てない」規則: 満杯でも押下を捨てない)。ポップアップごとに別リングにすると**合流の順序が決まらない**(実験は 1 窓だけを測った)。
3. **窓ハンドルの対応づけ**: イベントに「窓の論理 id」を付ける。HWND はデタッチ(再作成)で変わる(§2.2)ので、**HWND を送り先に埋め込まない**。ポップアップが閉じる・アタッチに戻る間に飛んでいるイベントは、古い id の窓へ届く(捨てるか、新しい HWND に写すか)。
4. **座標**: マウスイベントはポップアップのクライアント座標で来る。ゲストは `WM_MOUSEMOVE` に**窓のクライアント座標**を、`GetCursorPos`/`ClientToScreen` に**仮想画面の座標**を期待する(FL は `GetCursorPos` を 72 関数から呼ぶ。§2.6)。変換には**ポップアップの画面位置**(`screenX/screenY` と枠の厚み)が要るが、**ブラウザは位置変化のイベントを出さない**(§4.3(6))。ポップアップ上のマウスイベントごとに `event.screenX − event.clientX` で原点を更新すれば、**ポップアップ上にカーソルがあるあいだの座標は正確**(実測: headless で一致)。**ポップアップの外へ出たカーソル(ゲストのドラッグでの `SetCapture` 中)の座標は、ポップアップが `mouse capture` を持っていれば取れる(`setPointerCapture`。未測定)。窓をまたぐドラッグ(プレイリストから別窓へ)は未検証。
5. **フォーカスとキーボード**: キー入力は**フォーカスを持つ窓だけ**に届く。ゲストの `GetFocus`/`SetFocus`/`GetActiveWindow`(§2.6: `GetFocus` 12、`SetFocus` 28、`SetActiveWindow` 18)に対応して、**ブラウザのフォーカスの窓 → エンジンのアクティブ窓**の通知(`window.focus`/`blur`、`document.hasFocus()`)が要る。headless では窓間のフォーカス移動を観測できない。IME の入力もフォーカス窓の `<textarea>` 経由(`browser-event-loop.md` §9)で、ポップアップごとに必要。
6. **ブロッキング待ちは Worker 側**(`GetMessageW` 等)で、**ポップアップの数は待ちの機構に影響しない**(JSPI の中断は 1 本のゲストスタック)。ただし**モーダルなネスト**(`MessageBox`、`TrackPopupMenu`、`DoDragDrop`)の間に別窓のイベントが来る: Windows では**モーダルの外の窓は入力を受けない**(`EnableWindow(FALSE)` のオーナー)。ポップアップ側でも、その窓の入力を止める必要がある(窓表の `enabled` 属性。FL は `EnableWindow` を 3 関数、`IsWindowEnabled` を 11 関数が使う)。
7. **クリックの活性の連鎖**: `window.open` はユーザー操作の約 5 秒以内に**メインスレッドから**呼ぶ必要がある(§4.3(7))。**ゲストのメッセージ処理が重く 5 秒を超えると**(`WM_COMMAND` → デタッチ → `SetParent`)、ポップアップが拒否される。**活性は DOM イベントのハンドラの中でしか確認できない**(`navigator.userActivation.isActive`)ので、ホストの `SetParent` 相当の呼び出しで「活性が切れていたら浮動パネルに落とす」フォールバックを持つ(§6)。

## 6. 優先度と着手時期の材料(決めない)

### 6.1 段階表(`goal-ladder.md` G2〜G6)のどこで必要になるか

| 段階 | デタッチに関わる内容 | 備考 |
|---|---|---|
| G2(メインウィンドウの作成) | 窓表の**親/オーナー、矩形(親相対と画面)、Z 順、`SetWindowPos`/`ShowWindow`/`GetWindowRect`**(決定 0119 が「実装していない」に挙げたもの)。**ここで §3.2 の一次の属性を決める**と、デタッチの有無にかかわらず再設計を避けられる | メインフォーム(`TFruityLoopsMainForm`、`bsNone`)と `WorkPanel`、子フォーム(`TStepSeqForm` ほか)が最初から親子窓。**アタッチ状態の描画だけでも親子の矩形と Z 順が要る** |
| G3(メッセージループ) | `WM_WINDOWPOSCHANGING/CHANGED`、`WM_SIZE`、`WM_MOVE` の送出順(R8) | 既に「送らない」と決めた範囲(決定 0071/0119)の見直しが入る |
| G4/G5(描画) | **描画面の持ち方(R10)**。top-level 窓ごとの描画面。DIB は wasm のメモリ、canvas は出口 | ここで子窓ごとの canvas にすると、デタッチ時に作り直しになる |
| G6(入力) | 窓ごとの入力の送り先(**窓の論理 id**)、`GetCursorPos`/`ClientToScreen` の座標変換、`SetCapture`、フォーカス(§5) | ポップアップを後で足すとき、入力の経路(ポップアップ → オープナー → Worker)を足す。窓の論理 id を最初から持てば、足すだけ |
| G7 以降 | プラグイン窓(G9。`TPluginForm`)が**デタッチの主用途**(「Detach all new plugins」が設定にある)。プラグインの GUI がブラウザで 1 つの canvas に収まらない(外部プラグイン G11)ときの窓の置き場 | |

### 6.2 前倒しするとコストが小さい項目 / 後でよい項目

**前倒しのコストが小さい(窓表の設計時、G2 で)**: 親とオーナーを別の欄にする(R1)、矩形を親相対で持ち画面座標を計算する(R2)、Z 順を兄弟リストにする(R3)、TOPMOST は `SetWindowPos` で更新する(R4)、HWND を窓の論理 id と別にする(R5)、描画面を top-level 単位にする(R10)、`WINDOWPOS` の形(R8)。**どれもデタッチ専用ではなく、メインウィンドウと子フォームのアタッチに要る**。コストの桁: 窓表の欄の追加と API の骨格で**数百行**(`fle_win.c` は 729 行。親子/Z 順/矩形の API で同じ桁を足す。`goal-ladder.md` §3.6 の「数千行規模」の一部)。

**後でよい**: ポップアップ自体(`window.open`/PiP の開閉、入力の転送、`closed` の検知、位置変換の層)。桁: **小〜中(数百〜千行規模。窓表の一次の属性が決まっていれば)**。`SetParent` の実装(VCL のトグル)は、フォームの `Parent` の変更で `DestroyWindow` + `CreateWindowExW` が窓表に来るので、**窓の破棄・再作成(R5)ができれば足りる**(新しい API は不要。VCL が自前で行う。読み)。

### 6.3 延期した場合に今から守る制約(要約)

1. 窓表の添字 = HWND = 作成順 = Z 順 を前提にするコードを増やさない(R3、R5)。
2. `hWndParent` を 1 欄の「親」にしない。親とオーナーを分ける(R1)。
3. 矩形を作成時の画面座標だけで保存しない(R2)。決定 0119 の「作成時の矩形をそのまま保存する」を、親相対で持つ形に直せる余地を残す。
4. 描画面(canvas)を HWND に結ばない。top-level の窓ごと、論理 id に結ぶ(R10)。
5. `WM_WINDOWPOSCHANGING` を永久に「送らない」と固定しない(R8)。
6. COOP/COEP(`same-origin` + `require-corp`)のヘッダをホストページに付ける前提を保ち、ポップアップに同じヘッダを付けられるようにする(§4.3(4))。M-A(`Atomics.wait`)の保険を使う場合は入力が SAB リング + ドアベルに限られる。
7. ポップアップの `closed` を「アタッチに戻す」に結ぶ入口(エンジンの API)を空けておく(§4.3(5))。

## 7. 未決の点

1. **「常に最前面」の意味**: FL のデタッチ窓が実機で `WS_EX_TOPMOST` か(読みでは**オーナー付き `WS_POPUP`**で、他アプリの上ではなく FL の上)。実機の観察(Spy++ や `GetWindowLong` でスタイルを見る)で決まる。**実機を持つ依頼者への依頼事項**(X5 の延長)。TOPMOST なら PiP(1 窓)しか手段が無く、オーナー相当なら `window.open` で足りる。
2. **起動時・一括のデタッチの扱い**: ユーザー操作 1 回につき `window.open` は 1 窓、約 5 秒で失効するため、**プロジェクト読み込み時のデタッチ復元と「全プラグインをデタッチ」は複数窓を一度に開けない**。(i) ポップアップブロックの許可をユーザーに頼む(MDN の例示)、(ii) 初回は浮動パネルに落として、ユーザーのクリックごとに順にポップアップ化、(iii) 1 窓だけ開いて残りは浮動パネル、のどれにするか(仕様の決定)。
3. **デタッチ窓の実現方式**: (A) `window.open`(複数窓、最前面なし)、(B) PiP(1 窓、最前面、位置を指定できない)、(C) 浮動パネル(OS の窓ではない)。要求は「独立したポップアップウインドウ」なので (A) が第一候補と読めるが、**複数窓と最前面のどちらを優先するか**。
4. **メイン窓との相対位置の保持**: ブラウザは窓位置の変化イベントを出さず、ブラウザ窓の外の OS 座標を返す。**FL のデタッチ後の追従**(メイン窓が動いたとき追従するか)は実機で未確認(§2.2)。実機の観察が先。
5. **JSPI が使えない場合の保険(M-A)を持つか**(`browser-event-loop.md` §12 から継続): 持つ場合、ポップアップの入力は `MessagePort` で届かず、SAB リング + COOP/COEP が必須になる(§4.3(1))。
6. **スナップの扱い**: 「デタッチ中はスナップを無効」の分岐を静的に特定できていない(§2.4)。ブラウザの別窓は OS 側のスナップ(Aero Snap 相当)がブラウザ窓に効く。FL のエンジン内スナップ(`ScreenSnap`)は窓表の矩形が正しければ動く(`Parent == nil` のときだけ効く層があるので、デタッチ = `Parent` を外す動作で自然に有効化されるはずで、**「デタッチ中は無効」ではなく逆**に見える。実機の観察と食い違う)。実機で確認するべき。

## 8. 限界・未確認・訂正

1. **実機 Windows での FL のデタッチの観測は無い**。上の「デタッチの実体」は逆アセンブルと DFM/RTTI の読みで、**実行していない**(FRUITY の実行は別の作業が使用中)。デタッチ時に実際に `WS_POPUP` + どのオーナーになるか、TOPMOST か、スナップが無効になる分岐、メイン窓追従は**未確認**。
2. **VCL との同一視**: `TCustomForm.SetParent`、`DestroyHandle`、`CreateParams` の形は Delphi の VCL の記憶と RTTI の名前から対応づけた。Embarcadero のソースは参照していない。命名(`f_85c580` = `SetParent` など)は RTTI の published プロパティの setter の表(`Parent` の set = vtable+0x138)で裏づけたが、各関数の内部の意味は読み。
3. **「ブラウザの別窓」の実測は headless のみ**。headed の Chrome、Firefox、Safari、実 2 画面、OS の最前面、窓の装飾、実マウス・実キーボード、実 GPU は未測定。**合成クリックで `window.open`/PiP が開けること**は、実ユーザーのクリックとブロック UI の挙動が同じとは限らない。
4. **この機械の Chrome の既定コンテキストで `http://127.0.0.1` が読み込めなかった**(原因不明。約 40 通りのフラグ、net-log、サーバー側の生ソケットで調べた。要求は TCP 接続までは行くが HTTP 要求が送られず、ナビゲーションが `Cert verifier changed` のソケット閉鎖でキャンセルされる様子までは見えた)。隔離コンテキストでは読めたので、**全測定は隔離コンテキストで行った**。先行調査(`browser-event-loop.md` の 2026-10-29)の測定は同じ Chrome 156.0.8078.47 で通っているので、この機械の環境が変わった可能性がある(パッケージ更新の保留 `156.0.8078.79`、再起動、ネットワーク設定のいずれか。未確認)。測定の結果に影響する可能性は、隔離コンテキストが既定のコンテキストと同じ描画・タイマー経路か(未確認)。
5. **代役のエンジンは 16 ms 周期**: fps は伝送の能力値ではない。遅延(到達 p50 0.2〜0.3 ms)は能力の目安として読めるが、**実エンジンが main thread を重く使う**とポップアップも止まる(§4.3(3))。
6. **FL のコードではなく、実 wasm でもない**。JSPI での中断を実エンジンで測っていない(`goal-ladder.md` §4.2 U4)。ポップアップが JSPI の中断中のエンジンに入力を渡す形は `loop` の代役で見た。
7. **自分で訂正した誤り**: (a) 最初の `port` 経路のテストは popup のレポートを MessagePort 経由で返し、Worker が受けて届かなかった → レポートは常にオープナー宛にした。(b) `block` 形で SAB が popup に届かなかったのは、`window.open` 直後に SAB を送ったため(about:blank の文書が差し替わる前に送ると失われる)→ `hello` の後に送る形にした。(c) `block` ループは最初、1 本のリングの書き込み位置を待ったため、popup のリングの書き込みで起きなかった → ドアベル 1 本に変えた(最終の数値はこの後の測定)。(d) `Atomics.wait` の期待値を 0 ではなく読み取り済みの数にした(取りこぼし)。(e) 最初の `canvas`/`loop` のテストで popup 側のフレーム数が 0 になったのは、描画が Worker 側で popup が数えないため(正しい)。(f) PiP の 2 窓のテストは最初、活性を 1 回しか与えず「2 つ目は拒否」と読めたが、活性を 2 回与えると「2 つ目は解決し、1 つ目は閉じる」(§4.2)。(g) 取得ファイル `window.txt` の `move` の検索は最初「あり」と出たが、本文の「move ahead」の語だった。イベント名を 31 個列挙して「move 系なし」とした。
8. **スナップの無効化**: 上の §2.4 と §7(6)。静的には裏づけられていない。
9. **Learn の取得**: `wm-parentnotify` は 404(別の URL。使っていない)。Learn の記述は 2026-10-30 の取得で、引用した文言は `docs/investigations/detach-window/sources.tsv` の URL にある。

## 9. 規則違反の自己申告

- **AGENTS.md の「作業は task agent を通じて」**: 私は task ツールを持たないサブエージェントなので、調査・分析・実験・文書作成をすべて自分で行った。
- **cargo/emcc/clang のビルドはしていない。FL Studio のバイナリは実行していない。git の変更系操作はしていない。commit していない。**
- **ワークツリーに作ったもの(追跡外の新規)**: `docs/investigations/detach-window.md`、`docs/investigations/detach-window/`(TSV/JSON/小さなスクリプト、約 0.3 MB)、`lift/harness/web/detach/`(試作 62 KB)。**既存の追跡ファイルの変更は `docs/investigations/goal-ladder.md` の 1 節の追記だけ**(下の確認)。他者の未追跡物(`docs/investigations/README-0101-line.txt`、`lift/iat-unsup-closure.txt`、`docs/investigations/thread-model/`)は触れていない。`lift/out-x2/`、`lift/out-fruity7/` は触れていない・読んでいない(`ps` の一覧に別の作業のビルドのプロセスが見えたが、プロセスを操作していない)。
- **`/tmp/opencode/detach/`**: 作業ディレクトリ(最終 約 114 MB。200 MB 以内。FLEngine の索引の pickle 4 MB、Chrome の一時プロファイル(測定ごとに削除)、取得した Learn/MDN のテキスト、探索用の小さなスクリプト)。`/tmp` 直下には書いていない(`/tmp/opencode/detach/` の中だけ。ただし `ps`/`ss` の出力を読んだ)。
- **Python**: すべて `PYTHONDONTWRITEBYTECODE=1`。標準ライブラリと衝突するファイル名: **最初に `dis.py` を作り、capstone の `import dis` と衝突して壊れたので、直ちに削除し `fdis.py` に改名した**(自分の物だけ。`/tmp/opencode/detach/work/`)。`__pycache__` は残っていない(確認済み)。
- **Chrome**: 専用の一時プロファイル、localhost のみ。ただし**原因調査の途中**で、フラグなし(`--host-resolver-rules` 無し)の Chrome を起動した際に**Chrome 自身のバックグラウンド通信**(variations、component update、safe browsing、Google の各ホストへの接続)が net-log に出た(原因調査で、この設定なしの Chrome を数十回起動した。回数は数えていない。net-log を取った起動では Google 系のホスト(www.google.com、update.googleapis.com、clients2.google.com ほか)への要求が出た)。**意図した通信ではない**(実験の内容ではなく、Chrome が起動時に行うもの)。その後、`--disable-background-networking` ほかと `--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1` を `run-detach.mjs` に入れ、net-log で `127.0.0.1` 以外への TCP 接続試行が 0 件(1 回の測定で確認)であることを確認してから、**最終の測定をすべて取り直した**。それ以前の測定の JSON は削除した(`rm` は `docs/investigations/detach-window/` の自分の JSON だけ)。資格情報、ブラウザの既存プロファイルには触れていない(`HOME` を一時的に変えて試したが、読んだのは `~/.pki/nssdb` の `ls` だけ)。`google-chrome` の既存プロセスは操作していない。
- **外部ネットワークの取得**: Microsoft Learn と MDN/Chrome のドキュメントの HTML の取得(`urllib`)のみ。ブラウザ実験は外部ネットワーク取得なし。
- **製品の機械語の引用**: 本文と成果物に機械語・長い文字列の引用は無い(関数 VA、レジスタ/オフセットの要約と、エンジン自身の UI 文字列の短い断片のみ)。インストーラは開いていない。
- **`docs/investigations/goal-ladder.md` の編集**: 末尾への 1 節の追記だけ(全体の上書きではない)。

## 10. 成果物の場所

- 本文: `docs/investigations/detach-window.md`
- 補助: `docs/investigations/detach-window/`(`measure.tsv`、`measure-median.tsv`、`other.tsv`、`api-usage.tsv`、`forms-top.tsv`、`form-classes.tsv`、`detach-closure.tsv`、`handlers.tsv`、`sources.tsv`、`m-*.json` ほか実測の生データ、`summarize.py`、`tools/`)
- 試作: `lift/harness/web/detach/`(`README.md`、`index.html`、`popup.html`、`main.js`、`popup.js`、`engine-worker.js`、`run-detach.mjs`)
