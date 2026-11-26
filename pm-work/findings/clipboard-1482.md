# #1482 クリップボード: Clipboard API だけで行い、ブラウザのメニューを出さず、権限は読み込み時に求める

実エンジン: この runner で `npm run build --prefix app`・`cargo build --release --locked --manifest-path lift/Cargo.toml`・`lift/target/release/fe-build engine`・`fe-build bundle` を 1 回ずつ実行(build は約 38 分)。`tools/boot-check/boot-bundle.mjs` の `state` engine-idle・`standIn` false・`error` null・`elapsedMs` 9181(12,500 以下)。`tools/smoke/welcome-flow.mjs` の流れ A〜D の 4 行、いずれも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true、B と D は `welcomeClosed` true。`npm test --prefix app`: Test Files 75 passed | 1 skipped、Tests 1463 passed | 10 skipped。`node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random` が終了コード 0(`rows 4, accepted true`)。`bash guard/scan.sh`: `scan: 0 finding(s) in 0 file(s).`

runner の値(この機械とは別枠。nproc 4。build 直後の loadavg 2.85 1.70 1.87。E2E の fps は 42〜57)。

## 1. 右クリック(E2E `context-menu-none`。行の順は 4k-dpr1・4k-dpr2・4k-random 1.43・4k-random 1.01)

ブラウザのメニューは headless の Chromium では描かれないため、出た項目は見えない。代わりに、ページ側の計測(document の capture で `contextmenu` を数え、`defaultPrevented` を読む)で、ブラウザが送った `contextmenu` が抑止されたかを見る。

