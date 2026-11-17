# 決定 0323(ci1): 契約の不足(b)の 3 件 — ① 背景の消去、② HTTRANSPARENT、③ WH_MOUSE / WH_KEYBOARD

## 0. 実際に実行した物(先頭)

| 実行した | 回数・内容 |
|---|---|
| clang -O0(nice 19、1 回に 1 つ)で ランタイム(`lift/runtime/fle_*.c` の 31 ファイル)とテストハーネスを native ビルド | HEAD 3969ad31 の木と 0323 の木の 2 つ。`-Wall -Wextra -fsyntax-only` の警告は fle_win.c fle_input.c fle_class.c fle_gdc.c で両方 0 件 |
| native のテストの実行 | `contract_0323`(シナリオ E P T H N)を HEAD / 0323 で、`win_upd` `win_seq` `win_act`(api2 の既存)を HEAD / 0323 で |
| Wine 10.0(mingw でビルド、headless、`DISPLAY` なし、`out-ci1/w0323/work-wine/wineprefix`、wine の同時起動 0、MemAvailable 16〜28 GB) | `contract_0323` を 6 回(初回 18 s〔wineboot〕、以降 約 2 s)。opt1 の Wine とは重ねていない(`ps` で wine 0 を確認してから起動) |
| `curl` / webfetch で Wine 10.0 の原本と Learn 3 ページを取得 | 差分の根拠(`learn/learn-extracts.md`) |
| `patch -p1 --dry-run` と実適用(別の複製) | `git archive HEAD`(3969ad31 と ed125d09)で rc 0。適用後の木 = ワークツリー |

**実行していない物**: 実エンジン(FLEngine と同梱 DLL の lift 済み C、G3 / N1 / wasm)は 0 回。Chrome・Firefox は 0 回。emcc・wasm-opt・cargo・vite build は 0 回。ランタイムの他の単体テスト(`lift/crates` の cargo テスト、`harness/` の他のテスト)は 0 回。つまり **差分が実エンジンの絵や挙動をどう変えるかは未測定**(下の §7)。依頼者の機械の値は 0 件。

成果物: `lift/out-git/ci1-results/0323/` — `REPORT-0323.md`(これ)、`diffs/0323-contract-fixes.diff`(1852 行、sha256 先頭 `9679079df88bad5d`、適用していない)、`learn/learn-extracts.md`、`logs/`(Wine と native の出力、下の表)。ワークツリーは `lift/out-ci1/w0323/`(終了時に消す)。

## 1. 結果の要約

| 項目 | 直した仕組み(全体) | テスト | HEAD(3969ad31) と Wine の差 | 0323 と Wine の差 |
|---|---|---|---|---|
| ① 背景の消去 | 窓の「背景の消去の手順」全体: `DefWindowProc` の `WM_ERASEBKGND` / `WM_PAINT`、`BeginPaint` と `RedrawWindow(RDW_ERASENOW)` が送る `WM_ERASEBKGND` の DC(更新領域でクリップ)、`fErase` と「消去が残る」状態 | E(メモリ DC、4 クラス)と P(窓の DC、10 場面) | E 20 行、P 16 行 | **E 0 行、P 0 行(Wine と同一)** |
| ② HTTRANSPARENT | マウス入力の対象窓を決める手順(`WM_NCHITTEST` を同じスレッドの下の窓へ順に送る) | T1〜T3 | T 51 行(T4 T5 を含む) | 子窓の場面(T2 T3)は配送の列が Wine と同じ。差は T1(Learn と Wine が違う)と Wine 側の余分な行 |
| ③ フック | 入力メッセージの取り出し点(`GetMessage` / `PeekMessage` が返す直前)での `WH_MOUSE` / `WH_KEYBOARD` のフック連鎖の呼び出し | H1〜H6 | H 31 行(フックの行が 0 件) | H 13 行(フックの行は Wine と同じ内容。差は `WM_SETCURSOR` の位置など、下の §6) |
| 既存テストの後退 | — | `win_upd` `win_act`(HEAD と 0323 の native 出力を比べた) | — | **0 行の差**。`win_seq` は `WM_ERASEBKGND` が 2 行増える(Wine にもある行) |

Wine との差の行数は `diff` の `<` `>` の行の合計: 全体で HEAD 130 行 → 0323 48 行(`logs/summary.txt`)。

## 2. ① `DefWindowProc(WM_ERASEBKGND)` が背景ブラシで消去する契約

