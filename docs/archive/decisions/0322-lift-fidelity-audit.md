# 決定 0322(ci1): lift 忠実性の静的点検

**実エンジンの実行は 0(読みだけ)。** 実行したのは `grep`・`sed`・`awk`・python(S 印の集計)・`git archive` + `patch --dry-run`(差分案 2 本)だけ。Chrome・Firefox・Wine・clang・rustc・wasm-opt・emcc・node の書き出し・テスト・cargo・vite build は 0 回。測定値は 0 件(下の数字はすべて「数えた件数」か「決定文書から転記した値」で、後者は決定番号を付けた)。依頼者の機械の値は 0 件。

基準: HEAD `3969ad31`(`git archive` の木)。期限 08:30(機械の時計)。
範囲: `app/src`(78 ファイル、11,435 行)、`lift/runtime/*.c *.h`(204 ファイル、57,486 行。`third_party/` は除く)、`app/worker` `app/shared` `app/tools` `app/scripts`(製品の経路にある物だけ見出しを確認)。音の経路(`audio/*`、`fle_dsound.c`、`fle_audio*`)は mt3 の担当なので表に入れない(件数だけ付録に載せる)。

## 0. 結論(先頭の数行)

1. **FL のクラス名・窓の種類・題名・位置による分岐は、製品の経路に 0 件。** ページ(`app/src`)に FL のクラス名の分岐はない(`TFruity…` は `engine/os-state.ts:2` のコメント 1 件だけ)。ランタイムの `"TFruityLoopsMainForm"`(`fle_appfile.h:13`)はテスト用の入口(`fle_appfile_open`)でしか使われず、製品のページは呼ばない(`app/src` に該当の語 0 件)。合成器(`fle_desk.c`)はクラス名・題名を診断の文字列にだけ使い、描画の判断には読まない(コメントと grep の両方で確認)。
2. **「余計な処理」と読めた物は 3 群。** (A) 診断を製品の描画経路で毎回作っている(`fle_desk.c` の `names_of` + `build_desc`、毎 compose・毎 scene)。(B) 死んだ設定: ページが `FLE_SCREEN_REMAXIMIZE=1` を渡すが、ランタイムは誰も読まない(決定 0243 が読む側を消した)。コメントはまだ「opt-in」と言う。(C) 入力の間引きが 2 段(ページの `IdleDriver` と ランタイムの `S-mousemove-coalesce`)。差分案は (A) と (B) の 2 本、**適用していない**(`diffs/`)。
3. **「契約の不足(外部 API の実装不足)」と読めた物が上位 6 件**(表 6): (i) `WH_MOUSE` / `WH_KEYBOARD` のフックを記録するだけで呼ばない(前提「入力が無い」は決定 0136 で崩れた。再検討の条件が満たされたまま)、(ii) `HTTRANSPARENT` で入力を捨てる(Learn は「同じスレッドの下の窓へ送る」と書く)、(iii) `DefWindowProc(WM_ERASEBKGND)` が何も描かない(クラス表は `WNDCLASS` の全バイトを保存する〔`fle_class.c:74-85`〕が、背景ブラシを消去に使う所は 0 件)、(iv) 最大化窓の再配置が差分(dw, dh)の加算で、`WM_GETMINMAXINFO` を通らない(推測と明記済み: S-screen1-12)、(v) 合成器が最上位から 64 個で切る(65 個目以降の可視窓は描かれない)、(vi) タスクバーのボタンが 1 個。
4. **「成功を返す」既定は多いが、ほとんどは文書の契約(Learn が値を書いている場合)か、Learn が沈黙している所での選択(S 印)で、止める(`fle_stop`)方針が一貫している。** 画素規則の未実装(模様ブラシ・太い線・破線・帯状の `StretchDIBits`)は黙って通さず、名前を付けて止める。
5. **場当たりな間引きと頻度の制限は、表 5 に 12 行**: 入力の移動の合流(2 段)、起動前の入力の破棄、仮想時計の遅れ上限(1000 ms)と進み上限(100 ms)、描画の 100 ms 眠り、アイドルからの描画 20 ms、リフレッシュ率を 120 Hz に切る(決定 0268)、`INLINE_TRY_MS` 6 ms、報告の 40 ms 間隔、`RoomDebouncer` 300 ms。
6. 付録: S 印(「出典なしの選択」)は ランタイムに **528 箇所・388 種・75 ファイル**、PROVISIONAL は **363 行・114 ファイル**(いずれも `grep` の件数)。全件の表は `census-S-markers.tsv`(自動生成)。主表(約 50 行)は私が文脈を読んで区分した行だけ。**読んだ範囲は ランタイム 57k 行のうち数千行**(下の「読んだ範囲」)。読んでいない所の「問題なし」は言っていない。

## 1. 区分と読み方

| 記号 | 意味 |
|---|---|
| (a) | 我々の余計な処理(FL の lift 済みコードがやるべき事の代行・補正・上書き、場当たりな近道)。抹消の候補 |
| (b) | 外部 API(Win32 など)の契約の実装、または契約の不足。除くと壊れる物は (b) の不足の表れ(方針 2) |
| (c) | wasm↔JS の変換と Worker↔メインの通信の削減(方針の性能の節)。出力が同一のもの |
| (d) | 診断・計測・ログだけ。製品の経路から切り離す対象 |
| (?) | 読みだけでは決められない。何を測れば決まるかを書く |

- 「根拠」は決定番号。コメントだけで決定番号が無い場合は「コメントのみ」、どこにも無い場合は「根拠なし」。
- 「除くと何が壊れる(仮説)」は**実行していない予想**。除去の実験は私がやらない(窓の指示待ち)。壊れる物が出たら (b) の不足として API を拡充する(方針 2)。
- 行番号は HEAD `3969ad31` のもの。

