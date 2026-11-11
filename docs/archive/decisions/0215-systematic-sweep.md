# 決定 0215: 画面の全域・全入力種別の機械的な走査(sweep1)— 停止する操作の割合と壁の全体像、最初の修理

状態: **暫定**(2026-11-04、作業者 sweep1、基準 HEAD 55e4e83 の `git archive`)。進捗の誠実性: **実エンジン(FLEngine_x64.dll と同梱 5 DLL を lift した C を native -O0 で動かし、fork したプローブに入力を注入した観測)だけ**を書く。**ブラウザでの実行ではない**(ship3 で確認する)。代役・画像の再生・推測は含まない。実機 Windows の挙動の推測は『推測』『UNOFFICIAL』と書く。

## 0. 冒頭三点

1. **何を実測したか。** 構成 B0(HEAD 55e4e83 + `ship2-imports.txt` + `dec1-imports.txt` + `api2-imports.txt`、act1/wrp の行は入れない)の native -O0 fork プローブで、**窓の格子・ドラッグ・キー・メニューの走査操作 8,552 件**(48 状態: 起動ウィザード、Basic 既定、チャンネル設定、F7/F8/F10 の窓、Settings の 10 タブ、メニュー 8 + サブメニュー 18、Browser、Sampler.flp)と、依頼者の再現 4 件のアンカー 29 件を実行した。結果: **正常 8,484、停止 59、CPU 90 秒超(タイムアウト)2、エンジン自身の終了 7(閉じるボタン・File>Exit。正常な終了であり停止に数えない)。操作に対する停止率 0.71%(61/8,552)。** 停止は**13 種類の壁**に分かれ、件数の 41%(25)が `shell32!DragAcceptFiles`(Sampler 窓とチャンネル設定窓のタブ列。依頼者の再現 (a))、34%(21)が `UpdateLayeredWindowIndirect` の `prcDirty`(サブメニュー窓)、残りは 1〜5 件ずつ。
2. **結論。** (i) 『何をすれば落ちる』は少数の窓・少数の API に集中している(上位 2 つの壁で停止の 75%)。操作ごとの停止率は低い(0.71%)が、依頼者の機械の 12 セッション中 10 が停止したのは、(a) その壁が**通常の操作の道筋にある**(Sampler のタブ、サブメニュー)ことと、(b) **走査していない状態・入力形が多い**(後述の限界)ことによる、と読む。(ii) 依頼者の再現 4 件のうち (a) は走査が**自動で**見つけた(Sampler の格子点 15 件、チャンネル設定窓 10 件)。(b) Settings の General タブ、(c) Browser のフォルダ展開、(d) Playlist の中ボタンドラッグは、native では**再現しない**(状態に入る操作も、タブ上の格子 168 件も、中ボタンドラッグの 23 + 25 件も、停止しない。入力を『ページが実際に送る形』〔密な MOVE 列・画面外の座標・ボタンの同時押し・up の欠落〕にした 12 変種も停止しない)。依頼者の機械の停止の原因はブラウザ版にだけある経路か、この走査の外にある、と読む(未確認)。(iii) **新しい壁**: 依頼者の実ブラウザのセッションで `user32!ToAscii`(ゲスト戻り先 0xa0c2c6)が出た。エンジンのキー処理(f_a0b250)は、**普通のキーを押すたびに** GetKeyboardState + ToAscii を呼ぶ(下の §5)。**user32 のキーボードレイアウト・キー変換の族 18 関数を、Microsoft Learn の契約と Microsoft の kbdus.c(MS-PL)の表から新規ファイル `fle_sweep1_kbd.c` に実装した**(`GetKeyboardLayoutNameW/A`、`GetKeyboardLayoutList`、`LoadKeyboardLayoutW/A`、`ActivateKeyboardLayout`、`ToUnicode`/`ToUnicodeEx`、`ToAscii`/`ToAsciiEx`、`VkKeyScanW/ExW/A/ExA`、`OemKeyScan`、`MapVirtualKeyExW/A/ExA`)。Wine 10.0 との差分テスト(vk 0〜255 × 10 状態 + VkKeyScan + OemKeyScan + MapVirtualKey、1,231 ケース)は、差のある 126 キーのすべてに出典つきの理由がある。(iv) PM の指示で、壁 2(`UpdateLayeredWindowIndirect` の `prcDirty`、21 件)を Learn の契約に追従させ(`sweep1-layered.diff`)、`fle_class.c` のクラス名の形による分岐(FL の命名の読み)を除去した(`sweep1-extra.diff`)。旧挙動を固定していた 8 検査を契約を確認する検査に書き換え、**全体テストは 184 results / 2,311 passed / 0 failed**(基準 183 / 2,307 / 0 に `sweep1.rs` の 4 件)。FLEngine の既定の生成 C の sha256 は `cd9107ce…` のまま。
3. **未確認。** (a) 走査は**間引き**(列挙した格子点 約 11,000 点/状態 に対し実行は 30〜700 件/状態。全数ではない。『全域を走査した』とは書かない)。(b) ブラウザ(wasm)での実行は未確認(native -O0 の観測)。(c) CPU 90 秒を超える 2 件は PROBE_CPU 900 でも終わらず、ゲストの中のループ(`SendMessageW(アプリ窓, 0xB03F, 0, 構造体)` の繰り返し)であることまでは確定したが、**原因の API は未特定**。(d) 実機 Windows との比較は無い(Wine 10.0 との差分と Learn だけ)。(e) キー変換の続き(Alt+テンキー入力、デッドキー、他の言語のレイアウト)は無い(US 1 つ)。(f) 修理後の再実行は『停止したプローブ全部 + 通過したプローブの 1/12』と、キー 4 窓の走査(§6)であり、全件ではない。

## 1. 背景(依頼者の言葉、2026-11-04)

『何をすれば落ちる、というより、何をすればまだ落ちない』。依頼者の機械で 12 セッション中 10 が engine exit code 134(停止)。落ちる操作は Sampler の Envelope タブのクリック、Settings の General タブのクリック、Browser のフォルダ展開、Playlist をマウスホイール押下でドラッグ、ほか多数。これまでの ia1 / ia2 は担当が選んだ 133 / 859 の操作だった。**今回は『人が選ぶ』をやめ、画面の全域・全入力種別を機械的に走査して、停止する操作の割合と原因(壁)の全体像を出す。**

## 2. 方式(機械的に列挙し、間引きは数えて書く)

道具はすべて `lift/out-sweep1/tools/`(再実行は `tools/README.txt` の 1 コマンド)。ia1 / ia2 の fork プローブ(準備済みの状態から `fork` した子で入力を注入し、N 仮想ミリ秒走らせ、終了状態と停止メッセージを記録する)をそのまま使い、違いは次のとおり。

- **操作を人が選ばない。** 状態ごとに、エンジンの窓表(`fle_win_table_report` の出力)から可視の窓(祖先と仮想モニター 1920x1080 で切り取った矩形)を読み、窓ごとに 16 画素の格子点(辺が 48 画素未満の窓は 8 画素)を取り、その点で最上位に当たる窓がその窓である点だけを残す(`tools/wintab.py` の当たり判定。覆われた点は上の窓の点)。
- 各格子点に 7 種の点入力(左クリック・右クリック・中ボタンクリック・左ダブルクリック・ホイール上・ホイール下・ホバーのみ)。ドラッグは 64 画素の副格子の各点に 3 方向(水平・垂直・斜め)× 3 長さ(8/64/256 画素)× 3 ボタン(左・中・右)= 27 組。キーは VK 0x08〜0xFE の 247 個 × 修飾 5 通り(なし/Ctrl/Shift/Alt/Ctrl+Shift)= 1,235 件。
- **全数は実行できない**(測定: 仮想 1 ミリ秒 ≈ CPU 10 ミリ秒、プローブ 1 件 ≈ 1〜2.5 CPU 秒、起動 1 回 ≈ 360〜420 CPU 秒。Basic 既定だけで格子点 10,667 点 × 7 種 ≈ 75,000 件 + ドラッグ約 18,000 件 ≈ 15 万 CPU 秒)。そこで、(i) 列挙した点の数を全部数えて表にし(`tables/enumeration.tsv`、`scripts/sweep1/plan.tsv`)、(ii) **基準状態と同一の窓**(lid・クラス・矩形が同じ)は基準状態でだけ走査し(S-sweep1-4)、(iii) 点を一定の間隔(stride)で間引き、点 j の入力種を `j mod 7`、ドラッグ j の組を `j mod 27` で巡回させた(S-sweep1-3)。**間引いた分の結果は分からない。** キーだけは全数(起動ウィザードと Basic の 2 状態)。
- 待ち時間は入力の後に仮想 60 ミリ秒(ia1/ia2 は 700。S-sweep1-2)。停止は入力の処理中に出るものを見る。タイマーで遅れて出る停止は取りこぼす(ia2 の停止は全て入力の直後に出ていた、と読むが、ここでは未検証)。
- 状態は子プロセスで作る(入る操作列を `tools/states.py` に記録)。48 状態: 起動ウィザード(Enter の前)、Basic 既定、チャンネル設定(808 Kick のボタン)、F7(Piano roll)、F8(Plugin picker)、F10(Settings の MIDI タブ)と Settings の他の 9 タブ、メニューバー 8 項目、サブメニュー 18 行(開いたメニューの窓を格子走査 = 項目を全部選ぶ)、Browser、Sampler.flp。F5/F6/F9 は Basic と窓が同一なので別走査しない。状態に入る操作が停止するかは準備走(`runs/E2`、47 状態、全て停止せず)で確かめた。
- 結果の分類: `ok`(子が probe-end まで走って exit 0)、`stop`(ランタイムが名前付きで止めた、または未実装の命令)、`timeout`(CPU 90 秒の上限で SIGKILL)、`engine-exit`(exit 0 だが probe-end が無い = エンジン自身が終了した。閉じるボタン・File>Exit。**停止に数えない**)。停止は API 名(import の thunk、無ければメッセージ)と呼び出し元 VA(thunk の guest-return)の組を『壁』とする。