**Learn の契約**(`learn/learn-extracts.md`): DefWindowProc は WNDCLASS の hbrBackground で背景を消す。hbrBackground が NULL なら消さない。消したら非 0、消さなければ 0 を返す。0 なら窓は「消去が残る」(PAINTSTRUCT の fErase が TRUE)。hbrBackground はブラシのハンドルか「システム色 + 1」。BeginPaint は DC のクリップを更新領域にする。

**HEAD の実装との差**(読みと実行の両方で確認):
1. `fle_win.c:1486` の DefWindowProc は、何も描かずに常に 1 を返す(決定 0123 の選択、出典なし)。クラス表(`fle_class.c`)は WNDCLASSW の 72 バイトを保存するが、`hbrBackground`(オフセット 48)を消去に使う所は 0 件。
2. BeginPaint は `WM_ERASEBKGND` を送った**後**で DC のクリップを更新領域にする(`fle_win.c:5380`)。消去を受けるゲストの DC には更新領域のクリップが無く、ゲストが「クライアント全体」を塗ると有効な画素を上書きする(テスト P8〜P10: `RectVisible` で更新領域の外が見える。Wine は見えない)。
3. 既定の `WM_PAINT` は更新領域を検証するだけで、`WM_ERASEBKGND` を送らない(Wine は BeginPaint / EndPaint 相当で送る。P1 P2)。
4. 消去を 0 で答えた窓の「消去が残る」状態が無い(NULL ブラシのクラスでも fErase は常に 0。P4 P7)。

**差分**(`diffs/0323-contract-fixes.diff`、ランタイムの行数は fle_win.c +181 −51 など):
- `fle_class.c/h`: `fle_class_hbrbackground_of(atom, &hbr)`(`fle_class_style_of` と同じ形)。
- `fle_gdc.c/h`: `fle_gdc_is_window_dc(dc)`(窓の DC かの判定。`GetClipBox` は窓の DC を扱わず止まるため)。
- `fle_win.c`: `def_erase_background`(ブラシなし → 0。窓の DC → クライアント矩形、メモリ DC → `GetClipBox`、`FillRect` でブラシ〔ハンドルかシステム色 + 1〕を塗り 1 を返す。CS_PARENTDC もクライアント矩形)、`erase_now_one`(窓の DC を取り、更新領域でクリップして `WM_ERASEBKGND` を送り、解放。0 を返された窓は `upd_delayed` で「消去が残る」)、BeginPaint はクリップの設定を `WM_ERASEBKGND` の前に移し fErase に `upd_delayed` を含める、既定の WM_PAINT は消去を送る、`paint_now_tree` は `erase_now_one` を使う。
- DC のクリップは「窓の DC は更新領域でクリップされている」前提で、塗る矩形はクライアント全体(クリップが画素を絞る)。窓の DC で `GetClipBox` を呼ばないのはそのため。

**テスト**: シナリオ E(メモリ DC に `WM_ERASEBKGND` を送り、ブラシあり〔COLORREF 0x563412〕、ブラシなし、COLOR_WINDOW + 1、CS_PARENTDC、各々クリップあり / なしの画素と戻り値)は Wine と**同一**(HEAD は 20 行違う: 塗らない、戻り値が常に 1)。シナリオ P(窓の DC: 既定の WM_PAINT、自前の BeginPaint、bErase FALSE、RDW_ERASENOW の後の BeginPaint、更新領域のクリップの探り)も Wine と**同一**(HEAD は 16 行違う)。シナリオ N(ランタイムだけ。窓の面の画素を読む): 更新矩形(10,10,50,40)を消去すると、内側の画素がクラスのブラシの色になり(N1〜N3、N6 は白)、外側の画素は元の 0x404040 のまま。ブラシ無しのクラス(N4)と bErase FALSE(N5)は何も塗らない。HEAD は N1〜N6 のどれも塗らない。

**(b) の契約として直した仕組み**: 窓の背景の消去の手順全体(DefWindowProc の 2 つのメッセージ、BeginPaint と RedrawWindow が送る `WM_ERASEBKGND` の DC のクリップ、fErase)。FL の窓の挙動には触れていない(クラス名・窓の種類・位置による分岐は 0)。