## 2. 表 1: FL のクラス名・窓の種類・題名・位置・大きさによる分岐

| ID | 場所 | 内容 | 区分 | 根拠 | 除くと何が壊れる(仮説) |
|---|---|---|---|---|---|
| 1-1 | `lift/runtime/fle_appfile.h:13`、`fle_appfile.c:107-115` | クラス `TFruityLoopsMainForm` の最上位窓を探して `WM_COPYDATA` を送る(FL64.exe の二重起動の経路をホストが演じる) | (d)。テスト・ホストの入口。製品のページは呼ばない | 0144 | G3 の `open` コマンドと g8 のテストが -1 を返す。製品への影響は無い(読み。呼び出し元は `lift/harness/fruity_g3.c:556` と `lift/harness/web/g8/g8_scenarios.c` だけ) |
| 1-2 | `app/src/engine/appview.ts:34-52`、`lift/runtime/fle_appview.c:89` | 窓一覧のクラス名(32 バイト)と題名(64 バイト)を、一覧が変わるたびに全窓分 latin1 で文字列にする | (d) を製品の経路で行っている / (c) の余地 | 0141、0170(seq が同じなら読まない) | 製品が使うのは「active かつ visible で題名が空でない最初の窓」の題名だけ(`os-state.ts:14-17`)。クラス名は診断パネル(`ui/panels.ts:11`)だけ。除くとパネルの「class」列が空になる |
| 1-3 | `lift/runtime/fle_desk.c:46,310-320,340,416-432,476,703` | 毎回の compose と scene で、可視の窓ごとに `names_of`(窓の表を引いてクラス名と題名を ASCII 化)と `build_desc`(最大 64 行の `snprintf`)を実行する。`dwin` に 128 バイトの文字列が入り、`memset` と `memcpy(prev, cur)` で毎回動く | (d) を製品の描画経路に置いている。**(a) に近い**。コメントが「診断のみ。合成器は読まない」と認めている | 0223、0244 | `fle_desk_describe`(テストの補助)を呼ぶテストが影響を受ける。描画は変わらない(読み)。コストは**未測定** → 差分案 D2 |
| 1-4 | `app/src/engine/os-state.ts:14-17`、`worker.ts:396-402` | `document.title` を「active・可視・題名が空でない最初の窓」の題名にする。空題名の窓(メニュー・ツールチップ)で題名が消えないよう、直前の題名を保つ | (?)。ページが窓の一覧を読む点は方針「ページは窓の知識を持たない」に触れる。FL の挙動には影響しない(ブラウザのタブの題名だけ) | 0199、0223 | タブの題名が FL の前面窓の題名に追従しなくなる。FL 自身の挙動は変わらない。置き換え案: ランタイムが「前面窓の題名」を 1 本の export で返す(GetForegroundWindow + GetWindowText の契約)。空題名を飛ばす規則は、その場合ランタイムかページのどちらかに必要か要検討 |
| 1-5 | `app/src/ui/screen-fit.ts:6`、`input/events.ts:10`、`ui/desktop.ts` | 画面の大きさの既定 1920×1080(窓ではなく画面) | (?)。分岐ではなく既定値。エンジンが画面の大きさを報告するまでの仮の値 | 0098、0212 | 報告前のキャンバスの大きさ。報告後は無関係 |
| 1-6 | `lift/runtime/fle_vfs.c:1688`、`fle_env.h:40,45` | 仮想ボリュームの路 `C:\Program Files\Image-Line\FL Studio 21\…`(インストールの配置のデータ) | (b)。環境のデータ。分岐ではない | コメントのみ(決定番号は未確認) | FL の起動が資源を見つけられない |

確認の方法: `grep -nE "className|\.cls\b|TFruity|isMain|mainWindow" app/src/engine/{worker,desk,desk-gl,idle-driver,fruity}.ts app/src/input/*.ts app/src/ui/{taskbar,desktop,cursor}.ts app/src/main.ts` は題名・`className` の使用 4 件(1-2、1-4、main.ts の DOM の `className`)だけを返した。`strcmp|wcscmp` を `fle_win.c fle_desk.c fle_input.c fle_layered.c fle_dwm.c fle_appview.c fle_visrgn.c fle_gdc.c fle_monitor.c` に掛けた結果は、窓の分岐に使う比較 0 件(診断のコピーだけ)。

## 3. 表 2: FL の窓管理の代行(前面化・閉じる・配置・可視・窓ごとの canvas の順序)