### 2.1 測定した費用(この機械、load 20〜70、CPU 秒。`tables/timing.tsv`)

起動(Basic を開くまで)372 CPU 秒、アイドル 仮想 1 秒 ≈ 9〜10 CPU 秒、入力 + 仮想 100 ms のプローブ 1.1〜1.6 秒、200 ms で 2.0 秒、700 ms で 4.2 秒、窓を開く操作 15〜21 秒、窓のドラッグ 11〜27 秒。8,552 件の走査は Basic 側 3 並列 + Sampler 側 1 本で約 90 分(load 20〜40)。

## 3. 結果(基準 B0、8,552 操作。`tables/sweep-B0.tsv` が 1 操作 1 行、`tables/walls-B0.txt` が集計)

### 3.1 状態ごとの停止率(停止 = stop + crash + timeout + limit。『閉じる/終了』は含めない)

| 状態 | 列挙した格子点(16px・窓ごと) | 実行した操作 | 正常 | 停止 | タイムアウト | 閉じる/終了 | 停止率 |
|---|---:|---:|---:|---:|---:|---:|---:|
| w0 | 11107(基準と異なる窓: 11107)| 1575 | 1575 | 0 | 0 | 0 | 0.00% |
| basic | 10667(基準と異なる窓: 10667)| 2118 | 2111 | 6 | 0 | 1 | 0.28% |
| chset | 11085(基準と異なる窓: 1582)| 242 | 232 | 10 | 0 | 0 | 4.13% |
| f7 | 10955(基準と異なる窓: 3388)| 280 | 280 | 0 | 0 | 0 | 0.00% |
| f8 | 8040(基準と異なる窓: 8040)| 292 | 292 | 0 | 0 | 0 | 0.00% |
| f10 | 10364(基準と異なる窓: 2418)| 280 | 278 | 2 | 0 | 0 | 0.71% |
| s_audio | 10462(基準と異なる窓: 2516)| 169 | 169 | 0 | 0 | 0 | 0.00% |
| s_general | 10466(基準と異なる窓: 2520)| 169 | 169 | 0 | 0 | 0 | 0.00% |
| s_file | 10548(基準と異なる窓: 2602)| 175 | 175 | 0 | 0 | 0 | 0.00% |
| s_theme | 10462(基準と異なる窓: 2516)| 168 | 167 | 1 | 0 | 0 | 0.60% |
| s_project | 10676(基準と異なる窓: 2730)| 173 | 173 | 0 | 0 | 0 | 0.00% |
| s_info | 10690(基準と異なる窓: 2744)| 174 | 174 | 0 | 0 | 0 | 0.00% |
| s_debug | 10462(基準と異なる窓: 2516)| 168 | 168 | 0 | 0 | 0 | 0.00% |
| s_account | 10681(基準と異なる窓: 2735)| 172 | 172 | 0 | 0 | 0 | 0.00% |
| s_about | 10607(基準と異なる窓: 2661)| 168 | 168 | 0 | 0 | 0 | 0.00% |
| m_file | 10420(基準と異なる窓: 273)| 137 | 131 | 0 | 0 | 6 | 0.00% |
| m_edit | 10540(基準と異なる窓: 35)| 35 | 35 | 0 | 0 | 0 | 0.00% |
| m_add | 10273(基準と異なる窓: 308)| 103 | 103 | 0 | 0 | 0 | 0.00% |
| m_patterns | 10302(基準と異なる窓: 475)| 119 | 119 | 0 | 0 | 0 | 0.00% |
| m_view | 10437(基準と異なる窓: 585)| 117 | 117 | 0 | 0 | 0 | 0.00% |
| m_options | 10445(基準と異なる窓: 455)| 114 | 114 | 0 | 0 | 0 | 0.00% |
| m_tools | 10187(基準と異なる窓: 783)| 112 | 111 | 1 | 0 | 0 | 0.89% |
| m_help | 10470(基準と異なる窓: 273)| 137 | 137 | 0 | 0 | 0 | 0.00% |
| m_file_NewFromTemplate | 10356(基準と異なる窓: 30)| 30 | 30 | 0 | 0 | 0 | 0.00% |
| m_file_Import | 10448(基準と異なる窓: 33)| 33 | 33 | 0 | 0 | 0 | 0.00% |
| m_file_Export | 10418(基準と異なる窓: 242)| 41 | 40 | 1 | 0 | 0 | 2.44% |
| m_add_Effect | 10160(基準と異なる窓: 77)| 39 | 39 | 0 | 0 | 0 | 0.00% |
| m_view_Toolbars | 10298(基準と異なる窓: 350)| 39 | 37 | 2 | 0 | 0 | 5.13% |
| m_view_ArrangeWindows | 10392(基準と異なる窓: 110)| 37 | 37 | 0 | 0 | 0 | 0.00% |
| m_view_Background | 10452(基準と異なる窓: 60)| 30 | 30 | 0 | 0 | 0 | 0.00% |
| m_tools_OneClickAudioRec | 10187(基準と異なる窓: 48)| 48 | 48 | 0 | 0 | 0 | 0.00% |
| m_tools_DumpScoreLog | 10180(基準と異なる窓: 42)| 42 | 42 | 0 | 0 | 0 | 0.00% |
| m_tools_AudioLoggerSettings | 10196(基準と異なる窓: 81)| 41 | 41 | 0 | 0 | 0 | 0.00% |
| m_tools_LastTweaked | 10147(基準と異なる窓: 220)| 37 | 37 | 0 | 0 | 0 | 0.00% |
| m_tools_NextToLastTweaked | 10148(基準と異なる窓: 220)| 37 | 37 | 0 | 0 | 0 | 0.00% |
| m_tools_SwitchAllAudioClipChannels | 10147(基準と異なる窓: 144)| 36 | 36 | 0 | 0 | 0 | 0.00% |
| m_tools_SwitchSelAudioClipChannels | 10134(基準と異なる窓: 144)| 36 | 36 | 0 | 0 | 0 | 0.00% |
| m_tools_SwitchAllPluginEditors | 10222(基準と異なる窓: 50)| 50 | 50 | 0 | 0 | 0 | 0.00% |
| m_tools_SetKeyboardFocusPlugins | 10222(基準と異なる窓: 50)| 50 | 50 | 0 | 0 | 0 | 0.00% |
| m_tools_ChangeDeclicking | 10187(基準と異なる窓: 63)| 32 | 32 | 0 | 0 | 0 | 0.00% |
| m_help_Tutorials | 10485(基準と異なる窓: 90)| 45 | 25 | 20 | 0 | 0 | 44.44% |
| br_tab2 | 10799(基準と異なる窓: 176)| 88 | 88 | 0 | 0 | 0 | 0.00% |
| br_tab2_row | 10799(基準と異なる窓: 176)| 88 | 88 | 0 | 0 | 0 | 0.00% |
| sampler | -(基準と異なる窓: 1828)| 476 | 458 | 16 | 2 | 0 | 3.78% |
| **合計(48 状態)** | | **8552** | **8484** | **59** | **2** | **7** | **0.71%** |

サブメニュー(18 状態)の合計は 703 操作中 23 停止(3.27%)、メニュー(8 状態)は 874 中 1(0.11%)、Settings の 10 タブは 1,816 中 3 ですべてのタブが 0〜1 件、起動ウィザードは 1,575 中 0(キー 1,235 全数を含む)、Basic 既定は 2,118 中 6(0.28%)。**停止率が高いのは Sampler 窓(3.78%)、チャンネル設定窓(4.13%)、サブメニュー(Help>Tutorials 44%)**で、どれも**1 つの壁がその窓の一部分(タブ列、サブメニューの窓)を覆っている**ためで、窓の全域に散らばってはいない。

### 3.2 入力種別ごとの停止率

| 入力種別 | 操作 | 正常 | 停止 | タイムアウト | 閉じる/終了 | 停止率 |
|---|---:|---:|---:|---:|---:|---:|
| 左クリック | 794 | 777 | 14 | 0 | 3 | 1.76% |
| 右クリック | 786 | 777 | 7 | 0 | 2 | 0.89% |
| 中ボタンクリック | 777 | 775 | 1 | 1 | 0 | 0.26% |
| 左ダブルクリック | 776 | 754 | 20 | 0 | 2 | 2.58% |
| ホイール上 | 767 | 767 | 0 | 0 | 0 | 0.00% |
| ホイール下 | 761 | 760 | 1 | 0 | 0 | 0.13% |
| ホバーのみ | 757 | 757 | 0 | 0 | 0 | 0.00% |
| ドラッグ(左・中・右) | 664 | 652 | 11 | 1 | 0 | 1.81% |
| キー(VK 0x08〜0xFE × 5 修飾) | 2470 | 2465 | 5 | 0 | 0 | 0.20% |

