# #1389 クリップボード: FL の Copy・Paste を利用者の OS のクリップボードへ通す

実エンジン: この runner で `lift/target/release/fe-build engine` と `fe-build bundle` を作り(2 回。最後の版は 12:06〜12:53 UTC)、`tools/clipboard/measure.mjs` が `{"error":null,"copied":true,"match":true,"pasted":true,"deniedHandled":true,...}` を出すのを観測した。`tools/boot-check/boot-bundle.mjs` は `state` engine-idle・`standIn` false・`error` null、`tools/smoke/welcome-flow.mjs` の流れ A〜D は全て `error` null・engine-idle・`hostErrors` 0・`opened` true を観測した。

runner の値(この機械とは別枠。nproc 4。loadavg は測定の終わりに 2.18 1.49 1.94)。

## 0. 呼び出しの記録(測定を先に行った。`window-api:` の行は `window.__app.windowApiTrace(true)` で取った)

Copy(「Problems loading the project」の窓で 'FLEX (FL)' の行を押し、出た 'Copy' の popup(`TQuickPopupMenuWindow`)を押す):

```
window-api: OpenClipboard hwnd=0x7f990014
window-api: EmptyClipboard
window-api: SetClipboardData fmt=13 handle=0x7ffa50000010 bytes=20 text="FLEX (FL)"
window-api: CloseClipboard
```

FL が使う形式は CF_UNICODETEXT(13)。Copy の文字列は UTF-16 で 9 文字。

Paste(名前の編集窓 `TNameEditForm` で Ctrl+V。窓は、チャンネルを選んで F2 で開く):

```
window-api: IsClipboardFormatAvailable fmt=1 -> 1 (os text 1, open 0)
window-api: OpenClipboard hwnd=0x7f990014
window-api: GetClipboardData fmt=13 -> 0x7ffa50000030 (os text) bytes=26 text="flrvp2qe4a3g"
window-api: CloseClipboard
```

FL は、貼り付けの前に CF_TEXT(1) の有無を見てから、CF_UNICODETEXT(13) を取る。Ctrl+V を机の上(窓の無い所)で押しても、クリップボードの呼び出しは出ない。

## 1. 採った方式と理由

- Copy: ゲストの `SetClipboardData(CF_UNICODETEXT)` の handle(GMEM_MOVEABLE)を読み、文字列を Worker から page へ渡す(`clipboard-write`)。page は `navigator.clipboard.writeText` を呼ぶ。Copy の後、約 1 秒の内に読み戻すと、文字列は一致した(測定の `match`)。
- Paste: `GetClipboardData(13)` と `IsClipboardFormatAvailable(1/7/13)` は、JSPI の非同期の import(`fle_clip_host_read`)で止まる。Worker が page へ `clipboard-read` を送り、page が `navigator.clipboard.readText` の結果を返す(`clipboard-text`)。読みは Paste ごとに 2 回: `IsClipboardFormatAvailable(1)` を、開く前に確かめるとき 1 回(その結果は捨てる)と、開いた中の `GetClipboardData(13)` の 1 回(開いている間は 1 回の読みを保つ)。入力の遅れは、Ctrl+V の入力から page の最初の読みの要求まで 47〜51 ms(`pasteDelayMs`)。2 回目の読みを省けば、その分の遅れは減る(未実施)。
- 権限: `navigator.permissions.query` で `clipboard-read`・`clipboard-write` の状態を見る。`denied` のとき、page は読みも書きもせず、ページの状態行(`#clipboard-status`、`role="status"`)に理由を出す。Clipboard API が無いときも同じ。engine は止めない(読みに答えが無いときは 3 秒で NULL を返す)。
- 形式: CF_TEXT(1)・CF_OEMTEXT(7) は、OS の文字列から 1 文字 1 バイトに変換する(ASCII 以外は '?')。ゲストの CF_UNICODETEXT しか無いときの CF_TEXT も、同じ変換で答える(従来は `fle_stop`)。

## 2. 測定(tools/clipboard/measure.mjs の JSON の行)

