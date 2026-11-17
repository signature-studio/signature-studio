# ci1 → PM 決定 0260 報告1（暫定）: File メニュー ×100 のまれな失敗 — 入力時刻の付け方 (a) と記録器の状況

**先頭の事実**: 実エンジンで観測した結果はまだ 0 件（この報告の時点）。下の表 (a) は `lift/runtime/fle_input.c`・`fle_sys.c`・`fle_win.c`・`app/src/engine/idle-driver.ts`（HEAD 62ca650b）を読んだ結果で、実行して確かめた物ではない。代役・再生・推測の成果物は無い。記録器入りの別ビルド（c9）をリンク中で、その観測は次の報告に入れる。

## (a) 入力の時刻がどこで付くか（コードの読み）

| 段 | 場所 | 何が起きるか | 値 |
|---|---|---|---|
| 1. ページ → エンジン | `idle-driver.ts` `submit`/`apply` | ゲストが idle のときは 1 イベントごとに `apply([ev])`。idle でないときは `queue` に積み、次の `onIdle` で `apply(queue.splice(0))`。`apply` は先頭で `moveClock()` を **1 回** だけ呼び、続けて並んでいる全イベントを注入する | 同じバッチの全イベントは同じ仮想時刻 |
| 2. 仮想時計の動かし方 | `idle-driver.ts` `target`/`moveClock` | 壁時計に歩調を合わせる（`anchor + (壁−anchor.wall)×rate`）。ただし idle の `next`（次の期限）を越えない、遅れは最大 1000 ms（`lagCuts`）、進みは最大 100 ms で切る | 入力時の仮想時刻 ≦ 壁時計に対応する値 |
| 3. 注入 | `fle_input_host_mouse` / `fle_input_host_key` | `e->ticks = clk()`（= `fle_sys_clock_ticks()`、100 ns 単位）を **注入の瞬間** に記録。ボタンは注入の瞬間に `key_async`（GetAsyncKeyState）を更新。`mk`（wParam の MK_ フラグ）も注入の瞬間の値 | 注入時 |
| 4. 取り出し（ドレイン） | `fle_input_synth`（GetMessage/PeekMessage から呼ばれる） | キューの先頭から 1 件ずつ `translate_one` → `out_push`。`m->time = e->ticks / 10000`（ms）。**取り出し時の時計は使わない**（MOUSEHOVER だけは取り出し時の `now`） | 注入時の ms |
| 5. 変換中の同期送信 | `translate_mouse`（BDOWN） | `WM_PARENTNOTIFY` → `WM_MOUSEACTIVATE`（アクティブ窓でないとき）→ 必要なら `WM_ACTIVATE`(WA_CLICKACTIVE) → `WM_SETCURSOR` をゲストへ **同期送信**。DOWN の `out_push` は **その後** | ここでゲストが GetMessage/PeekMessage を回すと `fle_input_synth` が再入する |
| 6. ダブルクリック | `translate_mouse` | クラスに CS_DBLCLKS (0x8)・同じボタン・同じ窓・±2 px・**仮想時刻の差 ≦ 500 ms** で `WM_LBUTTONDOWN` が `WM_LBUTTONDBLCLK` に変わる | 仮想時刻 |
| 7. 状態表 | `key_async` / `key_sync` | `GetAsyncKeyState` = `key_async`（注入時に更新）、`GetKeyState`/`GetKeyboardState` = `key_sync`（**変換時**に更新） | 2 つの表が別の時点を指す |
| 8. ゲストが読む時刻 | `fill_msg`（fle_win.c）/ `GetMessageTime` | `MSG.time` = `m->time`。`GetMessageTime` = そのスレッドが最後に取り出したメッセージの `time` | 注入時の ms |
| 9. 他の時計 | `fle_sys.c` | `GetTickCount` / `timeGetTime` / `QueryPerformanceCounter` は同じ仮想時計（`fle_clock_now`）。読むたびに 10 tick (1 µs) 進む。`Sleep` は時計を進める | 取り出し時の時計。`MSG.time` と同じ時計だが読む時点が違う |

Learn の契約との差の読み: GetMessageTime（"the time the message was created (placed in the queue)"）に対し、注入時刻を使うのは契約どおり。GetAsyncKeyState は "at the time of the call" の実機状態で、注入済みのイベントのうち **ゲストがまだ取り出していない UP も既に反映される**（実機でもハードウェア状態は先に進むので契約と矛盾はしない）。

## 読んで見つけた疑わしい点（実エンジンでは未検証。記録器で判定する）