左ダブルクリックが最も高い(2.58%)のは、サブメニューの窓・タブ列での 2 回目のクリックが壁を踏むため。ホバー・ホイールはほぼ 0。キーは 2,470 件中 5 件(右 Alt = VK 0xA5 × 修飾 5 通り。§5)だけ。

### 3.3 壁(API + 呼び出し元 VA)の件数順、自然な操作か、持ち主

自然さの規則(`tools/natural.py`): クリック・ダブルクリック・ホイール・ホバー・ドラッグは自然。キーは修飾が高々 1 つ(なし/Ctrl/Shift/Alt)で、キーボードにある鍵(VK 0x08〜0x87、左右の Shift/Ctrl/Alt、OEM 記号)なら自然。Ctrl+Shift の組、メディアキー・予約コード(0x88〜0xB9、0xE0〜)は稀。

| # | 壁(API @ 呼び出し元 VA) | 件数 | 当たる状態・入力 | 自然な操作か | 持ち主 / 扱い |
|---:|---|---:|---|---|---|
| 1 | `shell32!DragAcceptFiles` @0x827593(`unsupported instruction at 0x592850`) | 25 | Sampler 窓 15、チャンネル設定窓 10: タブ列の左クリック・ダブルクリック・ドラッグ・ホイール | **自然**(タブを押す) | crash1(`fle_crash1_drag.c`、行は `crash1-imports.txt`)。**触らない** = 依頼者の再現 (a) |
| 2 | `UpdateLayeredWindowIndirect` の `prcDirty` @0x80dd3b | 21 | Help>Tutorials のサブメニュー窓 20、View>Toolbars のサブメニュー窓 1: クリック・右クリック・ダブルクリック | **自然**(サブメニューの項目を押す) | 既存 `fle_layered.c`(決定 0140 の名前付き停止、`tests/wall_fr16.rs` が停止を固定)。**PM の指示で sweep1 が直した**(§5.4、`sweep1-layered.diff`) |
| 3 | (エンジン自身の終了) | 7 | 閉じるボタン 1、File>Exit 6 | 自然 | 正常な終了。壁ではない |
| 4 | `user32!GetKeyboardLayoutNameW` @0x8433d2(`unsupported instruction at 0x424d80`) | 5 | 右 Alt(VK 0xA5)× 修飾 5 通り(Basic のキー走査。Ctrl+Shift+右 Alt の 1 件だけは『稀』) | **自然**(右 Alt を押す) | **sweep1 が直した**(§5) |
| 5 | (CPU 90 秒超、2 件) | 2 | Sampler の波形表示: 中ボタンのクリック (463,491)、8px の縦ドラッグ (455,467) | 自然 | ゲスト内のループ(§5.6)。**未特定** |
| 6 | `gdi32!ExtTextOutW`(グリフの描画は未実装、決定 0111)@0x7fd56f | 1 | Browser の見出し (85,102) の左ドラッグ | 自然 | txt1(`fle_gdi.c`)。触らない |
| 7 | `advapi32!RegOpenKeyExW`(キーパスの空の要素)@0x5b0d58 | 1 | Settings MIDI タブ (720,963) のダブルクリック(『Refresh device list』付近) | 自然 | `fle_reg.c`(Learn に記述なし)。依頼 |
| 8 | `kernel32!CompareStringW`(ASCII 英数字以外の不等な文字列)@0x439a3d | 1 | Settings MIDI タブの左ドラッグ (944,675) | 自然 | `fle_locale.c`(crash1 が同じファイルを編集中)。依頼 |
| 9 | `version!GetFileVersionInfoSizeW`(ディレクトリ部の無いファイル名)@0x44adeb | 1 | File>Export サブメニューのダブルクリック | 自然 | `fle_vfs.c`。依頼 |
| 10 | `shell32!PathCleanupSpec`(不正文字の置換規則はページに無い)@0x81a2bf | 1 | Tools メニューのダブルクリック | 自然 | 既存 `fle_*`(ページが沈黙)。依頼 |
| 11 | `kernel32!LoadLibraryW "ilsvg_x64.dll"` @0x44db70 | 1 | View>Toolbars サブメニューのダブルクリック | 自然 | モジュールの供給(M2)。dec1 の系統 |
| 12 | 呼び出し先 0 への間接呼び出し @0x43d093 | 1 | Settings Theme タブの中ボタンクリック (760,777) | 自然 | **未特定**(CompareStringW の並べ替えの直後のコールバックが 0) |
| 13 | `kernel32!LoadLibraryW "elastique_x64.dll"` @0x1d3e9d0 | 1 | Sampler のつまみの左ドラッグ | 自然 | dec1 の次の壁(Elastique の供給) |

**停止の 75% は上位 2 つの壁**(46/61)。API の壁の種類は 13 だが、`Win32 の API の壁をファミリー単位で直す` の対象になるのは、**自分の新規ファイルだけで直せるもの**に限られる。4 が該当し(§5)、PM の指示で 2 も直した(§5.4)。1・6・8 は他担当が持ち、7・9・10 は既存ファイルの編集か出典の無い選択が要る。

## 4. 依頼者の再現 4 件はどの壁に当たるか(実エンジン、native -O0)

| 依頼者の操作 | 走査・アンカーでの結果 | 壁 |
|---|---|---|
| (a) Sampler の Envelope タブのクリック | **停止を再現**。走査(Sampler.flp、格子 15 件が壁 1)とアンカー(2 番目のタブ (481,244) → exit 134、1 番目・3 番目のタブは通る)。チャンネル設定窓(808 Kick のボタン)の同じタブ列でも 10 件 | 壁 1 `shell32!DragAcceptFiles` @0x827593(crash1 が行と実装を持つ) |
| (b) Settings の General タブのクリック | **停止しない**。状態に入る操作(F10 → General タブ)も、General タブ上の格子 169 件(クリック・ダブルクリック・右/中・ホイール・ドラッグ)も ok | - |
| (c) Browser のフォルダの展開 | **停止しない**。『Browser の 2 番目のタブ → 最初の行』の状態に入る操作と、その上の 88 + 88 件、アンカー(クリックを 3 回)が ok。Browser の左の一覧は黒く描かれる(別件) | - |
| (d) Playlist を中ボタンで押したままドラッグ | **停止しない**。走査の中ボタンのドラッグ 224 件(うち Playlist 側 23 件)、アンカー 13 件(2 点 × 水平・垂直・斜め × 64/256px + 左 1)、『ページが実際に送る形』の 12 変種(密な MOVE 列〔60〜200 歩〕、画面外の座標〔x < 0、y < 0、遠方〕への移動、左または右ボタンの同時押し、up の欠落、ドラッグ後のホイール)が全て exit 0 | - |

読み: (b)(c)(d) が native で停止しないのは、crash1 の『再現できず』と一致する。依頼者の機械の 134 の原因は、(1) ブラウザ版にだけある経路(ページの入力・描画の経路、Firefox)、(2) この走査が間引いた入力、(3) native の仮想時間とブラウザの実時間の違いによる順序の違い、のいずれかで、**どれかは未確認**(推測)。crash1 のテレメトリ(`engine.exit.message`)の次の実セッションの停止メッセージが決め手になる。実際に依頼者のセッションから 2 件の停止メッセージが届いており(PM 経由)、1 件は壁 1、もう 1 件は **`user32!ToAscii`(ゲスト戻り先 0xa0c2c6)**で、これは本走査(キー走査はフォーカスが主窓のとき)が踏まなかった壁である(§5.1)。

### 4.1 ブラウザ版にだけある経路の候補(PM の依頼。`app/src/input/pointer.ts` と `tracker.ts` を読んだ。**推測**であり、native のプローブで確かめられたものと確かめられないものを分ける)

ページが送る形: `pointerdown/move/up/cancel` を `buttons` のマスクで読み、位置が変わったら MOVE、ボタンの変化は up を先、down を後に BUP/BDOWN(`data` = BTN: 左 0、右 1、中 2。G3 の `down M` も `FLE_IN_BTN_M` = 2 で**同じホスト API `fle_input_host_mouse`**を呼ぶ)。`pointerdown` で `preventDefault` + `setPointerCapture`。`contextmenu` `auxclick` `dragstart` `selectstart` は止める。ホイールは `WheelAccumulator`。

