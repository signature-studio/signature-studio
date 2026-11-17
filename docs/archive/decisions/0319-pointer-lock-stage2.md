# 0319 Pointer Lock 第 2 段の設計(読みのみ。O20 の Tempo drag の仮想ポインタ)

status: 設計(読みのみ。実機の run は 0 本)。第 1 段の virtual pointer(`0319-o20-virtual-pointer-contextmenu.diff`)が済んだ後の page 側の設計。RT-E への取り込みは、PM の GO をもらってから。

## 0. 何が分かったか

- FL の drag 関数の挙動(gpu1 の静的な読み): ShowCursor(FALSE) でカーソルを隠し、縮めた矩形(32,32,W-32,H-32)と前位置 prev を比べ、prev が矩形の外なら SetPhysicalCursorPos で反対側へ回り込ませる(サムスン式 wrap)。起動直後の tempo は 15 から 522 で、Tempo drag の 522 固定はこの wrap と、runtime が host の実座標で上書きして仮想位置を消していたことの合成で説明できる(構造的)。O20 第 1 段で runtime の仮想位置は直した。**ブラウザの実ポインタが画面の端で止まる限り、回り込みの後も動き続けることができない**。そのため Page 側で Pointer Lock が要る。
- 現在の page の input(読み。`app/src/input/pointer.ts`、`app/src/input/tracker.ts`、`app/src/engine/appview.ts`): 
  - `tracker.pointer({x, y, buttons})` が絶対座標だけを受け、movementX/Y を使わない。`InputTracker` は `lastX/lastY` で連続の絶対座標だけを見ている。
  - `Document.prototype.pointerLockElement` や `exitPointerLock()`、`requestPointerLock()` はどこにも無い。
  - `contextmenu` は `preventDefault` だけ。
  - appview(`fle_appview.c`)は `fle_win_pointer_get(&st.pointer_x, &st.pointer_y)` でゲストのポインタ位置と、`fle_gdi_cursor_current()` で現在の cursor、`fle_input_host_pending()` で待ち数を見せているが、**ShowCursor の display count(`fle_gdi_cursor_display_count()`)は見せていない**。`cursor_idc` 等はある。

## 1. 必要な設計

### 1.1 Pointer Lock の読み(出典)
- Pointer Lock API(MDN): ロック中の MouseEvent の clientX/clientY/screenX/screenY は固定され、movementX/movementY が前イベントからの変化を返す。値に上限が無く、継続して動いても端で止まらない。カーソルは隠れる。ロックは明示的に解除するか、ユーザーの解除アクション(Esc)で終わる。
- 出典: https://developer.mozilla.org/en-US/docs/Web/API/Pointer_Lock_API
- requestPointerLock() はユーザーのジェスチャ(クリック等の engagement gesture)の後に要求できる(同じ出典の Note)。
- MDN に「解除時にカーソルを『ロック要求の位置』へ戻す」という規定は無い(この記事の範囲で見た)。Chrome の実装の挙動は**未確認**として検証項目に回す(§3)。

### 1.2 page 側の変更(設計のみ)
- **runtime は何も変えない**: appview は既にゲストのポインタ位置(仮想)とカーソルの状態を見せている。Pointer Lock の page 側の変更は、show cursor の display count を runtime 側に返すための小さい追加だけにする(下の 1.3)。
- **要求のタイミング**: ゲストが ShowCursor(FALSE) を呼んである間(display count < 0)。この間だけ、desktop element に Pointer Lock を要求する。要求は ES2018 の promise 形(`requestPointerLock({unadjustedMovement: true})`)と単なる呼び出しの両対応。成功したら document.pointerLockElement を見て armed にする。
- **解除のタイミング**: display count が ≥ 0 に戻ったら(FL の end 側で SetPhysicalCursorPos → ShowCursor(TRUE))、document.exitPointerLock() を呼ぶ。実ポインタは画面の端で止まるので、仮想座標の wrap-up を優先する。
- **座標の伝え方**: ロック中は MouseEvent の絶対座標(clientX/screenX)は変わらないので、InputTracker の `tracker.pointer({x, y, buttons})` の x/y を `movementX/movementY` の累積に置き換える。具体的には、ロック中は `previous virtual + movement` で `tracker.pointer({x: virtualX, y: virtualY, buttons})` を送る(仮想位置)。実ポインタを端まで送っても、時間と共に変わらないので、仮想位置は movement の累積で進む。仮想座標は画面範囲に丸める(runtime の `fle_win_host_pointer_set` の clamp と同じ)。
- **ポインタ位置の初期化**: ロック開始時に、仮想位置を「入力が入ってくる前に appview が見せたゲストのポインタ位置」に合わせる(appview の pointerX/pointerY)。これが無いと、ロック後の最初の movement で仮想位置が跳ぶ。
- **失敗時**: requestPointerLock が Esc を押した後にすぐ指に場合は成功しない(出典の Note)。再要求してはいけない(ユーザーのジェスチャが要る)。その場合は第 1 段の仮想ポインタだけに戻る。
- **終了の検証**: exitPointerLock の後、pointerlockchange で armed を外す。Esc で出ても同じ。

### 1.3 runtime 側への小さな追加(設計のみ。PM の未決)
- appview(`fle_appview.c`)が `st.flags` か構造体の新フィールドに `cursor display count`(ShowCursor の現在の count)を出すこと。現行の構造体は `fle_gdi_cursor_display_count()` が未 export のため。page はその count < 0 の間だけ Pointer Lock を要求する。**第 1 段の差分には入れない**(変更が必要)。

### 1.4 検証項目(実機。まだ 0 本)
未検証の項目: (1) Chrome の解除時のカーソル位置。(2) Esc での解除の挙動。(3) SetPhysicalCursorPos の後、カーソルを探し直すかどうか。(4) 長い drag での仮想位置の wrap の見え方。

## 2. 寿命

第 2 段で必要なこと: **page の input の change** が一番である(下の 1.2)。runtime の追加は 1.3 だけでよい。いずれも (a) 余計な処理ではなく (b) 外部 API の契約の実装(表示範囲の契約)である。O18 の上昇下降の cursor の経路は同型なので、この変更が O18 の修正にもなれる(再利用。FL のクラス・窓の分岐なし)。

## 3. 未確認の項目

- Chrome が解除時にカーソルをどこに戻すか(未確認)。
- Esc で解除された時に再要求できるかどうか(出典によると「解除アクションの後は次の engagement gesture の後」なら再要求できる。FL の drag 中は次のジェスチャが無いので、再要求は end の後までできない)。
- 長い drag の後の movement 累積の誤差(ページ側の座標が中心付近で跳ぶ可能性。未検証)。
