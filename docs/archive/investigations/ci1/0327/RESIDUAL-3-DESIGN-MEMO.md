# ③の残り: 副作用の発生点を『変換』から『取り出しの確定』へ(設計メモ。ci1、仮の番号 0327)

## 0. 実際に実行した物(先頭)

| 実行した | 内容 |
|---|---|
| ランタイムの読み | `lift/out-ci1/w0323/new5/lift/runtime/fle_input.c`(= HEAD + 0323 + 0323b + 0325 のワークツリー)の `translate_mouse`・`fle_input_synth`・`out_push`・`fle_input_defproc`。決定 0136、0323 の REPORT §4・§6 |
| Wine 10.0 の原本の読み | `dlls/win32u/message.c` の `process_mouse_message`(2443〜2655 行)と `process_hardware_message` を curl で取得して読んだ(UNOFFICIAL。作業用の複製は `lift/out-ci1/kit/out/wine-src/`) |

**実エンジンの実行 0。Chrome・Wine・clang・emcc・cargo も 0。コードの変更 0**(メモだけ)。この文書の『Wine の順序』は原本を読んだ事実、『FL への影響』は読みまたは未確認。

## 1. 現在の動き(0323 の後)

| 段 | 動き | 場所 |
|---|---|---|
| 1 | `GetMessage`/`PeekMessage` が投函キューに何も無いとき `fle_input_synth` を呼ぶ(ここまでは Wine と同じ形。決定 0136 の『所有スレッドが取り出すときに変換』) | `fle_input_synth` |
| 2 | 先頭の生の事象を**取り出して捨てる**(`ev_head++`)。`translate_one` → `translate_mouse` で: ヒットテスト(`WM_NCHITTEST`、`HTTRANSPARENT` の探索)、**`WM_SETCURSOR`**(move・button down)、ボタンを押したとき**`WM_PARENTNOTIFY`**・**`WM_MOUSEACTIVATE`**(→ 活性化・食う)、ダブルクリックの状態更新、`WM_MOUSELEAVE` の生成を**その場で**行い、結果のメッセージを `outq` へ積む | `translate_mouse`、`out_push` |
| 3 | `outq` から範囲・窓の絞り込みに合う 1 通を選ぶ。0323 ③のフック(`input_hook_swallows`)はここ(選んだ直後)で `WH_MOUSE`/`WH_KEYBOARD` を呼ぶ。非 0 なら捨てる | `fle_input_synth` の後半 |

## 2. Wine 10.0 の順序(`process_mouse_message`。読み、UNOFFICIAL)

| 順 | Wine の動き | 条件 |
|---|---|---|
| a | `window_from_point`(`WM_NCHITTEST`。`HTTRANSPARENT` を下の窓へ)。**peek のたびに再実行**(事象はサーバのキューに残る) | 常に |
| b | メッセージ(NC への変換後)が範囲 `first..last` の外なら `return FALSE`(**副作用なし**、事象は残る) | 常に |
| c | ダブルクリックの変換。**状態 `clk_msg` の更新は `remove` のときだけ** | ボタン押下 |
| d | `WH_MOUSE`(`remove` なら HC_ACTION、でなければ HC_NOREMOVE)。非 0 なら事象を受理(捨てる)し `HCBT_CLICKSKIPPED` を呼び `return FALSE`(**`WM_SETCURSOR` は出ない**) | 常に |
| e | `hittest` が HTERROR/HTNOWHERE なら `WM_SETCURSOR` を送り、受理して `return FALSE` | 常に |
| f | `remove` のとき受理。**`!remove` またはキャプチャ中ならここで `return TRUE`**(`WM_PARENTNOTIFY`・`WM_MOUSEACTIVATE`・`WM_SETCURSOR` は出ない) | |
| g | ボタン押下: `WM_PARENTNOTIFY` → `WM_MOUSEACTIVATE`(窓が active でなく最上位が `WS_CHILD` でないとき)→ 応答で活性化・食う | `remove` かつ非キャプチャ |
| h | **`WM_SETCURSOR` を送る(move だけでなく button down・button up でも。lParam の下位は元のマウスメッセージ)** | `remove` かつ非キャプチャ |

## 3. 差(FL に見える形)