**壊れる物の仮説**(実エンジンでは未確認):
- 効く: クラスに背景ブラシを持つ窓(VCL 以外、プラグイン本体の窓など)の消去されなかった背景が、ブラシの色になる。PM の観察「Balance の GUI が枠だけ・client が透ける」「Settings の描画の崩れ」の原因になりうる(**確認していない**。どのクラスが hbrBackground を持つかを、実エンジンの `RegisterClassW` の呼び出しで数える必要がある)。
- 変わる: NULL ブラシのクラスで `fErase` が TRUE になる。`fErase` を見て塗る窓は塗り始める。既定の `WM_PAINT` を通る窓にゲストの `WM_ERASEBKGND` が増える。消去のクリップが狭くなり、クライアント全体を塗っていた窓の「有効な部分の上書き」がなくなる(その上書きに頼る描画があれば崩れる)。
- 止まりうる: クラスのブラシが `FillRect` の扱えない種類(模様ブラシ)なら名前つきの stop。

**残る差**: WM_NCPAINT を既定の WM_PAINT が送らない(Wine は送る。win_seq の差)。WM_ICONERASEBKGND(0x27)は触っていない。`DCX_USESTYLE`(兄弟・子のクリップ)は GetDC 側の既存の規則(S-style-clip)のまま。

## 3. ② `HTTRANSPARENT` を「同じスレッドの下の窓へ送る」契約どおりに

**Learn の契約**: `docs/investigations/alpha6/tables/learn/wm-nchittest.txt:58-59`。「HTTRANSPARENT: 同じスレッドで別の窓に覆われている窓。メッセージは、HTTRANSPARENT 以外を返す窓が見つかるまで、同じスレッドの下の窓へ送られる」。

**HEAD の実装との差**: `fle_input.c:509`(`S-httransparent`、決定 0136)は、`WM_NCHITTEST` が `HTTRANSPARENT` を返すと入力イベントを**捨てる**。再検討の条件は「下の窓を試す実装を入れるとき」(`g3in/logs/S-table.md`)。

**差分**: `fle_win.c` に `fle_win_input_hit_list(x, y, out, max)`(点を含む可視窓の一覧。順序は Wine 10.0 の `get_window_children_from_point` + `all_windows_from_point`〔UNOFFICIAL〕: 上の兄弟から、各窓について**先にその子孫、次にその窓**)。`fle_input.c` に `hit_walk`: 最初の窓に `WM_NCHITTEST` を送り、`HTTRANSPARENT` でなければ従来どおり(一覧を作らない)。`HTTRANSPARENT` なら一覧の次の窓へ(別スレッドの窓には聞かず `HTCLIENT`、無効な窓は従来どおり捨てる〔S-disabled-drops〕、全部が `HTTRANSPARENT` なら窓なし)。キャプチャ中は従来どおり結果を使わず `HTCLIENT`(`WM_NCHITTEST` は送る)。

**テスト**(シナリオ T): T2(透明な子 → 親が受ける)、T3(透明な子の上に不透明な子 → 上の子が受ける。透明な子だけの場所 → 親が受ける)は、`WM_NCHITTEST` の連鎖と `WM_MOUSEMOVE` / `WM_LBUTTONDOWN` / `WM_LBUTTONUP` の配送が Wine と同じ列(絞り込み比較。HEAD は全部落ちる)。T1(同じスレッドの最上位窓 2 つ、上が透明): **0323 は下の窓 A に配送する(Learn どおり)が、Wine 10.0 は配送しない**(Wine の `window_from_point` はサーバーが選んだ最上位窓の下しか見ない)。Learn が明示しているので Learn に従った。

**(b) の契約として直した仕組み**: マウス入力の対象窓を決める手順(ヒットテストの連鎖)。

**壊れる物の仮説**: FL が `HTTRANSPARENT` を返す窓を持つか**未確認**。持つなら、今まで落ちていたクリックが下の窓に届く(正しい方向)。最上位窓の透明(T1)は FL が使うなら挙動が Wine と違う(Learn に従う)。呼び出しは 1 イベントあたり、最初の窓が `HTTRANSPARENT` でない限り従来と同じ(一覧を作らない)。

## 4. ③ `WH_MOUSE` / `WH_KEYBOARD` を入力の経路で呼ぶ契約

**契約**(`learn/learn-extracts.md`、MouseProc / KeyboardProc): `GetMessage` / `PeekMessage` が、処理すべきマウス / キーのメッセージを返す直前にフック手続きを呼ぶ。nCode は取り出すとき HC_ACTION(0)、`PM_NOREMOVE` の peek では HC_NOREMOVE(3)。wParam はメッセージ(マウス)/ 仮想キー(キー)、lParam は MOUSEHOOKSTRUCT へのポインタ / キーの lParam。非 0 を返すとメッセージはウィンドウ手続きへ渡されない。連鎖は新しい順で、`CallNextHookEx` で次へ。