```
{"error":null,"copied":true,"match":true,"text":"FLEX (FL)","textLength":9,"pasted":true,"pasteDelayMs":47,"deniedHandled":true,"deniedMessage":"The browser does not allow this page to read the clipboard (clipboard-read: denied). Paste gets no text. Allow it in the browser's site settings.","state":"engine-idle","permissions":{"read":"denied","write":"granted"}, ...}
```

(`copyCalls` と `pasteCalls` は 0 の並び。`pasteDelayMs` は 2 回の測定で 47 と 51。)

## 3. 受け入れの判定の項目(この版)

| 項目 | 結果 | 出力 |
| --- | --- | --- |
| `tools/clipboard/measure.mjs` | 受理 | `error` null・`copied` true・`match` true・`pasted` true・`deniedHandled` true |
| 起動(boot-bundle) | 受理 | `error` null・`state` engine-idle・`standIn` false・`elapsedMs` 12242 |
| 流れ A・B・C・D | 受理 | 全て `error` null・engine-idle・`hostErrors` 0・`opened` true。B・D の `welcomeClosed` true。`elapsedMs` の最大 11195。`preBootBytes` の最大 311114634 |
| `npm test --prefix app` | 受理 | Test Files 70 passed・1 skipped。Tests 1357 passed・10 skipped(clipboard.test.ts の 6 件を含む) |
| `cargo test --test wall_ia1`(native) | 受理 | 19 passed |
| `cargo test --test wall_ia1w`(native) | 受理 | 7 passed |

## 4. 未確認・注意

- 製品のソースは、測定したエンジンの版から `#ifdef`(`__EMSCRIPTEN__`)を外しただけ。外す前と後の `fle_clip.c` のオブジェクトの sha256 は一致し、`lift/out-app/obj/rt/fle_clip.o` とも一致した(その後の再ビルドは行っていない)。
- Paste は、FL の名前の編集窓(F2)だけで確かめた。他の FL の欄での Ctrl+V は未確認。
- 'Copy' の権限の拒否(clipboard-write: denied)の表示は、コードの経路を単体テストで確かめただけで、この測定では確かめていない(測定の拒否は clipboard-read)。
- `EnumClipboardFormats`・`CountClipboardFormats` は OS の文字列を数えない(FL の貼り付けは IsClipboardFormatAvailable と GetClipboardData を使うことを観測した)。
- 利用者の操作の無い IsClipboardFormatAvailable(窓のクリックの probe)は、観測では出なかった。出る場合は、読みの要求が 1 回増える。

## 5. この試行(run の後の再試行): 衝突の解消と、受け入れの連鎖