| 領域 | 種類 | 押下がページに届いた数(4 行) | contextmenu の数(4 行) | 抑止された数 | 編集可能な対象 |
|---|---|---|---|---|---|
| before-idle(エンジンの入力は止まっている) | 実 | 20・20・20・20 | 20・20・20・20 | 全て | 0 |
| canvas | 実 | 19・15・19・20 | 同じ | 全て | 0 |
| desktop | 実 | 19・19・19・19 | 同じ | 全て | 0 |
| toolbar(#1481 で隠れている) | 合成 | 0 | 20・20・20・20 | 全て | 0 |
| sidebar(aside。隠れている) | 合成 | 0 | 20・20・20・20 | 全て | 0 |

- 実の領域で押下の一部(20 のうち 1〜5)がページに届かなかった。原因は未確認。届いた押下は全て抑止された。
- 対象は全て `div#desktop`・`div#desktop-box`・`div#toolbar`・`aside`(編集不可)。focus は `div#desktop` か `body`。
- iframe の中(Browser の面)は、この E2E の領域に無いため試していない。抑止は iframe の window にも付けた(`ui/webview2-surface.ts`)が、動作は未確認。

## 2. 「貼り付け」だけのメニューの出現条件

未確認。headless では、ブラウザのメニューの項目は確かめられない。試した条件: 上の 5 領域(canvas・#desktop・ツールバー・サイドバー・エンジンの入力が止まっている時)。編集可能な対象は、どの領域にも無かった(表の最後の列)。ページの DOM に textarea・contenteditable・input の生成は無い(grep)。`#send-tab` はチェックボックス、`<input type=file>` は隠れた #filedlg の中。

直す前のコードの読みでは、`contextmenu` の抑止は、入力が有効な時(`opts.enabled()`)の #desktop だけだった。だから、入力が止まっている時、隠れた領域、ページの他の要素では、ブラウザのメニューが出ていた可能性がある(推定。直した後は全領域で抑止されることを確かめた)。「貼り付け」だけの項目は、編集可能な場所での、ブラウザの既定のメニューの一つ(推定。確かめていない)。

## 3. 直した箇所

- `app/src/input/context-menu.ts`(新): `contextmenu` を、document の capture で `preventDefault()`(`app/src/main.ts`)。Browser の面の iframe は、そのウィンドウにも同じ抑止を付ける(`app/src/ui/webview2-surface.ts` の `install`。iframe のイベントは親に届かないため)。
- `app/src/input/pointer.ts`: 有効な時だけ抑止していた `onContext` を外した。
- `app/src/engine/clipboard.ts`(書き直し): `paste` イベントの経路・`pending`・`navigator.clipboard.read()`(書式の読み)・`#clipboard-status`(通知)・`toast` を無くした。読みは `readText`、書きは `writeText` だけ。右クリックの問い合わせは読まない(#1406 の決定を保つ)。権限は、読み込み時に `permissions.query` で読み、`prompt` か `granted` なら `readText` を 1 回呼ぶ。拒否された時は、最初の押下・キーで同じ要求を 1 回だけやり直す。`granted` の時は、フォーカス時にも `readText` で既知の文字を更新する。拒否の理由は `state.refusal` に残すだけ(表示しない)。
- `app/src/input/keymap.ts`: Ctrl+V の特別扱いを外した。Ctrl+V は他のキーと同じく、転送し、ブラウザの既定の動作を止める(paste イベントは起きない)。
- `app/src/style.css`: `#clipboard-status` の規則を外した(要素はもう作られない)。
- テスト: `app/test/unit/clipboard.test.ts`(書き直し)、`context-menu.test.ts`(新)、`pointer-dom.test.ts`・`keymap.test.ts`(期待を新しい動作に合わせた)。
- E2E: `tools/e2e/run.mjs` に `beforeLoad` の口を足した(読み込みの前に、権限と計測を置くため)。シナリオ `context-menu-none`・`clipboard-load-permission`・`clipboard-no-toast`(新)を `smoke/index.mjs` に並べた(`cursorSize` の後ろ)。`knob-right-click-permissions` の印 `#1406` を外した。`tools/e2e/README.md` を更新した。

paste 経路の確認(`grep` の結果): `addEventListener('paste')`・`onpaste` は `app/src/` に無い。`'paste'` の残りは `app/src/files/explorer.ts` の、ファイル一覧の「Paste」(ファイルの貼り付け。クリップボードの API とは無関係)の 3 行。`execCommand`・`clipboard-status`・`navigator.clipboard.read(` は `app/src/` に無い。`app/index.html` の `role="status"` は、ページ自身の常設の表示(`#status`・`#audio-status`・`#taskbar`)で、通知ではない。

## 4. 権限の要求のタイミング(E2E の実測)

- 読み込み時(権限 prompt。`Browser.setPermission` で設定。読み込みの前に組んだ wrapper で数えた): `readText` が 1 回(操作の前)。headless には許可の画面が無く、ブラウザは `NotAllowedError: Read permission denied.` で拒否した。権限は prompt のまま。
- 最初の押下(`clipboard-load-permission` の左クリック): 同じ要求がもう一度行われ、`readText` が 2 回目(これも拒否。prompt のまま)。
- 権限 denied(`clipboard-no-toast`): `readText` は 0 回。通知は 0。
- 右クリック(ノブ。prompt・denied・granted の 3 状態。`knob-right-click-permissions`): `readText`・`read` とも 0。権限の変化 0。メニューは開いた。

## 5. 未確認・既知

- 「貼り付け」だけのメニューの出現条件は、確かめていない(2 節)。直す前の基準の測定は、していない。
- `clipboard-no-toast` の Ctrl+V・Ctrl+C では、FL の `GetClipboardData`・`SetClipboardData` は呼ばれなかった(reads 0・writes 0)。拒否された読みを FL の中で通したことにはならない。拒否された読みが無通知で空を返すことは、単体テスト(`state.refusal` だけ)で確かめた。
- `clipboard.ts` の `refresh()` は、テストからしか呼ばれない(製品の経路には無い)。残すか消すかは監査の判断。
- `tools/clipboard/measure.mjs` は旧版(`#clipboard-status`・paste のリスナー)を測る道具。この変更で測る対象が無くなった。受け入れには無い。残すか消すかは監査の判断。
- 実の領域で押下の一部がページに届かない原因は未確認(1 節)。
- `app/` に触れる push は、main の `deploy.yml` を動かす(AGENTS.md の決まり)。