**HEAD の実装との差**: `fle_win.c:2327`(`S-hook-no-events`、決定 0126)はフックを記録して**呼ばない**。理由は「この環境にマウス / キーのメッセージは存在しない」で、決定 0136 の入力モデルで崩れた。`WH_CALLWNDPROC` だけは決定 0171 で呼ばれる。`fle_input.c` に `hook` の語は 0 件だった。FL は起動時に 2 個入れる(決定 0194 の読み)。

**差分**: `fle_win.c` に `fle_win_input_call_hooks(id, thread, nCode, wParam, lParam, blob, len, &result)`(スレッドの最新のフックを呼ぶ。`blob` を guest のスタックにコピーして lParam にする。`CallNextHookEx` は既存の連鎖をそのまま使う)。`fle_input.c` の `fle_input_synth` で、メッセージを返す直前(取り出すときはキューから外した**後**)に `input_hook_swallows`: マウスのメッセージ(0x200..0x20E と非クライアント 0xA0..0xAD)なら MOUSEHOOKSTRUCTEX(pt = 画面座標、hwnd、wHitTestCode = クライアントは HTCLIENT・非クライアントは wParam、dwExtraInfo = 0、mouseData = wParam)、キーのメッセージ(`WM_KEYDOWN` `WM_KEYUP` `WM_SYSKEYDOWN` `WM_SYSKEYUP`)なら vk と lParam。非 0 ならメッセージを捨てる(peek でも外す)。フック手続きの中での再入は呼ばない(ガード)。

**テスト**(シナリオ H): H1(マウスのフックが全部通す: `WM_MOUSEMOVE` `WM_LBUTTONDOWN` `WM_LBUTTONUP` で nCode 0、pt・hwnd・ht)、H2(キーのフック: `WM_KEYDOWN` `WM_KEYUP` の vk と lParam〔0x1e0001 / 0xc01e0001〕)、H3(マウスのフックが `WM_LBUTTONDOWN` に 1 を返す → W は受けない)、H4(キーのフックが VK_A に 1 を返す → W は受けない)、H5(`PM_NOREMOVE` で HC_NOREMOVE〔3〕、`PM_REMOVE` で HC_ACTION〔0〕)、H6(フックを外す → フックの行なし)。**フックの行の内容は Wine と同じ**(HEAD は 0 件)。差は §6。

**(b) の契約として直した仕組み**: 入力メッセージの取り出し点でのフック連鎖の呼び出し(`WH_CALLWNDPROC` の連鎖と同じ表を使う)。

**壊れる物の仮説**: **FL のフック手続きが lift 済みコードとして初めて実行される**。未実装の API を呼べば名前つきの stop になりうる(実エンジンで要確認)。FL(VCL)のフックが何をするかは未確認(ヒントの消去、モーダル状態の検出などは私の推測で未確認)。非 0 を返して入力を捨てるフックがあれば入力が落ちる(契約どおり)。

## 5. 副産物(PM の指示の範囲外。見つけただけで、直していない)

| ID | 内容 | 根拠 | 状態 |
|---|---|---|---|
| ④ | `S-disabled-drops`(決定 0136): 無効な窓の上のマウス入力を捨てる。Wine 10.0 は、**無効な子窓を飛ばして親が受ける**、**無効な最上位窓には `HTERROR` で `WM_SETCURSOR` を送る** | テスト T4 T5(Wine と runtime が違う)、`server/window.c:891-905`、`window.c:2572-2620`。S 印の表の実例(`TWPControl` の中の `TUnlockBuyModule` の Buy ボタンの押下が捨てられる)は、Wine の規則なら親(ウィザード)が受ける | 差分なし。窓の指示で作る |
| ⑤ | Wine 10.0 の `GetPixel` はクリップの外でも画素を返す(Learn は CLR_INVALID)。ランタイムは Learn どおり | シナリオ E の初版で観測 | テスト側で回避(クリップを外してから読む)。Wine 側の逸脱 |
| ⑥ | `GetClipBox` は窓の DC を扱わず stop する(`fle_gdi.c`、「画面の DC」と同じ扱い) | 実装の読み | ① は窓の DC を `fle_gdc_is_window_dc` で分けて避けた。窓の DC の `GetClipBox` は未実装のまま(FL のゲストが呼べば stop) |

## 6. 残る差(Wine との 48 行の内訳)