| 候補 | native で確かめたか | 結果 |
|---|---|---|
| 密な MOVE 列(60〜200 歩、5〜10 ms 間隔)での中ボタンドラッグ | **確かめた**(A2: `client2.d_Mdense_*`、`d_Mlong_h`) | 停止なし |
| 画面外の座標(x < 0、y < 0、遠方)への移動(ポインタキャプチャ中は窓の外でも pointermove が届く) | **確かめた**(`d_Mout_left/top/far`。ランタイムは座標を 0..1919 × 0..1079 に丸める) | 停止なし |
| ボタンの同時押し(中 + 左、中 + 右。2 つ目のボタンは `pointermove` の `buttons` の変化として届く) | **確かめた**(`d_Mchord_L/R`) | 停止なし |
| up の欠落(フォーカス喪失・pointercancel。ページは `pointerCancel` で up を補う) | **確かめた**(`d_Mnoup`: up を送らず 400 ms 待つ) | 停止なし |
| ドラッグ後のホイール | **確かめた**(`d_Lwheel_after_M`) | 停止なし |
| ブラウザの中ボタンの既定動作(Firefox のオートスクロール、Linux の貼り付け)がページの `preventDefault` をすり抜ける | **確かめられない**(DOM の挙動) | - |
| ブラウザの実時間でゲストが仮想時間に追いつかない(依頼者の機械でエンジンの idle は 23.7/s = 約 42 ms 周期)ため、1 回のメッセージループの周期の間に**多数の MOVE が溜まる**(キューの深さ・併合の違い) | **確かめられない**(G3 の `move` は 1 回の idle に 1 件。溜めるには harness の変更が要る。native の仮想時間では遅れない) | 最も疑わしい差、と読む(未確認) |
| wasm 側のメモリ(2 GiB 中 0.46〜0.50 を使用。描画の大きな面の確保で失敗する経路) | **確かめられない** | - |

次の一手: crash1 のテレメトリ(`engine.exit.message`)で届く依頼者の次の停止メッセージを見る。

## 5. 修理: user32 のキーボードレイアウト・キー変換の族(18 関数、新規 `fle_sweep1_kbd.c`)

### 5.1 見つけた壁と、依頼者のセッションの `ToAscii`

走査の壁 4: 右 Alt(VK_RMENU 0xA5)を押すと `user32!GetKeyboardLayoutNameW`(行なし)で停止する(呼び出し元 0x8433d2。エンジンが AltGr かどうかをレイアウト名で決める経路)。PM から届いた依頼者のセッションの停止メッセージ `fle: unsupported instruction at 0x425600 [import user32.dll!ToAscii] [guest return 0xa0c2c6]`(bundle f4a57e84a5e1 = SHIP2)も、同じ族(api2 の census: `ActivateKeyboardLayout CharNextExA CharToOemBuffW GetKeyboardLayoutList GetKeyboardLayoutNameW LoadKeyboardLayoutW SendInput ToAscii ToUnicode`)にある。

**0xa0c2c6 の前後(静的に読んだ。実行はしていない)**: ToAscii の呼び出しは関数 `f_a0b250`(0xa0b250..0xa0caa5、3.7 KB)の中にある。Delphi/VCL のコントロールのキー処理(メッセージ記録 [msg+8] = キーコード、[msg+0x10] = lParam)で、先頭で仮想メソッド(VMT 0x548)に問い合わせた後、キーコード(VK)が集合 {Back, Tab, PgUp, PgDn, End, Home, Left, Up, Right, Down}(0xa0cab8 のビット集合 0x1fe000003 を読んだ)**に入っていなければ**(アドレス 0xa0c287)`GetKeyboardState(buf)` → `ToAscii(VK, (lParam >> 16) & 7, buf, &ch, 0)` を呼び、戻りが正で `ch` が `*` `+` `-` `/` のとき、キーコードをテンキーの `VK_MULTIPLY` `VK_ADD` `VK_SUBTRACT` `VK_DIVIDE` に書き換える(木の展開・折りたたみキーの正規化: 推測。コントロールの種類は静的には特定できなかった。Browser の木・一覧のようなものと読む。推測)。**つまりそのコントロールにフォーカスがあるとき、ナビゲーションキー以外のほぼ全てのキーが ToAscii に到達する。** 本走査のキー走査は『フォーカスが主窓/ウィザード』の状態だったので踏まなかった。再実行のキー走査(§6.2)は、Browser の一覧をクリックした後、Playlist をクリックした後、Piano roll、Settings で行う。

### 5.2 実装(出典と選択)

- 出典: Microsoft Learn の各ページ(GetKeyboardLayoutName(W)、GetKeyboardLayoutList、LoadKeyboardLayout(W)、ActivateKeyboardLayout、ToUnicode、ToAscii、VkKeyScan、OemKeyScan、MapVirtualKey)と、**Microsoft の kbdus.c**(microsoft/Windows-driver-samples の `input/layout/kbdus/kbdus.c`、MS-PL。ia2 の GetKeyNameTextW と同じ出典)の `aVkToWch1〜4`、`aVkToBits`、`CharModifiers`(wMaxModBits 3)、CAPLOK。仮想レイアウトは 1 つ(決定 0063: HKL 0x04090409、KLID "00000409")。
- **FL 側の特別扱いは無い**(キーの種類・窓・クラスで分岐しない。表は US レイアウトの表そのもの)。`fle_input.c` には**触らない**(既存のキー状態は ToUnicode/ToAscii の引数 `lpKeyState`〔ゲストが GetKeyboardState で得た 256 バイト〕を**読む契約**だけで接続し、書き込みを増やさない)。TranslateMessage が使う `fle_input.c` の表(`vk_to_char`)とは別物で、差(Ctrl+Enter が '\n'、Shift+Tab が '\t'、テンキー、Ctrl+Alt)は §7 に依頼として書く。
- 契約どおり: ToUnicode の戻り(0 = 変換なし、バッファに書かない / 1 = 1 UTF-16 単位 / 負 = デッドキー〔US には無い〕)、`lpKeyState` の 0x80(押下)と 0x01(トグル、CapsLock だけ有効)、修飾は汎用の VK_SHIFT(0x10)・VK_CONTROL(0x11)・VK_MENU(0x12)の 3 つ。ページが沈黙する入力(NULL のバッファ、負の nBuff、hkl が仮想の 1 つでない)は**名前付きの停止のまま**(偽の成功にしない)。
- 出典のない選択は S-sweep1-kbd-1〜8(§10)に列挙した。主なもの: Ctrl+Alt(AltGr の状態)は文字なし(kbdus.c の `CharModifiers` の構造から。Wine は Alt を無視して素の文字を返す)、Alt 単独は無視して素の文字(WM_SYSCHAR の文字)、A〜Z の Ctrl は制御コード(vk - 'A' + 1)、CapsLock は CAPLOK の行で Shift のように働く(Ctrl が押されていないとき)。

### 5.3 検証(Wine 10.0 との差分、`harness/sweep1/`、`tests/sweep1.rs`)

同じケースを mingw でビルドして Wine 10.0 の実 API で動かし(`build.sh wine`)、ランタイムの関数(`build.sh native`)と 1 行ずつ比べる: ToUnicode / ToAscii を **vk 0〜255 × 10 状態**(なし・Shift・Ctrl・Shift+Ctrl・Alt・Ctrl+Alt・Caps・Caps+Shift・Caps+Ctrl・Shift+Alt)、VkKeyScanW 0〜0x7F + 7 件、VkKeyScanA、ExW、OemKeyScan 0〜0xFF、MapVirtualKeyExW/A、Ex 変種。**1,231 行のうち差は 126 キー(`expected/known-divergence.txt`、全てに理由)**: Ctrl+Alt の 66 行(意図した選択)、kbdus.c にあり Wine に無い行(VK_CANCEL、Shift+Tab、Shift+VK_DECIMAL、VK_DIVIDE、VK_OEM_102 は Wine が '<' '>')、Wine が ToUnicode で 0 を返すときバッファに NUL を書く(Learn は『書かない』)、Wine の ActivateKeyboardLayout は未実装スタブ(120)、Wine の VkKeyScan('a') = 0x0141 / VkKeyScan('A') = 0x0041(Windows は 0x0041 / 0x0141。Caps Lock は消してあるので Wine 固有の挙動。UNOFFICIAL)、OemKeyScan の制御文字、既存 MapVirtualKeyW(`fle_input.c`、触らない)にテンキー 0 の走査コードが無い等。Rust のテスト 4 件(行の宣言・定義・新規性、Wine と同じ出力、名前付き停止 12 件、ヘッダと行の一致)が通る。

### 5.4 修理 2: `UpdateLayeredWindowIndirect` の `prcDirty`(壁 2、21 件、`sweep1-layered.diff`)

PM の指示(第 3 報への返信)により、決定 0140 の名前付き停止を『契約の不足』として解消した。Learn の UPDATELAYEREDWINDOWINFO: `prcDirty` は『更新する領域。NULL でもよい。NULL でなければ、その矩形の範囲だけがソース DC から更新される』。座標系はページに無い。**実装**(`fle_layered.c`): 矩形を**レイヤー窓自身の座標**(原点 = 窓の左上 = 窓の面の座標。S-sweep1-ulw-1、UNOFFICIAL: Wine 10.0 のドライバは窓の矩形 {0, 0, cx, cy} と prcDirty の交わりを取る)で読み、窓の外の部分・空の矩形・逆転した矩形は何も更新しない。窓画素 (x, y) のソースは (sx + x, sy + y)(全面更新と同じ)。面の持ち方・dirty の記録(`fle_raster_wrote`)は変えず、更新した矩形だけを記録する(空なら記録しない)。desk1(layered 表面を読む側)に見えるのは『dirty がより小さい矩形になる』ことだけ。差分は +57 行/-14 行(`-` 行は旧テスト `a_dirty_rectangle_stops_by_name` の 13 行と `stopf` の 1 行。全部読んだ)。
**テスト**: `wall_fr16.rs` の『停止を確認する検査』を『契約を確認する検査』に置き換えた(弱めたのでなく契約への追従): 3×1 の面で、全面更新 → 矩形 x 1..2 だけ更新 → 空の矩形・窓の外・逆転した矩形は不変 → -4 から始まる矩形は窓にクリップ → 窓より大きい矩形は全面。17 件通過。Wine との表面の差分は取れていない(Wine の layered 窓の表面を読み出す手段がこの機械に無い。Wine のソースの記述に基づく読み: 未確認)。