| ID | 場所 | 内容 | 区分 | 根拠 | 除くと何が壊れる(仮説) |
|---|---|---|---|---|---|
| 2-1 | `lift/runtime/fle_win.c:6748-6760`、`fle_input.c:691`(`apply_display`) | 画面の大きさが変わると、最大化状態の最上位窓すべてに「位置は同じ、大きさは今の大きさ + 画面の変化(dw, dh)」を `apply_pos` で与える | **(?) 推測と自称**(`docs/investigations/screen1/tables/s-choices.tsv` の S-screen1-12:「Windows が最大化窓を再最大化する、は私の推論。Learn は読んでいない」)。実際の Windows は `WM_GETMINMAXINFO` から最大化の矩形を計算し直す、と私は理解している(未確認) | 0212 | ブラウザの窓を変えても主窓が追従しなくなる(FL 自身は再配置しない: 同表「FL は自分で再フィットしない(測定)」)。(b) の正しい形は、`ShowWindow(SW_SHOWMAXIMIZED)` と同じ計算(`get_min_max_info`、`fle_win.c:3537`)を再実行すること |
| 2-2 | `lift/runtime/fle_input.c:742` | `S-mousemove-coalesce`: 直後にまた移動が続く移動を捨てる | (b)。実 Windows は未取得の移動を 1 つにまとめる(QS_MOUSEMOVE)ので結果は近い。「どうまとめるか」を Learn は書かない、とコメントが認める | 0136 | WM_MOUSEMOVE が増え、負荷が上がる。見える変化は予想しない。ページ側にも同じ合流がある(表 5 の 5-1)ので、**二重** |
| 2-3 | `fle_input.c:509` | `S-httransparent`: `WM_NCHITTEST` が `HTTRANSPARENT` を返すと、そのイベントを**捨てる**(下の窓を試さない) | **(b) の不足**。Learn の原文(`docs/investigations/alpha6/tables/learn/wm-nchittest.txt:58-59`)は「同じスレッドの下の窓へ送る。`HTTRANSPARENT` 以外が返るまで」と書く | 0136、`docs/investigations/g3in/logs/S-table.md` の再検討条件「下の窓を試す実装を入れるとき」 | 下の窓を試すと、透明窓の下の窓がクリックを受ける。FL が `HTTRANSPARENT` を返す窓を持つかは未確認 |
| 2-4 | `fle_input.c:453,731` | `S-no-target`: どの窓にも向かない入力を捨てる | (b)。S 印の表(`g3in/logs/S-table.md`)は「Windows はデスクトップへ送る」と認めた上での選択。FL の窓が無い所の入力で、影響は小さいと読む | 0136 | 影響なしと読む(未確認) |
| 2-4b | `fle_input.c`(`S-disabled-drops`) | 無効な窓(祖先が無効を含む)の上のマウス入力を捨て、下の窓にも親にも渡さない。実例: ゲスト自身が `WS_DISABLED` を付けた `TWPControl`(lid 129)の中の Buy ボタン(lid 153)への押下が捨てられる | (b)。S 印の表は「`WindowFromPoint` の頁を文字どおり読むと、無効な子の下の有効な祖先が答え」と自分で注記している。再検討の条件「親へ渡す実装に変えるとき」 | 0136 | 無効な窓の押下が親に届く。実 Windows の動作は**未確認** |
| 2-5 | `lift/runtime/fle_win.c:6713-6731` | `S-key-target-fallback`: 前面窓が決まっていないときは、フォーカス窓を持つ最初のスレッドがキーを受ける | (?)。「SetFocus と SetActiveWindow が前面窓を決めない」というモデルの穴を埋める推測。実際は前面窓がシステムで 1 つ | 0218、コメントのみ | 前面窓が無い間、キーが捨てられる。クリックでの活性化は `fle_win_input_activate`(`fle_win.c:6778`)が `fg_window` を設定するので、普段はこの分岐に入らないはず(読み。入る頻度は未測定) |
| 2-6 | `fle_win.c:3842`(`raise_owned`)、`3397-3423`、`3587-3618` | 活性化での前面化、所有窓の上げ、`SWP_*` の通知の有無、最小化の矩形。Learn に沿って実装し、メッセージの順だけ「読み」 | (b)。窓管理は FL が持ち、ランタイムは契約のメッセージを送る形になっている(決定 0222 の結論)。`raise_owned` は所有窓を 64 個で切る(`owned[64]`) | 0222、0258、0267 | 前面化と閉じるが効かなくなる(0222 の元の不具合) |
| 2-7 | `fle_win.c:6668`(`S-taskbar-owner`)、`app/src/ui/taskbar.ts`、`worker.ts:404-406` | 最小化窓のタスクバーのボタンを、ページが **1 個**だけ出す(「最初の最小化窓」)。押下は `WM_SYSCOMMAND(SC_RESTORE)` | (b)。OS の部品(シェル)の実装。ただし 1 個 | 0267、0272 | 最小化窓が複数あるとき、2 番目以降を戻せない(読み。未確認) |
| 2-8 | `lift/runtime/fle_desk.c:402-412` | z 順が変わった所から上の窓すべてを「描き直しの対象」にする | (c)。出力は同一(下の窓から描く) | 0223 | 保守的な描き直しが減る。画素は変わらない |
| 2-9 | `app/src/engine/desk-gl.ts:4-5`、`worker.ts:146-147` | 「窓を知らない。クラス名・種類・位置を見ない」と明記 | — | 0240 | 確認: `className` `cls` `title` の使用は `desk.ts` `desk-gl.ts` に 0 件 |
| 2-10 | `fle_desk.h:33`、`fle_desk.c:324-325,578`、`app/src/engine/desk-gl.ts:10-11` | **合成器は最上位から 64 個の最上位窓で切る**(`fle_win_input_toplevels(hs, 64)` は z 順の上から 64 個を返す)。65 個目以降は、可視でも描かれない。scene の窓 64、更新 64 も同じ。切り捨ては黙って行われる | **(a)/(b) の疑い。条件付きの黙った欠落** | 0223(上限の根拠は未確認: 根拠なし) | 生きている最上位窓が 64 個を超えると、z 順の下(主窓は下にある)が消える。実走行での最大数は**未確認**。`AppView` の上限は 32(`fle_appview.h:12`)で、決定 0170 は当時の「全窓 114 個」と書く(その時の上限と、最上位だけか子を含むかは未確認) |

## 4. 表 3: 画素・矩形の補正(重複排除・スキップ・間引き・手動の損傷・独自の再描画/非表示)