- T1: 上述(Wine 10.0 は最上位窓の兄弟へ進まない。Learn は進む)。
- T4 T5: ④。
- Wine がクリック後に足す末尾の `WM_NCHITTEST`(ヒットテストの更新)と、ボタンを上げるときの `WM_SETCURSOR`(ランタイムは送らない。決定 0136)。ともに ①〜③ の前から在る差。
- H: ランタイムは入力イベントを**変換する時点**で `WM_SETCURSOR` / `WM_MOUSEACTIVATE` / `WM_PARENTNOTIFY` を送るため、フックの呼び出しより前に出る。Wine は取り出すときのフックの後に送る。フックが 1 を返したメッセージについて、ランタイムは `WM_SETCURSOR` などが既に出ている(Wine は出ない)。`PM_NOREMOVE` の peek で Wine はその時点でヒットテストを送る。**直すには副作用の発生を「取り出し時」へ動かす必要があり、決定 0136 の設計に触れる**ので、今回はしていない。

## 7. 検証していない物(限界)

1. **実エンジン(G3 / N1 / wasm)での絵・挙動・stop**: 0 回。①②③ のどれも、FL の窓の見た目が変わるか、新しい stop が出るかは未測定。窓での確認の候補: (a) N1 の G3 で起動〜Settings〜Balance を走らせ、`RegisterClassW` の `hbrBackground` が NULL でないクラスの数を数え、前後の画の差を見る、(b) フックの手続きが初めて実行されたとき止まらないか。
2. wasm ビルド(emcc)での型・リンク: 0 回。`fle_win.c` ほかは弱い参照(`fle_gdc_is_window_dc` `fle_imp_FillRect` `fle_imp_GetClipBox`)で、GDI を link しないテストの link は壊れない(ブラシ付きの消去に当たったときだけ名前つきの stop)。
3. 他の単体テスト(`lift/crates` の cargo テスト、`harness/` の他の native テスト): 0 回。api2 の `win_upd` `win_act` の native 出力が HEAD と 0323 で同一であることだけ確認した。
4. 性能: `fle_input_synth` にフックの判定(`hook_count == 0` なら即 return)を足した。マウスの 1 イベントあたりの `hit_walk` は最初の窓が `HTTRANSPARENT` でない限り従来と同じ。コストは未測定。
5. Wine は Windows ではない(UNOFFICIAL)。Learn が黙る所だけ Wine を基準にした(`def_erase_background` の CS_PARENTDC、窓の一覧の順序、MOUSEHOOKSTRUCTEX の mouseData、HC_NOREMOVE の扱い)。

## 8. alpha1 との重なり

fle_win.c の hunk(元の行番号)を alpha1 に info で知らせた(115-121、401-407、1325-1334、1481-1500、2324-2332、2432-2438、2504-2550、2634-2641、2731-2737、5317-5324、5351-5357、5370-5390、6456-6565、6597-6633)。ScrollWindowEx の差分との重なりの返事は未着(返信不要と書いた)。重なれば私が載せ替える。

## 9. 自己申告

1. テストの GDI 部分で `CreateBitmap(…, 32 bits)` を使い、ランタイムが止まった(単色マスクだけ答える)。`CreateCompatibleBitmap(GetDC(NULL))` に直した。
2. Wine 側の `GetPixel` の逸脱(⑤)を読み違えて、初版の E の比較が合わなかった。クリップを外してから読むようにした。
3. 差分の `fle_win.c` は 7 本の hunk が既存の関数の途中(DefWindowProc、BeginPaint)を触る。意味を変える所は `decision 0323` の注釈を付けたが、元の注釈の書き直しも含む。
4. 本文の /tmp への書き込みが quota で空になった事故(PM に報告済み)の後、本文は `out/` に書いている。`bash` という名前のツールの呼び出しは 9 回で、今回の作業では増えていない。
5. ワークツリー `lift/out-ci1/w0323/`(Wine の prefix 1.2 GB を含む)は、成果物の確定後に消す(`lift/out-ci1` は終了時に 500 MB 以下にする約束)。
6. D2(0322 の診断の切り離し)は今回の差分に含めていない。

## 10. 再現

```
git archive HEAD lift/runtime lift/harness/api2 | tar -x -C T    # T = ワークツリー
(cd T && patch -p1 < 0323-contract-fixes.diff)
bash T/lift/harness/c0323/build.sh both T W            # native(clang -O0)+ Wine 10.0(mingw)、W/wine-vs-native.diff
bash T/lift/harness/c0323/build.sh native T W N        # 窓の面の画素(ランタイムだけ)
bash T/lift/harness/c0323/regress.sh T W               # win_upd / win_seq / win_act の native 出力
```