### 5.5 余計な処理の除去: `fle_class.c` のクラス名の形による分岐(`sweep1-extra.diff`)

PM の指示により、方針『lift の忠実性』に反する分岐を除いた: `GetClassInfoW` とクラス検索(CreateWindowEx)が、クラス名が『T + 大文字』(Delphi のクラス名の形)か、ゲストの 7 つの名前のときだけ『見つからない(0)』を返し、他は停止していた。Learn の契約は『一致するクラスが無ければ 0』で、名前の形は関係しない。**除去**: `is_own_static_name` を `!is_system_class_name` に置き換え(システムクラス = Button/Edit/ComboBox/… は system が登録したクラスで、データは system が答える。未モデルなので停止のまま)、`is_delphi_class_name` と 7 つの名前の表を削除。停止メッセージも『システムクラス』に直した。**壊れたもの(= 旧挙動を固定していた検査)**: `class.rs` 4 件、`wall_fr7.rs` 2 件、`window.rs` 1 件の計 7 検査が『その他の名前は停止』を確認していた → 『システムクラスは停止、それ以外の未登録の名前は 0(CreateWindowEx では NULL)』を確認する検査に書き換え(契約への追従)。足りない API は見つからなかった(ゲストが問い合わせる名前は全部 T + 大文字か 7 つの名前だった = 実行経路は変わらない)。**未解決(依頼 R12)**: CreateWindowEx が未登録のクラスで NULL を返すとき、実システムは `ERROR_CANNOT_FIND_WND_CLASS`(1407)を設定する。ランタイム(`fle_win.c`、present2/txt1 が編集中)は最終エラーを設定しない。差分は +57 行/-82 行(`-` 行は 7 検査の旧い期待値と `fle_class.c` の旧い判定。全部読んだ)。

### 5.6 CPU 90 秒超の 2 件(Sampler の波形表示)— ゲスト内のループ、原因の API は未特定

Sampler の波形表示の上の中ボタンのクリック (463,491) と 8px の縦ドラッグ (455,467)(同じ領域)が、PROBE_CPU 900 でも終わらなかった(`runs/CT`: 1 件目が SIGKILL まで 900 CPU 秒)。原因の手がかりを得るため、エンジンの命令数の上限(`FLE_FRUITY_MAX_CALLS` = ヘッダ + 7,000 万)を付けて再実行した(`runs/CT2`、2 件とも上限で止まり、停止報告を出した)。**2 件とも同じ場所**: シャドウスタックは `f_863f20 ← f_863f60 ×3(再帰)← f_864030 ← f_410090 ← f_60c3e0 ← f_612e10 ← f_85c6d0 …`、import の直近 48 件の大半が `SendMessageW(0x7f990014 [アプリケーション窓], 0xB03F, 0, 構造体へのポインタ)`(呼び出し元 0x604f94、1 回ごとに違うポインタ、14,400 関数エントリごとに 1 回)で、残りは SetWindowOrgEx / SelectClipRgn / DeleteObject(描画)。ループの前の import は `PeekMessageW → GetCursorPos → WindowFromPoint → GetWindowThreadProcessId → GlobalFindAtomW → GetPropW(hwnd, 0xC001)`(VCL のコントロール探索)`→ ClientToScreen → GetCapture → RedrawWindow → IsWindowVisible/IsWindowEnabled`。つまり**マウスのキャプチャ/ホバーの追跡(VCL)の後に、コントロール木の再帰的な走査が終わらない**。API 呼び出しの 1 つが返す値(GetCapture、WindowFromPoint、GetCursorPos)が木の走査の終了条件を満たさない可能性がある(推測)。`call [IAT]` の呼び出しは記録に出ないため、どの API かは確定できない。**静的に読んだこと(推測を含む)**: 0xB03F を送るのは `f_5de830`(アクション/コマンドの配送: 対象の持ち主の仮想メソッド〔VMT +0x80〕→ 大域の一覧 `f_86ca60` → `f_529080` の順に試し、どれも真を返さなければ `f_604f60` 経由で `SendMessageW(アプリ窓, 0xB03F, 0, 対象)` を送り、戻りが 1 なら真)。0xB03F は VCL の `CM_BASE`(0xB000)+ 63 = `CM_ACTIONUPDATE`、0xB040 は `CM_ACTIONEXECUTE` と読める(推測: VCL の定数)。受け側はアプリ窓の窓手続き `f_868c00`(`ecx - 0xB03F < 2` の分岐が 0x8691c7)。`f_863f60` は **コントロール木の再帰的な走査**(子の数 `f_611430`、子の取得 `f_6113f0`、子のフラグ [+0xa9] が立っていれば `f_863f20`〔`子.VMT[0x80](アクション)`〕を呼び、真なら終了。子がコントロールなら再帰)で、アクションの更新/実行を木の全ノードに差し出す VCL の処理にあたる。**走査が 1 周して戻らない**のは、(1) ノードの数や並びが走査中に変わり続ける、(2) 戻り値を決める API の応答が終了条件を満たさない、のどちらか。どちらかは未確認。ブラウザの『Playlist 中ボタン 2 fps』(PM の依頼 3 の最後)が同じ木の走査かは、Playlist の中ボタン操作でこの 0xB03F の送信の頻度を数えれば確かめられる(`call [IAT]` も数えるトレースが要る)。

**次の一手**: `call [IAT]` も記録するトレースを付けて、ループ 1 周の API 列を取る。**変種の結果(B3 で再実行、`runs/RS`)**: 同じ点 (463,491) への **down/up を 60 ms 離した中ボタン、400 ms 離した中ボタン、左クリック、右クリックが全て CPU 90 秒超(SIGKILL)**。つまり『ボタンの種類や down/up の間隔』ではなく**この場所(Sampler の波形表示の下の部分)を押すこと**がループの引き金である。同じ Sampler の別の場所 (463,300) の中ボタンクリックは正常。自然な操作(波形を押す)なので、依頼者の『Sampler を押すと固まる』にも関係しうる(推測)。

## 6. 再実行(前後)

再実行は**全件ではない**(CPU が足りない: 起動 1 回 360〜420 CPU 秒 + 1 件 1〜2.5 秒で、load 20〜70 の機械)。規則: (i) 基準で停止/超過だった物は全部、(ii) 通過した物は状態ごとの標本、(iii) 基準に無い変種。`tools/rerun.py` は基準と**同じ入力**(同じ名前・同じ操作列)で再実行用のスクリプトを作る。分類: 『停止 → 通過』(壁が消え、プローブが終わりまで走った = **実際に次へ進んだ**)、『停止 → 別の名前付き停止』(**進んだとは数えない**)、『停止 → 同じ壁』、『通過 → 停止』(退行)。

### 6.1 B3(= HEAD + sweep1 の rows 18 行、`fle_layered.c` と `fle_class.c` は基準のまま)

| 範囲 | 再実行 | 停止 → 通過 | 停止 → 別の停止 | 停止 → 同じ壁 | 通過 → 停止(退行) | 通過 → 通過 |
|---|---:|---:|---:|---:|---:|---:|
| Basic 側(起動ウィザード、Basic、チャンネル設定、F7、F8、F10 の状態。`runs/RB`) | 181 | **5**(右 Alt の `GetKeyboardLayoutNameW`) | 0 | 13(DragAcceptFiles 10、ExtTextOutW 1、RegOpenKeyExW 1、CompareStringW 1) | **0** | 163 |
| Sampler 側(`runs/RS`) | 40 + 変種 6 | 0 | 0 | 18(DragAcceptFiles 15、CPU 超過 2、Elastique 1) | **0** | 16 |

Basic 側は Settings の 9 タブとメニューの状態の前で止めた(下記)。**『5 件が通過』が、キーボード族の修理の効果として測れた全て**: 右 Alt(VK 0xA5)× 修飾 5 通りが、停止せずに終わる。他の壁は、そのファミリーを直していないので変わらない(正しい)。通過 163 件は退行なし。

**ツールの失敗(正直に)**: Basic 側の再実行で、Settings の 2 つのタブの状態の子(`S_s_audio_0` `S_s_general_0`)が SIGKILL(signal 9)で消えた。状態の子自体の CPU 上限(PROBE_CPU = 90 秒)を、Settings 窓を開く操作(60〜80 CPU 秒、load 50 では上限を超える)が超えたため。基準の走査は同じ上限で通った(全 96 状態の子が exit 0)が、load が違った。以後のスクリプトは上限を 400 秒にし、プローブの上限を 90 秒に下げる(`setenv FLE_G3_PROBE_CPU 90`)。B3 の Basic 側はそこで止め、残りの状態を B4 の再実行(6.3)に回した。

### 6.2 キー走査(B3、模擬: native のプローブ)— 『キーを押して止まらない』を Browser の一覧・Playlist・Piano roll・Settings で