| ID | 場所 | 内容 | 区分 | 根拠 | 除くと何が壊れる(仮説) |
|---|---|---|---|---|---|
| 3-1 | `lift/runtime/fle_desk.c:486-575`(`sh_*`、`sh_trim`、`trim_row`) | 窓ごとに「ブラウザが持つ画素」の影(全面のコピー)を持ち、書かれた矩形が影と同じなら報告しない。差のある範囲の外接矩形だけを報告する | (c)。出力は同一。方針の点検表は「画素の重複排除は抹消の候補。通信・変換の削減で置き換えられるか評価」と言う。決定 0245 の実測では、FL がピッカーで同じ画素を 62.5 回/秒書き直す(FL 自身の挙動)のを、native の合成を 8.87 → 0.016 ms/フレーム、Chrome の Worker CPU を 973 → 585 ms/s に下げた(決定文書の値、25 組で画素の hash 同一) | 0245、0245b | 同じ矩形を毎フレーム GPU に送る(画素は変わらない)。影のメモリは窓の面と同じ量が増える。**この群は方針上「残す理由が測定で示されている」ので抹消を勧めない**。置き換え案は、FL が同じ画素を書き直す源(`RedrawWindow` の過剰、決定 0262 が 2 件直した)を減らすこと |
| 3-2 | `fle_desk.c:75-111`(`list_add`)、`FLE_DESK_MAX_RECTS 32` | 矩形の包含の除去、接する矩形の合併(合併した面積が 1.25 倍以内)、32 個を超えると最も増えない組を合併 | (c)。出力は同一(描き直しは冪等) | 0223 | 矩形が増え、`putImageData` の回数が増える |
| 3-3 | `fle_desk.c:578`(`scene_wrote`) | 窓ごとに書かれた矩形を 1 個の外接矩形に合併する | (c)。出力は同一 | 0240 | 疎な書き込みで、外接矩形の全体を影と比べる量が増える |
| 3-4 | `lift/runtime/fle_win.c:2624-2629`(`S-0247-fold`) | 更新領域は矩形の集合だが、`UPD_PIECES_MAX` を超えると外接矩形に畳む(過大な推定) | (b)。実 Windows は領域を保つ。畳みは過大な再描画になるだけ | 0247 | 再描画が減る。画素は変わらない(畳みは過大側) |
| 3-5 | `fle_win.c:2870`(`S-0247-hidden`、決定 0262) | 不可視の窓は更新領域を持たない。隠された窓の古い更新矩形を捨てる | (b)。Wine 10.0 の挙動に合わせる(UNOFFICIAL) | 0247、0262 | 隠した窓が再表示で古い矩形を再描画する |
| 3-6 | `lift/runtime/fle_gdi.c:1558`、`fle_raster.c:813`(`S-draw-hidden`) | 可視でない窓への描画を捨てて数える | (b)。可視領域が空なので Windows でも描かれない | 0126 | 不可視窓へ描いた画素が面に残る |
| 3-7 | `fle_gdi.c`(`S-stretch-groups` `S-stretch-halftone` `S-line-dda` ほか、付録の GDI 76 箇所) | 引き伸ばし・半調子・線の画素規則を、Learn が書かない所で自分で決めている(`HALFTONE` は群の平均) | **(b) の不足の可能性**。実 Windows の画素と同一かは**未確認**(Wine との比較は UNOFFICIAL) | 0126、0132 | FL の描く画素が 1〜数 LSB 違う可能性。実機 Windows との比較は 0 件 |
| 3-8 | `app/src/engine/desk.ts`、`desk-gl.ts` | ページは矩形をそのまま `putImageData` / `texSubImage2D` に渡す。ページ側の重複排除・補正・隠す処理は **0 件**(確認: `desk.ts` は全文を読み、`desk-gl.ts` は `dedupe` `skip` `unchanged` などの grep) | — | 0223、0240 | — |

## 5. 表 4: 「成功を返す」既定・黙らせる既定

ランタイムの方針は「未対応は黙って成功にせず、名前を付けて止める」(`fle_proc.c:3`、`fle_env.h:60`)で、確認した範囲では一貫している。次は、その例外(成功や 0 を返して先へ進む所)のうち、FL の挙動に影響しうる物。