## PM の決定と取り込み(2026-11-07 08:10、docs のみ。製品の差分なし)

取り込み: ci1 の 0323(契約の不足 ①背景の消去 ②HTTRANSPARENT ③WH_MOUSE/WH_KEYBOARD のフック)、差分(`docs/investigations/ci1/0323/diffs/`)、Learn の引用と Wine 10.0 の出力(`evidence/`)。**PM の検証**: `0323-contract-fixes.diff`(1852 行)を HEAD の git archive に実適用して rc 0・FAILED 0・offset 0。alpha1 の 3 本・mt3 の 0313f2・ci1 の d1・d2 と合わせた 7 本を順に当てても FAILED 0・.rej 0。変更した runtime は `emcc -fsyntax-only -Wall -Wextra` で新しい警告 0。Wine 10.0 との差は 130 行から 48 行(ci1 の報告)。実エンジンでの効果は未測定(cand1 の bundle ができてから)。区分 (b)、全体の仕組みはそれぞれ『窓の背景の消去の手順全体』『マウス入力の対象窓を決める手順』『入力の取り出し点でのフック連鎖』。依頼者の機械は 0 件。


### PM の追記(2026-11-07 08:49、ci1 の 0327・0328 と 0326 §9 の取り込み。docs のみ)

- **0327(③の残りの設計メモ)**: `docs/investigations/ci1/0327/RESIDUAL-3-DESIGN-MEMO.md`。Wine 10.0 の `process_mouse_message` の順序(ヒットテスト → 範囲の絞り込み → ダブルクリック → WH_MOUSE → HTERROR → remove のみ受理 → PARENTNOTIFY/MOUSEACTIVATE → WM_SETCURSOR)とランタイムの差 D1〜D5。変換(`translate_mouse`)は既に取り出し(`fle_input_synth`)の中なので決定 0136 の方針は合っており、直す対象は『変換の中の副作用の位置』。案 M1(副作用を `outq` の確定へ移す、約 150 行の見込み)を推奨、M2 は D3 が FL に効くと分かったときだけ。**未決**: 優先度は低〜中。cand1 の実評価(FL のフックが 1 を返すか、絞り込みの呼びがあるか)の後に決める。実装は cand1 の実評価の後(未決)。Wine の `message.c` は読みの出典(UNOFFICIAL の取得物)で、ここには置かない。
- **0328(cand1 の評価と D2 の手順)**: `docs/investigations/ci1/0328/CAND1-EVAL-AND-D2-PROCEDURES.md`。評価は 9 段(E1 起動、E2 Filter の読み込み、E3 0323① で Balance の client が描かれるか、E4 フック、E5 HTTRANSPARENT、E6 0319 の 3 本、E7 D1/D2、E8 Export〔原 FLP + 変種〕、E9 参考の first idle)。**cand1 には 0325(カーソル)と 0323b が入っていない**ので、cand1 ではカーソルの変化は評価できない(RT-A には入る)。`RegisterClassW` の `hbrBackground` を数える診断は cand1 に無い(観察だけの診断を足す次の build が要る。gpu1 の V3 の結果で E3 の見通しがついてから要否を決める)。D2 の確認は『D2 を載せた木を APP_REPO_ROOT にした e2e の compositor-synth 群』で足りるとする案(S2)を採る。
- **0326 §9(S3a の保留)**: 掃引は保留。RT-A の bundle(09:40 ごろ)で `FLE_NUM_CPU` × `FLE_WORKERS` の軸で行う。確かめた事実: `fle_imp_GetSystemInfo` は `dwNumberOfProcessors = 1`・`dwActiveProcessorMask = 1`(`fle_sys.c` の注釈『run が単一の guest thread だから』)、`GetLogicalProcessorInformation` も 1 個。runtime に `FLE_NUM_CPU` の語は無い(= 0319 の alpha1 の読みと一致)。
- 注記: ci1 の e2e(`run-e14-e2e.sh preview dev worker`)は `others()` の検査を、nice < 10 の重い job と `chrome-count.tsv` 末尾の『他の自動 Chrome』の数だけに直した版で動いている。e2e は非計時の機能の合否なので load 条件を ≤ 12 に緩めた(結果に『load 緩和』と書く)。