`tools/mkkeys.py`(`runs/K`、`tables/keys-B3.tsv`)。各状態は『その窓をクリック(F7 は Piano roll を開く、F10 は Settings を開く)してから VK 0x08〜0xFE を押す』。実行できた範囲:

| 状態 | 計画(VK 0x08〜0xFE) | 実行できた | 正常 | 停止 |
|---|---|---:|---:|---:|
| Browser 一覧をクリック後の 12 キー(確認) | 12 キー | 12 | 12 | 0 |
| Browser の一覧をクリックした後 | なし/Shift/Ctrl → 実行は なし/Shift の 494 件(Ctrl は未実行) | 494 | 494 | 0 |
| Playlist をクリックした後 | なし 247 件 | 247 | 247 | 0 |
| Piano roll(F7) | なし 247 件 | 247 | 246 | 1 |
| Settings(F10) | なし 247 件 | 247 | 247 | 0 |

合計 1247 件、停止 1 件(k_f7.k_7b_n: user32.dll!DispatchMessageA 0x86a200)。実行は途中で止めた(時間): 未実行の状態・キーは『未実行』であり『通過』ではない。

**`ToAscii` が依頼者のセッションでだけ踏まれた理由は、この走査では特定できなかった**: 基準 B0(ToAscii の行なし)で、Browser の一覧・Playlist・Piano roll・Settings・Channel rack 名・フォーカスなしの 6 状態に A / 1 / Space / Enter を送ったが(`runs/TW0`)、**ToAscii の停止は 1 件も出なかった**(24 件すべて exit 0)。f_a0b250 を呼ぶコントロール(Delphi の木/一覧)に、クリックではフォーカスが移っていない、または描かれていない(Browser は黒)ためと読む(推測)。B3 の側は 12 キーの確認(`chk.*`)も含めて全て通過した。

**新しい壁(キー走査で発見)**: Piano roll(F7 で開いた状態)で **F12(VK 0x7B)を押す**と `user32!DispatchMessageA`(行なし、`unsupported instruction at 0x4249b0`、呼び出し元 0x86a200 = エンジンのメッセージループ)で停止する。自然な操作。api2 の census の『window/message plumbing(A-variants)』の族(`DispatchMessageA` `SendMessageA` `SetWindowTextA` `FindWindowExW` `GetClassLongPtrW` ほか 12 関数)の 1 つで、窓のメッセージの層(`fle_win.c` = present2/txt1 の編集中)に触れるため実装していない(依頼 R13)。

### 6.3 B4(= B3 + `sweep1-layered.diff` + `sweep1-extra.diff`)

再実行 **114** 件(基準で停止・超過だった物のうち、実行できた状態の分 + 通過の標本。状態: basic, chset, f10, f7, f8, m_file, m_file_Export, m_help_Tutorials, m_tools, m_view_Toolbars, s_theme)。停止 → 通過 **24**(うち `UpdateLayeredWindowIndirect` の prcDirty の壁 **19 / 19**)、停止 → 別の名前付き停止 0、停止 → 同じ壁 17、**通過 → 停止(退行)0**、通過 → 通過 73。

```
## re-run: 114 probes (every probe that was not ok in the baseline + a sample of those that were)
    24  not ok -> ok (advanced)
    17  not ok -> same wall (unchanged)
    73  ok -> ok

## per wall of the baseline (only the probes that were re-run)
wall	before	advanced	another wall	unchanged	example
:UpdateLayeredWindowIndirect:0x80dd3b	19	19	0	0	m_help_Tutorials.cL_0469_0078
:shell32.dll!DragAcceptFiles:0x827593	10	0	0	10	chset.dL_0735_0336
:user32.dll!GetKeyboardLayoutNameW:0x8433d2	5	5	0	0	basic.k_a5_n
:gdi32.dll!ExtTextOutW:0x7fd56f	1	0	0	1	basic.gL_h64_0085_0102
:advapi32.dll!RegOpenKeyExW:0x5b0d58	1	0	0	1	f10.dL_0720_0963
:kernel32.dll!CompareStringW:0x439a3d	1	0	0	1	f10.gL_h8_0944_0675
:version.dll!GetFileVersionInfoSizeW:0x44adeb	1	0	0	1	m_file_Export.dL_0378_0523
:shell32.dll!PathCleanupSpec:0x81a2bf	1	0	0	1	m_tools.dL_0397_0428
:kernel32.dll!LoadLibraryW:0x44db70	1	0	0	1	m_view_Toolbars.dL_0419_0266
:insn@0x44b9b0:0x43d093	1	0	0	1	s_theme.cM_0760_0777
```

B4 は『Settings の 9 タブとメニューの全状態』を全部は再実行していない(上記の状態だけ)。

### 6.4 停止率の前後(測った範囲での読み)

基準 61/8,552 = 0.71%。再実行で消えた物: 右 Alt の 5 件、B4 で prcDirty の 19 件。**全 8,552 件を再走査していないので、『修理後の停止率』は測定値ではない**。再実行した範囲での『停止 → 通過』を基準に引いた読みは (61 − 24)/8,552 = 0.43%(B3 の 5 件 + B4 の prcDirty 19 件。標本で確かめた範囲に退行は 0 件)(通過を標本で確かめた範囲に退行 0 件、という仮定を置いている)。

## 7. 他担当・PM への依頼(自分では直さなかった壁と、既存ファイルへの依頼)

指示により、同時に動いている担当のファイル(`fle_win.c` `fle_gdi.c` `fle_raster.c` `fle_gdc.c` = present2/txt1、`fle_filedlg.c` = opfs1、`fle_monitor.h` `fle_screen.h` = screen1、LCMapStringW・shell32 Drag・menu/ImageList = crash1、`fle_input.c` の translate_mouse/defproc = win2)は編集していない。**例外はない**(`fle_input.c` に末尾 5 行を足す案は、ToUnicode の表を kbdus.c から作ったので不要になり取り下げた。`fle_input.c` の差分は 0 行)。PM の指示で編集した既存ファイルは `fle_layered.c`(§5.4)と `fle_class.c`(§5.5)の 2 つだけで、それぞれ別の diff に分けてある。