| ID | 場所 | 内容 | 区分 | 根拠 | 除くと何が壊れる(仮説) |
|---|---|---|---|---|---|
| 4-1 | `lift/runtime/fle_win.c:1483-1487` | `DefWindowProc(WM_ERASEBKGND)` は何も描かず 1(消した)を返す。クラス表は `WNDCLASS` の全バイトを保存する(`fle_class.c:74-85`。`GetClassInfoW` が返す)が、`hbrBackground` を消去に使う所は**0 件**(`grep hbrBackground` の出現はコメント 2 件だけ) | **(b) の不足**。Learn:「DefWindowProc はクラスの背景ブラシで消す」 | 0123(「選択」と自称) | クラスの背景ブラシに頼る窓(VCL 以外の窓、プラグインの窓)が黒(面の初期値 0、`S-surface-init`)のまま残る。**未確認**: Balance の GUI(段階 3 で枠だけ・クライアント領域が未描画、PM の info による)との関係は**読みだけの仮説で、確認していない** |
| 4-2 | `fle_win.c:2327-2342`(`S-hook-no-events`) | `WH_MOUSE` / `WH_KEYBOARD` のフックを記録し、**呼ばない**。理由は「この環境に mouse/keyboard メッセージは存在しない」。決定 0126 の再検討条件は「入力モデルを作るとき」 | **(b) の不足。前提が崩れている**。入力モデルは決定 0136 で作られた。`fle_input.c` に `hook` の語は 0 件 | 0126、0171(`WH_CALLWNDPROC` は呼ぶ) | FL(VCL)が起動時に入れる 2 個のフック(決定 0194 の読み)が反応しない。何を取りこぼすかは**未確認**(ヒント消去・モーダル状態の検出などは私の推測で、未確認) |
| 4-3 | `fle_input.c:828,840,866,948,1007` | `S-mouseactivate-default`(MA_ACTIVATE)、`S-getdlgcode-zero`、`S-mouse-default-zero`、`S-extrainfo-zero`、`S-beep-silent`(音を鳴らさず成功) | (b)。Learn が既定値を書かない所の選択 | 0136 | 既定の応答が変わる。見える変化は予想しない |
| 4-4 | `lift/runtime/fle_crash1_menu.c:600`、`fle_win.c:5506`(メニュー) | `TrackPopupMenu`: 「何も表示せず、誰も選ばず、メニューは閉じられる」。メニューは状態だけで、描かず、追跡しない | (b) の不足。FL のメニュー(VCL の `TPopupMenu`)が本物の `TrackPopupMenu` を呼ぶ場合に限る(FL が使う経路は未確認。ページ上で FILE メニューが開く事実〔決定 0194〕は、FL が自前の窓でメニューを描くことを示すと読む) | 0126、0171 | 本物の `TrackPopupMenu` を通るメニューが現れない |
| 4-5 | `fle_win.c:1563`(`S-displaychange-def`)、`fle_win.c:5916`(`S-flatsb`)、`5899`(`DrawMenuBar` 1) | 既定の応答 0 / 成功 | (b)。Learn が沈黙 | 0212、0126 | 影響なしと読む |
| 4-6 | `lift/runtime/fle_vmap.c:3` | `VirtualProtect` の保護は記録するだけで強制しない | (b)。FL が保護違反に頼る処理を持つかは未確認 | コメントのみ | 例外を使う仕掛けが働かない |
| 4-7 | `app/src/ui/cursor.ts:12-30` | モジュールから読んだカーソルや FL 自作のカーソルは**矢印で表示**(`S-module-cursor-default`)。`IDC_UPARROW` は `default` | (b) の不足(カーソルの絵を ブラウザに渡していない) | 0141 | カーソルの形が FL の物と違う |
| 4-8 | `lift/runtime/fle_gdi.c:1945,3367,3622-3623` | 模様ブラシ・帯状 `StretchDIBits`・破線・太い線は**止まる**(`pixel_stop`) | (b)。黙って通さない方針の例。「止まる」ことを (b) の不足として報告する | 0132 | — |

## 6. 表 5: 場当たりな軽量化(時間・頻度の間引き、描画の頻度の制限、条件付きの省略)

| ID | 場所 | 内容 | 区分 | 根拠 | 除くと何が壊れる(仮説) |
|---|---|---|---|---|---|
| 5-1 | `app/src/engine/idle-driver.ts:119-135`(`S-move-merge`)、`maxQueue 256` | エンジンが忙しい間、連続する移動を 1 つに合流する。256 件を超える入力は捨てる | (a)/(?)。ランタイムにも同じ合流がある(2-2)ので二重。256 の根拠はコメントのみ | 0141、0170 | 忙しい間の移動の列がそのままエンジンの入力キューに入り、キューが満杯になる(`rejected`)。**満杯の上限はランタイム側にもある**ので、この合流を除くには入力の取り込みを速くする必要がある |
| 5-2 | `idle-driver.ts:113-118`(`S-input-before-idle`) | エンジンが最初のアイドルに達する前の入力を**捨てる**(`droppedBeforeIdle`) | (a)。実機では起動前の入力は無い(窓が無い)ので「契約」とも読める。ただし「最初のアイドル」は FL の状態ではなく我々の仕組みの時点 | 0136 | 起動直後のクリックがエンジンに届く。起動画面を指したクリックが後の画面に当たる危険(コメントの理由) |
| 5-3 | `idle-driver.ts:167-178`(`S-clk-maxlag` 1000 ms、`S-clk-maxlead` 100 ms) | 仮想時計が壁時計から 1 秒以上遅れると、遅れを切る(仮想時計は壁時計に永久に遅れる)。進みは 100 ms で切る | (a)。ゲストの時計を壁時計に対してずらす。FL(と音の拍)の時間が壁時計と一致しなくなる。私の測定: E14 の純粋 6 組で `lagCuts` は E13 の 18.2 → P+V7 の 3.8 | 0128、0136、0170 | 切らなければ、長い停止の後に仮想時計が壁時計に追いつくまで期限を続けて配る(時計は壁時計と一致する)。切る代わりに追従を速くする方向が方針に合う、と読む(実験していない) |
| 5-4 | `app/src/engine/desk-sleep.ts`、`worker.ts:41,336-358`(`DESK_SLEEP_AFTER_MS 100`) | 100 ms 変化が無いと、フレームのループを止め、エンジンのアイドルからの確認(20 ms 間隔)で起こす | (a)/(c)。測定値(決定 0223 §12: リフレッシュ率の上限なしで約 4,000 回/秒のループが GPU を動かす)を根拠とする頻度の制限 | 0223 | 眠りを無くすと、変化の無い画面でも rAF を毎フレーム回し続ける。眠りから起こすのが遅い場合、最初の変化が最大 20 ms 遅れる |
| 5-5 | `worker.ts:39,225-226`(`DESK_FALLBACK_MS 20`) | rAF が届かないとき、アイドルから描画を試すのを最大 20 ms 間隔に制限 | (a)。根拠は「測定: エンジンが遅れていると rAF が届かない」(コメントのみ) | 0223 | rAF が届かない間、描画が止まる |
| 5-6 | `app/src/engine/hz.ts:15-17,35-60`(`STANDARD_HZ [60,75,90,120]`、決定 0268) | 測ったリフレッシュ率を標準の率に丸め、**120 Hz を上限にする**。ゲストの `EnumDisplaySettings` と `DwmFlush` の周期がこの値になる | **(a)**。性能のために、ゲストに実際より低いリフレッシュ率を告げる。契約(`EnumDisplaySettings` は表示の実際の率)に反する | 0268 | 144/165/240 Hz の画面で、ゲストの描画周期が上がる。`hz.ts` のコメントは「120 超はゲストの描画周期の分だけ Worker の負荷が増える(約 3 ms/…)」と書く(文は途中までしか読んでいない)。PM の方針は「present の経路が 120 に届くまで 120 を維持」 |
| 5-7 | `worker.ts:141,201-209`(`INLINE_TRY_MS 6`、`INLINE_GIVEUP_MS 150`、決定 0280 L1/L1c、PROVISIONAL) | 入力の後、`settled` が立つまで描画の確認を 6 ms 間隔で試し、150 ms で諦める | (?)。応答を遅延計測に結ぶための仕組みでもある。製品の描画頻度を変える | 0280 | 入力への描画が、次の rAF かアイドルまで遅れる(133 ms の中央値、決定 0280 の元の値) |
| 5-8 | `app/src/engine/report-throttle.ts`、`worker.ts:412-415`(40 ms と 100 ms の間隔) | ページへの報告(アイドル・再開・描画の統計)の間引き | (d)。診断。ページがこれを元に判断するのは「最初のアイドル」と状態表示だけ | 0170 | ページの状態表示が細かくなる。メインスレッドの負荷が増える(6〜8%/コア、決定 0170) |
| 5-9 | `app/src/ui/screen-fit.ts:75-92`(`RoomDebouncer 300 ms`、`S-screen1-quiet`) | ブラウザの窓の大きさの変化を 300 ms 静まるまで待ってからエンジンに通知する | (?)。選択(根拠なし)。ゲストの画面の再計算の回数を減らす | 0212 | 窓の引っ張りの間に `WM_DISPLAYCHANGE` が多発する |
| 5-10 | `app/src/input/pointer.ts:80-108`(決定 0260) | ブラウザがフォーカスを勝手に別のボタンへ移したとき、デスクトップにフォーカスを戻す(headless Chrome で約 30 クリックに 1 回、原因は未解明) | (?)。FL の挙動ではなく、ブラウザの焦点。原因が分からないまま症状を隠している点が方針に反する | 0260 | 約 30 クリックに 1 回、キーが落ちる(Escape でメニューが閉じない) |
| 5-11 | `app/src/input/keymap.ts:55-65`、`index.html:24` | Tab・F5・Ctrl+W・Meta キーは エンジンに送らない(Tab は「send Tab to the engine」を入れると送る) | (?)。ホストの統合。FL 側から見ると、既定では Tab が届かない | 0141 | Tab を使う FL の操作が効かない(既定) |
| 5-12 | `app/src/input/tracker.ts:48-52` | 位置が前回と同じ移動は送らない | (c)。実 Windows も同じ位置の `WM_MOUSEMOVE` を作らない、と読む | 0141 | — |