| # | 差 | ランタイム | Wine | FL への影響 |
|---|---|---|---|---|
| D1 | `WM_SETCURSOR`/`WM_MOUSEACTIVATE`/`WM_PARENTNOTIFY` と `WH_MOUSE` の順 | 副作用が先、フックが後 | フックが先 | フックが 1 を返したメッセージで副作用だけ出る。**FL のフックが 1 を返すかは未確認**(返さないなら見えない) |
| D2 | 絞り込み(範囲・窓)に合わない事象の副作用 | 変換で出てしまう(絞り込みの前) | 出ない(b で戻る) | `PeekMessage(…, WM_PAINT..)` のように入力を除く絞り込みの呼び出しでも、`qs` に入力のビットがあれば事象が**変換され `WM_SETCURSOR` などが先に出る**疑い(`fle_input_synth` は絞り込みを見る前に `translate_one` へ進む)。**読み。FL がそういう呼び出しをするかは未確認** |
| D3 | `PM_NOREMOVE` の peek | 変換した結果を `outq` に残す。副作用は 1 回 | 副作用なし(ヒットテストだけ毎回)。取り出すときに 1 回 | ダブルクリックの状態 `last_click_*` は peek でも更新(Wine は `remove` のみ) |
| D4 | button up の `WM_SETCURSOR` | 出さない(決定 0136) | 出す(h) | クラスカーソルの再評価の機会が 1 回少ない。O18(カーソル)に関係しうる(ボタンを離した後の絵) |
| D5 | キャプチャ中 | `if (!cap)` で SETCURSOR 等を省く | 省く(f) | 差なし(確認済みの一致) |

## 4. 設計案(実装は PM の許可後)

**M1(最小): 副作用を `outq` の確定へ動かす。**
1. `translate_mouse` は上の a(ヒットテスト)と、メッセージ・`ht` の決定(b の変換)、c のダブルクリックの『判定』だけにし、`outq` の要素に `pend { ht, orig_msg, btn, ex, ey, win, effects_done }` を並べて持たせる(`fle_tmsg` は変えず、`outq` と同じ添字の並行配列)。
2. `fle_input_synth` の選択(範囲・窓の絞り込みを通った後)で: フック(0323 ③の `input_hook_swallows` を d の位置へ)→ HTERROR/HTNOWHERE なら SETCURSOR して捨てる(e)→ `remove` のときだけ g・h を実行(f)。`PM_NOREMOVE` の要素は `effects_done = 0` のまま残り、次の取り出しで 1 回だけ出す。
3. ダブルクリックの状態更新・`set_key(key_sync, …)` は `remove` の確定へ移す。
4. button up にも h を出す(D4。決定 0136 の『出さない』の見直し)。
5. `WM_MOUSELEAVE`(`cursor_win` の管理)と hover のタイマーは、いまの『変換の時点』のまま(Wine でも leave はサーバ側の移動で生成)。

**M2(忠実): 事象を取り出さずに残して、毎回 `process_mouse_message` 相当を実行する。** `evq` の先頭を peek し、a→h をその場で実行、受理したときだけ `ev_head++`。D3 を完全に Wine と同じにするが、`outq` の分離(mouse 以外の key・wheel・leave は今の形)と `S-mousemove-coalesce`・`ev_thread` の扱いの再設計が要り、変更が大きい。

**推奨: M1**(Wine の順序との差 D1〜D4 を消す。変更は `fle_input.c` 中心で約 150 行の見込み〔読み〕)。M2 は D3 の『毎回ヒットテスト』が FL に効くと分かったときだけ。

## 5. テスト(受け入れ)

- `lift/harness/c0323/`(シナリオ H: H1〜H6)の Wine との差 13 行が 0 になる(Wine 10.0 の出力が oracle。`WM_SETCURSOR` の位置・フックが 1 を返したときの `WM_SETCURSOR` なし・peek の副作用なし)。
- 追加するシナリオ: (i) 範囲の外の `PeekMessage` で副作用が出ない(D2)、(ii) `PM_NOREMOVE` を 2 回呼んでから `PM_REMOVE` で `WM_SETCURSOR` が 1 回だけ(D3)、(iii) button up の `WM_SETCURSOR`(D4)、(iv) ダブルクリックの状態が peek で変わらない。
- 既存: `win_upd`・`win_seq`・`win_act`・`contract_0323` の差が増えない、決定 0136 の変異検査 26 件(`wall_fr14.rs` 24 件)の期待値のうち『SETCURSOR を button up に出さない』『変換で出す順』に依存する物は更新が要る(**未確認**: どの件か)。

## 6. 危険と未確認

| 項目 | 内容 |
|---|---|
| 決定 0136 に触れる | `S-setcursor-result`・`S-httransparent`・`S-capture-client`・`S-disabled-drops`(0323b)・`S-mousemove-coalesce`・`S-no-target` と同じ関数内で、順序の変更が波及する。E9 の『入力 ring をスケジューラの点で読む』(0280)とは無関係(取り出しの前) |
| FL の挙動 | FL のフック手続きの初回の実行、`WM_SETCURSOR` の遅れによる O18 の絵の遅れ(小さい見込み。読み)、button up の `WM_SETCURSOR` を受けて FL が行う処理(未確認) |
| 根拠の格 | 取り出し点でのフックは Learn(MouseProc)にあるが、副作用との順序は Learn に無く Wine の読みだけ(UNOFFICIAL)。Windows 実機との照合は 0 |
| 優先度 | 0323 の cand1 の実エンジン評価(FL のフック手続きが何を返すか、`WM_SETCURSOR` の呼ばれ方)の後に決めるのがよい。フックが常に 0 を返し絞り込みの呼び出しが無ければ、D1・D2 は FL には見えない(読み) |