1. **同じバッチの DOWN と UP**: 負荷が高く Worker が忙しいと DOWN と UP が同じ `apply` に入り、`ticks` が同じ、`key_async` は取り出し時点で既に「離した」。→ 記録: 各注入の `ticks`、`evq` の個数、取り出しの順。
2. **`fle_input_synth` の再入による順序の逆転**: 手順 5 の同期送信中のゲストのハンドラがメッセージを回すと、再入した `fle_input_synth` が後ろにある UP を先に `out_push` し、`outq = [UP, DOWN]` の順になる（外側の DOWN は同期送信の後で `out_push`）。`key_sync` も UP の後に DOWN が書かれ「押下のまま」で残る。UP が既にキューに居るのは負荷が高いときだけなので、「約 1%・負荷 14〜30 のみ・負荷 10 以下で無し」と整合する。→ 記録: `NESTED-synth`、`tr-begin/tr-end` の入れ子の深さ、`push`/`get` の順。
3. **取り出し時の標的**: `mouse_target` が取り出し時点でキャプチャ窓／無効窓へ向ける・捨てる（DOWN が別窓へ行く・消える）。→ 記録: `target`（窓・cap・active）、`target-disabled`、`mouseactivate` の戻り値、`DOWN-EATEN`。
4. **ダブルクリック化**: 仮想時刻が壁時計より遅れる（手順 2 の切り）とき、実間隔が 500 ms を超えるクリックでも仮想の差が 500 ms 以内になり得る。→ 記録: `push msg=0203`。

## 記録器（(b)）
- 製品のリンクには入れない。c9（c8 = 4f054ecd の `git archive` の複製）の `fle_input.c` / `fle_win.c` の **先頭に `#define FLE_TEST_RECORDER 1`** を足した複製だけで有効。定義が無い製品ビルドでは `REC` は空（`((void)0)`）。PM の木は触っていない。
- 記録する物（`write(2)` で標準エラー → ページのエンジンログ、行頭 `REC`）: `inj`（注入: ボタンの ms/ticks・`key_async`/`key_sync`・キュー長）、`tr-begin`/`tr-end`（変換の入れ子の深さ）、`target`、`push`（`MSG.time`・`key_*`）、`get`（ゲストが取り出した順序・`MSG.time`・時計・`key_*`）、`ks`（`GetKeyState`/`GetAsyncKeyState(VK_LBUTTON)` の値の変化）、`NESTED-synth`、`mouseactivate`、`DOWN-EATEN`、`no-target-drop`、`OUTQ-FULL`。
- 差分は `ci1-recorder-NOT-FOR-PRODUCT.diff`（次の報告に付ける。製品の木に当てない）。
- テストの道具: `menuCycles` に `hold`（ボタンを押している時間）を足し、各周期の `REC` 行を `report.json` に保存（`tools5/sdrive-a1.mjs`）。周期ごとの要約は `rec-analyze.py`（成功周期と失敗周期の並び）。

## 進捗と次
- c9 のビルド: 記録器入りの 2 オブジェクト（`fle_input.o`/`fle_win.o`）を再コンパイル → リンク完了 22:32（wasm sha256 `410db7f1692b…`、製品 4f054ecd の `4ca5ba4eeca5…` とは別。差はオブジェクト 2 つだけ）。これから配信して周期テストに入る。期限 22:30 を過ぎての送信（上のリンクに時間がかかった）。
- 次: (c) hold 0 / 50 / 300 ms を各 10 回（1 回 = 新しいセッション 60 周期）、負荷は他の作業者の負荷のまま（各回の負荷を記録）。(d) 失敗周期の `REC` 行で、FL の TPopupMenu の問題か、ランタイムの時刻・順序の問題かを判定。ランタイムの逸脱が出れば `ci1-input.diff`（HEAD 基準・dry-run・フラグ無し・クラス名／アドレス分岐無し）。

## 自己申告の逸脱
- 1 回目のリンク（22:10〜22:17）は、既存の出力（`flengine6.opt.wasm` ほか）があり「skipped」になって、記録器の入らない製品と同一の wasm（sha `4ca5ba4e…`）を作っただけだった。sha を見て気づき、出力を消して再リンク（847 s）。約 15 分の空費。
- 背景のシェルに `( … ) &` の形（末尾 `&`）でコピーを起動した（禁止の形。コピーは `cp -a` で副作用は c9 の複製のみ、実害無し。以後 `exec` の背景実行のみ）。
- 前の区間で、失敗する連鎖編集でセットテストが空回りした／依存する 2 つの呼び出しを並列に出した（報告6に記載予定）。