## 7. 表 6: 「契約の不足(b)」の疑いの上位(確認の手順つき)

| 順 | 表の ID | 疑い | 読みの強さ | 確かめる方法(私は実行していない) |
|---|---|---|---|---|
| 1 | 4-2 | フック(`WH_MOUSE` `WH_KEYBOARD`)を呼ばない | 強い(コード上、前提の崩れを確認) | FL が入れるフックの型と手続を G3 の fork プローブで見る(決定 0171 W14 の手順)。呼ぶ実装を入れて差分テスト(Wine 10.0) |
| 2 | 2-3 | `HTTRANSPARENT` で入力を捨てる | 強い(Learn の原文と実装が食い違う) | `WM_NCHITTEST` が `HTTRANSPARENT` を返す窓があるかを窓表のログで探す。下の窓を試す実装を入れる(決定 0136 の再検討条件) |
| 3 | 4-1 | `DefWindowProc(WM_ERASEBKGND)` が背景を塗らない、`hbrBackground` を読まない | 強い(コード上) / 影響は未確認 | `RegisterClass` で `hbrBackground` が NULL でない窓を窓表から数える。Balance の GUI の段階 3 の窓のクラスを見る |
| 4 | 2-10 | 合成器が上の 64 窓で切る | 強い(コード上) / 発生は未確認 | 実走行の最大の最上位窓数を `fle_desk_describe` の行数か窓表で数える(窓の指示待ち) |
| 5 | 2-1 | 最大化窓の再配置が差分 | 中(推測と自称) | 実 Windows か Wine で、最大化窓の画面変更時の矩形を取り、`WM_GETMINMAXINFO` 経由と比べる |
| 6 | 2-7 | タスクバーのボタン 1 個 | 弱い(読み) | 最小化窓を 2 個作って試す |
| — | 3-7 | GDI の画素規則(引き伸ばし・半調子・線) | 弱い(未比較) | Wine 10.0 との画素比較は harness にある(UNOFFICIAL)。実機 Windows は 0 件 |

## 8. 差分案(提出のみ・適用していない)

`diffs/` に 2 本。どちらも `git archive HEAD` の複製で `patch -p1 --dry-run` が **rc 0**(D1 は実適用も別の複製で rc 0。D2 は dry-run だけ)。**どちらもコンパイル・型検査・テストを実行していない**(窓の指示待ち)。