| # | 依頼先 | 壁・内容 | 件数 | 提案(根拠) |
|---:|---|---|---:|---|
| R1 | crash1 | `shell32!DragAcceptFiles` @0x827593 | 25 | crash1.diff の行と `fle_crash1_drag.c` を出荷に入れる。入れた後の『次の壁』を測るためのビルド B2(= B3 + crash1.diff。測定専用、`src-c1`、行は `tables/sweep1-imports-final.txt` + crash1 の 18 行)の準備まではしたが、**時間切れで走らせていない**(未測定)。crash1 自身の記録では、Envelope タブの先は停止しない(NC1)が、つまみのドラッグは Elastique の壁 |
| R2 | (解消。PM の指示) | `UpdateLayeredWindowIndirect` の `prcDirty` @0x80dd3b | 21 | **§5.4 で実装した(`sweep1-layered.diff`)**。座標系はページに無く、窓自身の座標と読んだ(S-sweep1-ulw-1)。Wine の表面との差分は未取得 |
| R3 | `fle_reg.c` の持ち主 | `advapi32!RegOpenKeyExW`: キーパスの空の要素(先頭・末尾・二重の `\`)| 1 | Learn に記述なし(Microsoft のブログは二重の `\` を不正とする)。末尾の `\` を許す選択(Wine 10.0 の挙動)か、ERROR_FILE_NOT_FOUND を返す選択か。どの経路かは F10 MIDI タブの更新ボタン付近 |
| R4 | `fle_locale.c`(crash1 が同じファイル) | `kernel32!CompareStringW`: ASCII 英数字以外の文字を含む不等な文字列 | 1 | ia2 の Sorting Weight Table(`fle_ia2_sortw.inc`)にある範囲を超える文字。表の拡張 |
| R5 | `fle_vfs.c` の持ち主 | `version!GetFileVersionInfoSizeW`: ディレクトリ部の無いファイル名(LoadLibrary の検索順序は未モデル、決定 0098) | 1 | Learn: 『ファイル名にパスが無ければ… の検索順序』を実装する |
| R6 | shell32 の持ち主 | `shell32!PathCleanupSpec`: 不正文字の置換規則 | 1 | ページは不正文字を列挙するが置換先は書かない。選択が要る |
| R7 | txt1 | `gdi32!ExtTextOutW` グリフの描画(決定 0111) | 1 | txt1 の担当 |
| R8 | dec1 | LoadLibraryW `elastique_x64.dll`(Sampler のつまみのドラッグ)、`ilsvg_x64.dll`(View>Toolbars) | 2 | モジュールの供給 |
| R9 | `fle_input.c`(win2) | TranslateMessage の `vk_to_char` と本族の kbdus.c の表の差: Ctrl+Enter('\n' vs '\r')、Shift+Tab、テンキー(`VK_NUMPAD0..9`、`*` `+` `-` `.` `/`)に文字が出ない、Ctrl+Backspace(0x7f)。MapVirtualKeyW(同ファイル)に VK_NUMPAD0 の走査コード 0x52 が無い | - | `fle_sweep1_kbd.c` の `us_char`/`vk_key_scan` を共有する形にすれば、表は 1 つになる(`fle_input.c` の `vk_to_char` を `us_char` の薄い呼び出しにする)。win2 が `fle_input.c` を編集中なので私はしていない |
| R10 | 調査(未特定) | Settings Theme タブの中ボタンクリックで『呼び出し先 0 の間接呼び出し』@0x43d093 | 1 | CompareStringW の列の後(Delphi の並べ替えのコールバックが 0)。再現入力は `tables/sweep-B0.tsv` の `s_theme.cM_0760_0777` |
| R12 | `fle_win.c`(present2/txt1) | CreateWindowExW が未登録のクラスで NULL を返すとき、最終エラーが設定されない(実システムは ERROR_CANNOT_FIND_WND_CLASS = 1407) | - | `fle_class.c` の除去(§5.5)で、T + 大文字以外の未登録名も NULL になった。最終エラーの設定を足す |
| R11 | 調査(未特定) | Sampler 波形の中ボタンクリック (463,491) が CPU 900 秒でも終わらない(§5.6) | 2 | ゲストのループ。原因の API は未特定。次の一手は『`call [IAT]` も記録するトレース』 |

## 8. 余計な処理の候補(lift の忠実性の方針、2026-11-04。1 は除去した、他は読んだだけ。`tables/extra-logic-candidates.tsv`)

方針『FL の挙動を補う・代行する・特別扱いする処理は余計』に照らして、走査の途中で読んだランタイムとビルドの設定から見つけた候補:

1. **(除去した。§5.5、`sweep1-extra.diff`)** `lift/runtime/fle_class.c:121-136`: GetClassInfoW とクラス検索が、クラス名が **『T + 大文字』の形(Delphi のクラス名)か、ゲストの 7 つの名前**のときだけ『見つからない(0)』を返し、それ以外は停止する。ファイル自身が『名前の形を読んでいる、出典のない選択(S-classname)』と書いている。Learn の契約は『登録されていない、システムクラスでもない名前はすべて 0』。除去案: `is_own_static_name := !is_system_class_name(name)`。PM の指示で除去した(`class.rs` 等 7 検査を契約に追従させた)。
2. `lift/runtime/fle_win.c:542-590`: クラス名 `TFruityLoopsMainForm` が最初に可視になった時に窓表を 1 度出力する(診断だけ。挙動は変えない)。除去してよい(窓表は G3 のスナップショットで取れる)。`fle_win.c` は present2/txt1 の編集中。
3. `lift/runtime/fle_hle.c` と出荷設定 `HLE_DSP=1`(`app/scripts/build-engine-bundle.sh` L57): lift した FL / IPP の関数を手書きの C で置き換える(f_6a77b0 の pixscale ほか)。『FL の lift 済みコードが行うべきことの代行』にあたる。速度のための選択。HLE_DSP=0 で出荷バンドルを測り直し、必要な分は lift 側(コンパイラ)で速くする案。
4. `PRUNE_LIST`(prune-S/SM.tsv: dsp_ippv2 の関数を lift から除く)と `B2D_SWAP=1` / `B2D_UPSTREAM_*`(Blend2D を別のビルドに差し替える): 到達しない関数の除去と代替は FL の挙動の観測ではなく大きさ・速度の測定。決定 0168 が『条件付き』としている。
5. `fle_wall_*.c`: 壁にあたった呼び出し元(FL の呼び出し箇所)に名前を付けた API 応答。Learn の契約を実装している限り問題ない。今回の修理は『壁の API の族を全部』にしている。

## 9. 検証(同じ木 = `git archive 55e4e83` + sweep1 の 3 つの差分。HEAD 3e4f540 には `patch -p1 --dry-run` が当たる)

| 項目 | 結果 |
|---|---|
| FLEngine の既定の生成 C(`fe-lift lift FLEngine_x64.dll OUT.c --pdata`、fe-lift は無変更) | sha256 = `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B。**変わらない**(`logs/felift-default-sweep1.sha`) |
| 全体テスト `bash lift/harness/test.sh full --jobs 2`(`FLE_INPUT_ROOT` = 本リポジトリ。load 40〜70) | **results = 184、passed = 2,311、failed = 0**、SKIPPED 行 15(基準 183 / 2,307 / 0 に、`sweep1.rs` の 4 件が増えた。書き換えた 8 検査〔`class.rs` 4、`wall_fr7.rs` 2、`window.rs` 1、`wall_fr16.rs` 1〕は件数が変わらず通過)。`logs/test-full3.txt`。壁時計 537 秒 |
| 新しいテスト `sweep1.rs` | 4 件通過(行の新規性・宣言・定義、Wine 10.0 と同じ出力〔差 126 キーは理由つき〕+ native 出力の固定、名前付き停止 12 件、ヘッダと行の一致) |
| 起動 16 スナップ(82 行、`cmp-short.g3`、native -O0) | B0(sweep1 の行なし)と B1(第 1 版の 8 行)は**同一**(`tables/cmp-short-82-B0.txt` = `...-B1.txt`)。B4(= B3 + layered + extra、最終のコード)も B0 と同一(`tables/cmp-short-82-B4.txt`、`cmp82.sh` の出力は『IDENTICAL』)。なお、native の 82 行は `lift/harness/b2d2/expected/` の node の表とは別物(B0 自身が node の表と一部の画面ハッシュで異なる: 表面+窓表の 50 行中 15〜17 行)で、比較は native 同士 |
| a4 FRUITY 無効(942/942、895/895、functions 5,310、entries 1,009,731) | a4 FRUITY 無効、ia2 方式のモジュール表(`tools/a4off.sh`)の木 src(HEAD 55e4e83 + sweep1 の 3 つの差分): functions_distinct 4,209、function_entries 852,104、停止 `VirtualProtect: address is not in a VirtualAlloc region`、exit 134。**これは act1 が同じ方式のスクリプトで基準の木に対して得た値と同一**(act1 の `logs/a4off.txt`、ファイル名は act1 自身が `a4off-ia2script-NOT-THE-REFERENCE` と注記している別名の方式)。**基準値(942/942、895/895、functions 5,310、entries 1,009,731 = act1 の `logs/a4-fruity-off.txt`、`out-act1/a4ref`)の構成での再実行は、ビルドに 17 分かかり間に合わなかった**(未測定)。同一の値であることは、sweep1 の runtime の変更(`fle_layered.c`、`fle_class.c`)がこの走りの経路を変えないことを示す |
| 行を足したビルドの a4(FRUITY 有効、`functions_distinct`/`function_entries`) | B0・B1・B3・B4 とも **5,398 / 1,052,558**(行を足しても同じ) |

## 10. 出典のない選択(S-)の一覧(`tables/s-choices.tsv`)

| ID | 選択 | 理由 | 代替 |
|---|---|---|---|
| S-sweep1-1 | grid step 16 px, 8 px for a window with a side under 48 px; points are cell centres of the clipped window rectangle; a point belongs to the topmost window that contains it | the brief fixes 16 / 8 px; 48 px is mine (a 16-px grid would give a window under 3 cells in a side fewer than 9 points) | 32 px grid (4x fewer probes); the window's centre only |
| S-sweep1-2 | settle time after an input: 60 virtual ms (ia1/ia2: 700) | measured cost: the engine runs at about 10 CPU ms per virtual ms, so 700 ms costs 4.2 CPU s and 60 ms about 0.8 s; the stops of the earlier waves were reached inside the input handling (read from their logs, not re-measured here) | 100-200 ms (1.1-2.0 CPU s) |
| S-sweep1-3 | sampling: the enumerated points are thinned by a stride per state; the input kind of probe j is j mod 7 (points) and the combination j mod 27 (drags) | a full enumeration is about 15e4 CPU s; every kind is still seen in every state, but no point is seen with all kinds | random sample with a seed; stratified by window class |
| S-sweep1-4 | windows identical (lid, class, rectangle) to the base state's are scanned in the base state only; the states that add a window (F-keys, menus, Settings tabs ...) scan the new / changed windows | a click on an unchanged control while another window is open is assumed to behave as in the base state (not verified; a modal window changes it) | scan every window in every state (about 40x more probes) |
| S-sweep1-5 | state recipes: menu bar x positions and sub-menu rows are the ones read from pictures in decision 0191 (ops2.py); Settings tab x offsets and the browser clicks are read from pictures of this run (runs/E2 dumps); channel button (522,145) | the menu is drawn by the engine itself (class TQuickPopupMenuWindow), so there is no HMENU to enumerate; the recipe only has to get into the state, the probes inside it are mechanical | enumerate the bar with the same grid |
| S-sweep1-6 | CPU limit of a probe 90 s (rlim_cur = rlim_max, the kernel then sends SIGKILL = 'timeout'); 900 s for the confirmation of the two timeouts | about 40x the average probe | - |
| S-sweep1-7 | the wintable of the state is taken from one child per state (prep run); the hit test treats child rectangles as relative to the parent's origin (no non-client area: the engine's forms are popups without caption or border) | read from the window styles in the tables (0x9c030000, 0x94000000) | ask the runtime for the hit window (a new G3 command) |
| S-sweep1-8 | a probe that ends with exit 0 but without its end snapshot is 'engine-exit' (the engine ended by itself, e.g. the close button or File > Exit) and is not counted as a stop | closing the program is a normal user action | count it as a stop |
| S-sweep1-kbd-1 | ActivateKeyboardLayout with an identifier that is not the loaded one: 0 with ERROR_INVALID_KEYBOARD_HANDLE (1457) | the page names no error code; 1457 is the System Error Codes entry "Invalid keyboard layout handle"; Wine answers 120 (a stub) | 0 with ERROR_INVALID_PARAMETER; stop |
| S-sweep1-kbd-2 | ActivateKeyboardLayout(KLF_SETFORPROCESS) of the loaded layout sends no WM_INPUTLANGCHANGE | the locale does not change, the message is about a change | send the message to the focus window |
| S-sweep1-kbd-3 | ToUnicode/ToAscii: Ctrl together with Alt (the AltGr state) gives no character | kbdus.c CharModifiers has wMaxModBits 3: only Shift and Control bits have a modification number; the US layout has no AltGr column (UNOFFICIAL: from the structure of the sample); Wine ignores the Alt bit | treat as Ctrl alone / as no modifier (Wine) |
| S-sweep1-kbd-4 | Alt without Control does not change the character (Alt+A gives 'a') | the character of WM_SYSCHAR; Wine agrees (UNOFFICIAL) | no character (the literal CharModifiers reading) |
| S-sweep1-kbd-5 | a letter key with Control (with or without Shift) gives the control character vk - 'A' + 1 | the system's rule for control characters; kbdus.c's two-state letter rows do not list it; Wine agrees (UNOFFICIAL) | no character |
| S-sweep1-kbd-6 | Caps Lock acts like Shift on a CAPLOK row when no Control is down | kbd.h CAPLOK; the sample does not say when the toggle is ignored | always (also with Control) |
| S-sweep1-kbd-7 | lpKeyState NULL (declared [in, optional]) is read as no key down and Caps Lock off | the page does not say what NULL means | stop |
| S-sweep1-kbd-8 | ToUnicode with cchBuff < 1: returns 0 and writes nothing | the character cannot be held; Wine gives 0 | stop |
| S-sweep1-kbd-9 | ToAscii writes the whole WORD (the character in the low byte, the high byte 0) when it returns 1 | the page does not say what the high byte holds; Wine leaves it as it was | write the low byte only |
| S-sweep1-kbd-10 | VkKeyScan search order: modification number first (none, Shift, Ctrl, Shift+Ctrl), table order within a number; control characters 0x01..0x1A that no row lists are Ctrl + the letter key | so that a character a key makes without a modifier is that key (Escape for 0x1b); the page does not say which key is meant when two make the character | table order only |
| S-sweep1-kbd-11 | VkKeyScan: the keys of the numeric pad (VK 0x60..0x6F) answer only when no other key makes the character | the sample's own comment puts the numeric pad last "so that VkKeyScan interprets number characters as coming from the main section"; '*' and '+' are Shift+8 and Shift+= | include the pad in the first pass |
| S-sweep1-kbd-12 | OemKeyScan: an OEM code below 0x80 is the ASCII character (VkKeyScan + MapVirtualKey(VK_TO_VSC)); 0x80..0xFF have no key | every OEM code page is ASCII below 0x80; the accented / box-drawing characters above are not on the US keyboard | map through the OEM code page 437 to Unicode first |