- 衝突の解消: `app/src/engine/protocol.ts` の衝突は、main の `webview2` の `front`(#1376)と、この Issue の `clipboard-write`・`clipboard-read` を両方残して解いた。印は残していない。origin/main は 415abb71(pm-work の下書きだけ)が先に進んでいたが、`git merge-tree` で衝突が無いことを確かめた。
- `fe-build engine` は通った(runner で 14:09〜14:32 UTC)。
- `fe-build bundle` は、OTT の wasm(`lift/harness/web/mt1/plugin-worker/build.sh ott`)の入力が無いため止まった: `build: stage check failed: the installers do not match ci/artifacts.lock`。インストーラ(Serum 2・OTT 1.37・Vital)は、`plugin-external` のラベルの Issue だけが R2 のキャッシュから `lift/out-app/external` に置く。この Issue には無い。bundle が OTT を作るようになったのは d14ed548(14:00 UTC)で、前回の試行(12:06〜12:53 UTC)より後。
- 補足(受け入れの連鎖の残りを手で行った。bundle の `pack.args` から OTT の lazy file の 1 組(`lift/out-app/plugins/ott/vst-ext.wasm`)だけを外した一時の引数で、`pack-seed`・`pack-assets` を実行した。一時の files は削除した):
  - `tools/clipboard/measure.mjs`: `copied` true・`match` true・`pasted` true・`deniedHandled` true・`error` null・`pasteDelayMs` 50・`textLength` 9。
  - `tools/boot-check/boot-bundle.mjs`: `error` null・`state` engine-idle・`standIn` false・`elapsedMs` 9262。
  - `tools/smoke/welcome-flow.mjs`: 流れ A・B・C・D の全てで `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B・D は `welcomeClosed` true。`elapsedMs` の最大 7673。`preBootBytes` の最大 311116601。
  - `npm test --prefix app`: Test Files 70 passed・1 skipped。Tests 1380 passed・10 skipped。
- 受け入れの判定は、`fe-build bundle` で止まるため、通っていない。次に決めること: この Issue に `plugin-external` を付けるか、bundle の OTT を必須から外すか(後者は deploy に関わるので PM が決める)。 → 解決(§6): main の 4d73666a で、インストーラが無い時は OTT を作らず、--lazy-file を省くようになった。

## 6. この試行(run の後。2026-11-24 16:07〜17:05 UTC)

- 前回の判定の落ちた段(bundle の OTT の入力)は、main の 4d73666a で直っていた。この試行の `fe-build bundle` は、`fe-build: 外部プラグインのインストーラが無いので、省く: lift/out-app/plugins/ott/vst-ext.wasm=plugins/ott.wasm` を出して、通った。製品のソースは、前回の差分から変えていない。
- 計測の道具(`tools/clipboard/measure.mjs`)の直し: (a) `permissions.write` は、拒否の段で `navigator.permissions.query` で問い直す(前は読み込みの時の値の granted のままだった)。(b) `pasteDelayMs` は、Ctrl+V の入力から、読みの答えの最後(GetClipboardData の読みの答え)までに直した(仕様どおり。前は最後の読みの要求だった)。`pasteRequestMs` を足した(Ctrl+V から最初の読みの要求まで。IsClipboardFormatAvailable の読み)。
- 受け入れ(記載どおりのコマンドを、段ごとに実行した):
  - `npm run build --prefix app`: 型検査 4 本と vite の build は通った。
  - `cargo build --release --locked --manifest-path lift/Cargo.toml`: 通った。
  - `fe-build engine`: 終了 0(16:09〜16:39 UTC)。`hybrid: 165624634 bytes, 8 direct modules, 1 separate modules`。
  - `fe-build bundle`: 終了 0(16:39〜16:53 UTC)。
  - `tools/boot-check/boot-bundle.mjs`: `error` null・`state` engine-idle・`standIn` false・`elapsedMs` 11250。
  - `tools/clipboard/measure.mjs`(2 回。下の値): `error` null・`copied` true・`match` true・`pasted` true・`deniedHandled` true。
  - `tools/smoke/welcome-flow.mjs`: 流れ A・B・C・D の全て `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B・D は `welcomeClosed` true。`elapsedMs` 10512・10614・10599・10411(最大 10614)。`preBootBytes` 311116601(全て同じ)。
  - `npm test --prefix app`: Test Files 70 passed・1 skipped(71)。Tests 1380 passed・10 skipped(1390)。
  - `cargo test --locked -p fe-lift --test wall_ia1 --test wall_ia1w`: 19 passed・7 passed。
- 計測の値(runner の値。この機械とは別枠。nproc 4。loadavg(1・5・15 分)は計測の終わりに 2.29 1.46 1.75)。2 回:

  | 回 | pasteRequestMs(Ctrl+V から最初の読みの要求) | pasteDelayMs(Ctrl+V から GetClipboardData の答え) | 読みの往復 |
  | --- | --- | --- | --- |
  | 1 | 54 | 60 | 6 |
  | 2 | 58 | 72 | 14 |

  貼り付けの遅れの大半は、Ctrl+V から読みの要求までの engine 側の処理(約 54〜58 ms)で、読みの往復は 6〜14 ms。読みを 1 回に減らす案は、この版には入れていない(未実施)。
- 権限の拒否の実測: 計測の拒否の段で、`clipboard-read` と `clipboard-write` を `denied` にした。読みは拒否され(`deniedHandled` true)、書きの拒否は、この計測では確かめていない(Copy をしていない。単体テストだけ)。