| 差分 | 内容 | 区分 | 変わる挙動 | 検査していない物 |
|---|---|---|---|---|
| `diffs/d1-dead-remaximize-env.diff`(47 行、sha256 先頭 `00fc23c43bde4aba`) | 死んだ環境変数 `FLE_SCREEN_REMAXIMIZE: '1'` をページから渡さない(`app/src/main.ts:343`)。コメント 3 箇所(`main.ts:322`、`ui/screen-fit.ts:4`、`fle_win.c:6748`)の「opt-in」を事実に合わせる | (a)(死んだ設定。決定 0243 [commit 6eb1737d] が読む側を消した。`grep REMAXIMIZE` の出現は `main.ts`・`screen-fit.ts`・`fle_win.c`・docs だけで、ランタイムの `getenv` は 0 件) | 0(ランタイムは読まない) | 型検査(`tsc`)。manifest の env の内容を主張するテストは `app/test` に無い(`grep` 0 件) |
| `diffs/d2-desk-describe-lazy.diff`(sha256 先頭 `9996c00d7aa4aaaf`) | `fle_desk.c`: `dwin` から `cls[48]` `title[80]` を外して `hwnd` を持たせ、`names_of` を `build_desc` の中(= `fle_desk_describe` が呼ばれたとき)に移す。compose と scene から `build_desc()` を除く | (d) を製品の経路から切り離す(点検表の「診断→製品の経路から切り離す」) | 描画は変えない(読み)。`fle_desk_describe` は呼ばれた時点で窓の表を引くので、compose 後に壊された窓の行は題名が空になる | **C のコンパイル**、`harness/alpha1/glass_seq.c` `app/test/e2e/desk-synth/desk_synth.c` `app/test/e2e/*.mjs`(`describe` を読むテスト)、コスト(`names_of` + `snprintf` の µs。測っていない) |

適用するなら PM の窓で: D1 は型検査だけ、D2 は clang の build + `describe` を読むテスト + 前後の Worker CPU(ci1 の純粋 6 組)で、効果が見えなければ見送り。

除去の実験として**差分を出していない**物(壊れる物が出たら (b) の不足として API を拡充する、の方針で、窓の指示待ちの実験):
- 5-1 ページ側の移動の合流(2-2 と二重): ランタイムの `S-mousemove-coalesce` だけにして、`maxQueue` を超えない取り込みの速さが出るか(G3 の移動洪水、`rejected` の数を見る)。
- 5-2 起動前の入力の破棄: 最初のアイドル前の入力をリングに残し、エンジンの最初の取り込みで受ける。
- 5-3 `S-clk-maxlag` / `maxlead`: 切る仕組みを外し、`lagCuts` が出る状況(P 経路の `lagCuts 3.8`)で時計の遅れがどうなるか。
- 5-6 リフレッシュ率の 120 Hz 制限: 決定 0268 の表の条件(present 経路が 120 に届くまで)を満たしたら外す。

## 9. 読んだ範囲と限界

読んだ(全文または該当の範囲):
- ページ: `engine/idle-driver.ts`(全)、`engine/worker.ts`(全)、`engine/desk.ts`(全)、`engine/desk-gl.ts`(前 60 行と grep)、`engine/desk-sleep.ts` `os-state.ts` `taskbar-report.ts` `report-throttle.ts` `appview.ts`(全)、`engine/hz.ts`(前 40 行)、`input/{events,pointer,tracker,keymap,ring}.ts`(全)、`ui/{screen-fit,taskbar,desktop,cursor,panels}.ts`(全)、`main.ts`(grep と 320-352 行)。
- ランタイム: S 印の全箇所の文脈(各 200 字。`census-S-markers.tsv`)、`fle_win.c` の窓管理・フック・`WM_ERASEBKGND`・`BeginPaint`・`fle_win_input_*` の該当、`fle_input.c` 668-760 行と 509・828-1007 行の該当、`fle_desk.c` 28-125・300-345・405-480・484-720 行、`fle_screen.h`(全)、`fle_thread.c` 810-860 行、`fle_appfile.h`(前 40 行)。
- 決定・表: 0245、0262(冒頭)、0222(冒頭)、0143/0154(テストの記述のみ)、`docs/investigations/screen1/tables/s-choices.tsv` の 2 行、`docs/investigations/g3in/logs/S-table.md` の 1 行。

読んでいない(「問題なし」とは言わない):
- `fle_win.c` 7,000 行余のうち、上に挙げた以外の大部分。`fle_gdi.c` の画素規則の実装本体、`fle_raster.c`。`fle_filedlg.c`(32 箇所)、`fle_vfs.c`、`fle_locale.c`、`fle_seh.c`、`fle_sweep1_kbd.c`、`fle_wall_*.c`、`fle_ole.c`、`fle_clip.c` ほか。S 印の約 300 件は文脈を一覧にしただけで、一件ずつの区分は付けていない。
- `fle_x87.c` / `fle_mxcsr*.h` / `fle_sse*.h`: 命令の意味の近道。`fle_mxcsr_flags.h` は「RC=0 のとき float/double の高速経路、外れたら整数の経路」で、方針の点検表 (B)(厳密な前条件 + 元の経路へ戻る)の形。決定 0143(55 種 × 10^8 = 5.5×10^9 回、不一致 0、変異 36 件中 33 検出、3 件は等価と読む)と 0154(9.18×10^9 件、不一致 0、変異 39 件中 38 検出)にテストの記述がある。私はテストを再実行していない。`fle_x87.c` は 80 ビットの自前実装(f64 近似ではない)と読んだ(`from_f64` は f64 の入力を f80 に変換する関数で、演算を f64 で行っていない: 1461 行のうち 300-390 行を見ただけ。**未確認**)。
- lift 側(`crates/fe-lift`、`fe-wasmgen` の生成 C): 範囲外。
- `app/src` の `perf/*`、`ui/latency.ts`、`audio/*`、`engine/{fruity,hostfiles,filedlg,client}.ts`、`ui/filedlg.ts`、`files/*`、`plugins/*`、`bundle/*`: 計測・入出力・ファイル・音で、窓や画素を決める所ではないと読んだが、全文は読んでいない。
- `app/worker/*`(Cloudflare Worker)と `app/shared/*`: サーバー側で、FL の挙動に触れない(見出しのみ)。
- 入力の S 印の表(`g3in/logs/S-table.md`、27 行)の再検討の条件のうち、13 行は「実機測定」(実 Windows での測定)になっている。この環境に実 Windows は無い(Wine 10.0 は UNOFFICIAL)ので、これらの (b) の選択は検証されないまま残る。
- 付録の S 印の「再検討の条件」が満たされたかの照合(`docs/investigations/*/tables/s-choices.tsv` と `docs/decisions` の S 印の表): 4-2 と 2-3 の 2 件だけ確認した。他は未照合。この照合を続ければ、他にも「前提が崩れた S 印」が見つかる可能性が高い。