## 11. 限界(読むときの注意)

- **間引き**: 列挙した 16 画素の格子点は状態あたり約 8,000〜11,000 点、実行は 30〜700 件(基準状態で 700 件 = 7%、追加される窓は窓ごとに約 150〜250 件)。入力種は点ごとに 1 種(7 種を巡回)なので、**ある点に 7 種すべてを当てた物は無い**。ドラッグは副格子 684 点のうち 171 点。窓のすべての部品を踏んだ保証は無い。停止率 0.71% は『この間引きの標本での割合』であり、全域の割合の推定ではない。
- **状態**: 12 種の状態の系列だけ(ダイアログを開いた状態〔File>Open など〕、再生中、Browser のフォルダを展開した状態は走査していない。再生中 `play` と F5/F6/F9 は基準状態と窓が同一で、別の走査をしていない = 再生中の操作は未走査)。『メニューを開いてから項目を選ぶ』は項目ごとに別の窓(File>Open が開くダイアログなど)になるが、その先は走査していない。
- **待ち 60 仮想ミリ秒**: 入力から 60 ms 後に出る遅延した停止(タイマー)は取りこぼす。
- **native -O0 の観測**: 仮想時間で動く。ブラウザの実時間・Worker・描画の経路は無い。Firefox は未実施。
- **Wine 10.0 との差分**は Wine の挙動(実機 Windows ではない)。差のある所は出典つきの理由(`expected/known-divergence.txt`)。実機 Windows は未確認。
- **CPU 超過 2 件の原因 API は未特定**(§5.6)。
- 負荷: 走査中の load は 20〜70。時間(壁時計)の数字は参考値で、速度の主張ではない。

## PM の決定と取り込み(2026-11-04)

取り込んだ基準: HEAD 825fa28 に `sweep1.diff`・`sweep1-layered.diff`・`sweep1-extra.diff` をこの順に `patch -p1` で当てた(offset は extra の 1 hunk だけ、fuzz・reject なし)。`-` 行(3 + 14 + 82)は全部読んだ。PM が測った値(この機械、`lift/out-pmchk-s1/`): `cargo build --release -p fe-lift` 成功、FLEngine の既定の生成 C の sha256 **`cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(215,811,678 B)のまま**、a4 FRUITY 無効 **init_count 942/942、run_rank 895/895、functions 5,310、entries 1,009,737**(txt1 以後の基準と同じ。再実行できなかったという担当の穴はこれで埋まった)。全体テストは **189 results / 2,340 passed / 2 failed**:失敗は `fe-lift/sweep1` の 2 件だけで、原因は**この差分のテストが、screen1(決定 0212)が `fle_input.c` に足した参照 `fle_win_input_toplevels`・`fle_win_input_remaximize` のリンク用スタブを持たなかったこと**(担当の基準 55e4e83 には無かった参照)。PM が `harness/sweep1/sweep1_cases.c` と `tests/sweep1.rs` のスタブ表に 2 行ずつ足し(挙動なし、スタブのみ)、`cargo test --release -p fe-lift --test sweep1` が **4 passed / 0 failed**。他の 188 結果は失敗 0。

1. **user32 のキーボード層の族 18 関数(`fle_sweep1_kbd.c`、`sweep1-imports.txt` 18 行)を採る。** Learn と Microsoft の kbdus.c(MS-PL)から、`fle_input.c` は 0 行、FL 側の特別扱いなし。Wine 10.0 との差分 1,231 ケースで差のある 126 キーはすべて理由つき(Wine は実機 Windows ではない)。ship3 の EXTRA_IMPORTS には**最後に**足す(`ship2-imports.txt` → `dec1-imports.txt` → `crash1-imports.txt` → `api2-imports.txt` → `sweep1-imports.txt`。前の行の疑似 VA が動かない)。依頼者の実セッションの `ToAscii` 停止(guest return 0xa0c2c6)は、native の 6 状態 × 4 キーでは再現しなかったので、**この修正で直ったとはまだ言えない**。ブラウザでの再確認は ship3。
2. **`prcDirty`(`sweep1-layered.diff`)を採る。** 決定 0140 の名前つき停止を、Learn の記述どおりの矩形限定更新に置き換えた。座標系は窓自身の座標と読んだ**出典のない選択**(S-sweep1-ulw-1)。Wine の表面との差分は未取得なので**仮**。desk1 のコンポジタが layered 表面を読むので、面の持ち方・dirty の記録の形は変えていない(担当の確認)。
3. **クラス名の形による分岐の除去(`sweep1-extra.diff`)を採る**(方針:『FL のクラス名による分岐は余計なロジック』。『T + 大文字』の形と 7 名の表が決定 0121/0123 の出典なしの読みだった)。未登録でシステムクラスでもない名前は Learn の GetClassInfoW どおり『見つからない』を返す。システムクラス(Button, Edit …)は引き続き名前つきで停止。
4. **`probejobs`(`fruity_g3.c`、fork プローブの並列化)はハーネスのみ(`FLE_G3_FORKPROBE` のときだけコンパイル、既定 N=1 で従来の動作)。採る。**
5. **未解決(次の作業の入口)**: ① Sampler 波形(463,491)の CPU 超過 2 件(VCL の `CM_ACTIONUPDATE` 0xB03F を木の全ノードに送る走査が戻らない、と静的に読んだ**推測**。parity1 が見つけた『Basic.flp を開いたあとミキサー操作の列でエンジンが戻らない』と同じ型の可能性があるので、両者を突き合わせる)。② Piano roll の F12 → `user32!DispatchMessageA`(`fle_win.c` の層。win2 の担当範囲)。③ 依頼者の (b) General タブ・(c) Browser フォルダ展開・(d) Playlist 中ボタンドラッグは native で再現せず(c は crash1 の LCMapStringW で説明がつく)。(b)(d) の原因は**未特定のまま**で、API 拡充で直ったとは言わない。④ crash1 を入れたビルドでの再走査(DragAcceptFiles の先の壁)と再生中の状態の走査は未実施。
6. 余計なロジックの候補表(`tables/extra-logic-candidates.tsv`):`fle_win.c:542` の TFruityLoopsMainForm 診断は win2 に、HLE_DSP=1 は parity1 が不要と示したら外す(HLE は出力同一の検査つきの近道という位置づけ)、PRUNE_LIST と B2D_SWAP は維持(FL の挙動を変えない)。