## 10. 自己申告(逸脱・失敗)

1. `bash` という名前のツールは無いのに、今回のセッションで 9 回目の呼び出しをした(`echo test`。無害、`shell` として実行された)。以降は `shell` を直接呼ぶ。
2. 点検の途中で、PM から「Chrome の順番」の info が来た(Filter の V3 → ci1 の Chrome の Export WAV → opt1 の Wine → ci1 の E14 e2e)。私は Chrome を一度も使っていない。
3. 行番号は `grep -n` の出力から転記した。ページの `*.ts` の一部(`hz.ts` `pointer.ts` `keymap.ts` `tracker.ts` の範囲)は読んだ出力から数えた概数で、±数行ずれる可能性がある。表の「除くと何が壊れる」は読みの予想で、実行していない。
4. D2 は C の差分で、**コンパイルしていない**。`patch --dry-run` が通ることだけを確認した。
5. 表中の「決定文書から転記した値」(0245 の 8.87 → 0.016 ms/フレーム、973 → 585 ms/s、0143/0154 のテスト件数)は再測定していない。5-3 の `lagCuts 18.2 → 3.8` は私の E14 の純粋 6 組の値(`0277/e14/RESULTS-E14-pure.md`)。
6. 4-4(メニュー)と 4-6(`VirtualProtect`)は、FL が実際にその経路を通るかを確認していない。4-1 の Balance GUI との関係は読みだけの仮説。
7. 区分 (a)/(b)/(c)/(d)/(?) は私の判断。(a) と書いた行も、除去の実験をしていないので「余計」と断定していない。

## 付録

| ファイル | 内容 |
|---|---|
| `census-S-markers.tsv` | S 印の全 528 箇所(file、line、marker、area、文脈 200 字)。python による自動生成。区分は付けていない |
| `census-PROVISIONAL.tsv` | `PROVISIONAL` を含む行の数(ファイル別、計 363 行・114 ファイル) |
| `diffs/d1-dead-remaximize-env.diff` | 死んだ環境変数の除去 + コメントの訂正 |
| `diffs/d2-desk-describe-lazy.diff` | 診断の文字列を compose から切り離す |

S 印の領域別の件数(箇所/種類): 窓表とメッセージ 114/95、GDI 76/51、入力 29/25、画面 3/3、合成 1/1、音(範囲外)45/42、その他(ファイルダイアログ・仮想ボリューム・ロケール・例外・壁ほか)260/186。件数の多いファイル: `fle_win.c` 92、`fle_gdi.c` 59、`fle_dsound.c` 45、`fle_filedlg.c` 32、`fle_input.c` 29、`fle_locale.c` 25、`fle_layered.c` 14、`fle_sweep1_kbd.c` 13、`fle_seh.c` 13、`fle_vfs.c` 12、`fle_gdc.c` 12。
PROVISIONAL の行数が多いファイル: `fle_win.c` 53、`fle_gdi.c` 29、`fle_win.h` 20、`fle_locale.c` 18、`fle_vfs.c` 18、`fle_thread.c` 11、`fle_vfs.h` 9、`fle_gdi.h` 7、`fle_ole.c` 7。


## PM の決定と取り込み(2026-11-07 07:36、docs のみ。製品の差分なし)

取り込み: ci1 の 0322(本文)と表・差分案(`docs/investigations/ci1/0322/`)、0321 の再現シナリオと道具(`docs/investigations/ci1/0321/`)。**実エンジンの実行は 0(読みだけ)、依頼者の機械は 0 件**。差分案 2 本は適用していない。

**PM の決定**: (1) FL のクラス名・窓の種類・題名・位置による分岐は製品の経路に 0 件(ci1 の grep)。依頼者の問い『全体の仕組みを直す方針は徹底されているか』への根拠にする。読んだ範囲はランタイム 57k 行のうち数千行で、読んでいない所の『問題なし』は言っていない。(2) D1(死んだ `FLE_SCREEN_REMAXIMIZE` の除去、挙動 0)は採る。D2(診断の文字列を compose から切り離す)も方向として採る(診断は製品の描画経路に置かない。条件: clang の型検査、describe を読むテスト、Worker の CPU の前後)。どちらも runtime の C とページに触れるので、push は後の G9 の push に載せる(全段の冷間 build)。入力の合流 2 段などの除去の実験は窓で行う候補として記録(今は実験しない)。(3) 契約の不足(b)の 6 件は ci1 が差分を作る(0323。①`DefWindowProc(WM_ERASEBKGND)` の背景消去 ②`HTTRANSPARENT` ③`WH_MOUSE`/`WH_KEYBOARD`。④⑤⑥ は後)。①は Balance の GUI が『枠だけ・client が透ける』ことと関係する疑い(読みだけの仮説)。

