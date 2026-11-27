# #1503 ツマミの右クリックで、クリップボードを読まない(読むのは貼り付けの選択の時だけ)

実エンジン: この runner で `lift/target/release/fe-build engine` と `fe-build bundle` を 1 回(04:35〜05:15 UTC 頃)、`tools/boot-check/boot-bundle.mjs` が error null・engine-idle・standIn false・elapsedMs 12307 を返すのを観測した。`tools/smoke/welcome-flow.mjs` の流れ A〜D が 4 行とも判定を通すのを観測した(05:15〜05:17 UTC)。`npm test --prefix app` が 1467 passed・10 skipped。`tools/e2e/run.mjs` が smoke `knob-right-click-permissions` と regression `clipboard-permission-knob` を 2 表示(4k-dpr1・4k-dpr2)で観測し、4 行とも印なし(expectFail null)・ok true・全カウンタ 0(05:17〜05:23 UTC)。`bash guard/scan.sh` は 0 件。さらに、`app/src/engine/clipboard.ts` だけを HEAD の版に戻した比較でも、同じ 2 シナリオを観測した(05:23〜05:28 UTC)。

runner の値(この機械とは別枠。nproc 4。loadavg は起動確認の時 2.58 1.6 1.9)。

## 0. 読みの回数(右クリック。権限ごと。読み込みの後に数えた)

| 版 | シナリオ | 権限 | 読み(read() + readText()) | メニュー | 表示 | 権限の変化 |
| --- | --- | --- | --- | --- | --- |
| 直した後 | smoke `knob-right-click-permissions` | prompt・denied・granted | 0・0・0(DPR 1 と 2) | 3 つとも開いた | 0 | 0 |
| 直した後 | regression `clipboard-permission-knob` | prompt | read() 0・readText() 0(DPR 1 と 2) | 開いた | 0 | 0 |
| 直した後 | regression `clipboard-permission-knob` | denied | read() 0・readText() 0 | 開いた | 0 | 0 |
| 直した後 | regression `clipboard-permission-knob` | granted | read() 0・readText() 0 | 開いた | 0 | 0 |
| HEAD(比較) | smoke・regression(同じ) | 3 つ | 0 | 開いた | 0 | 0 |

- 直す前の版(HEAD)でも、右クリックの間の読みは 0 だった。#1482 で、形式の問い合わせから `navigator.clipboard.read()` を外したため、#1406 の記録(`granted` で `read()` が 1 回)の経路は、この版には無い。
- r06 の印 `#1503` は、この版では読みを示さない(印は外した)。

## 1. 読みの経路(`app/src/engine/clipboard.ts`)

| 経路 | いつ | 右クリックで読むか | 根拠 |
| --- | --- | --- | --- |
| `start()` | 読み込みの時(prompt・granted) | 起きない(計数の前に終わる) | 観測 0(計数は読み込みの後から) |
| `firstAction()` | 最初の押下・キーで、読み込みの要求が拒否されていた時(`askAgain`) | 右の押下で起きうる(直した) | コード上。E2E では読み込み後の左クリック(startDefault)が先に消費するため、発火は観測していない |
| `onFocus()` | 権限が granted で、窓が focus を得た時 | 押下の保持中に起きうる(直した) | 観測 0(E2E の右クリックでは focus が起きていない)。押下の保持中の focus は、単体テストで確かめた |
| `probe()`(形式の問い合わせ) | FL の IsClipboardFormatAvailable など | 読まない(コード上) | 観測 0 |
| `read()` の右クリックの問い合わせ(`isQuestion()`) | 右ボタンの離れから 1.5 秒以内の GetClipboardData | 読まない(コード上) | 観測 0 |
| `read()` の貼り付け | 左の離れの後の GetClipboardData(メニューの「貼り付け」、Ctrl+V) | 読む(readText。`denied` は読まない) | 単体テストで確かめた。E2E では「貼り付け」を選んでいない(未確認) |

## 2. 直した箇所

- `app/src/engine/clipboard.ts`: 右ボタンが押されている間を `rightDown` で覚える。右の押下では、読み込みで拒否された要求を再度行わない(`firstAction`)。右ボタンを押したままの窓の focus では読まない(`onFocus`)。
- `app/test/unit/clipboard.test.ts`: 2 件を足した(右の押下で要求しない。granted で、押下の保持中の focus は読まない)。
- `tools/e2e/scenarios/regression/r06-clipboard-permission-knob.mjs`: 印 `#1503` を外した(`expectFail: null`)。
- `tools/e2e/scenarios/regression/common.mjs`: 衝突を解消した。`OPEN_ISSUES` から `#1503`(この作業で印が無くなる)と `#1505`(main で閉じている)を外した。
- 直していない: 形式の問い合わせ(`probe`・`isQuestion`)、Ctrl+V の経路、Copy(`writeText`。#1389)、`app/src/files/select.ts`、`tools/e2e/judge.jq`、`tools/e2e/run.mjs` の `rowPasses`。

## 3. 判断と未確認

- `granted` の起動の読み(`start()`)と、focus の読み(`onFocus()`、押下の保持の外): 右クリックの間には起きないので、Issue の決まりに従い残した。「貼り付けの操作の時だけ」の規則の文字どおりには当たらない。依頼者の判断に残す。
- denied の貼り付けの表示 1 件(Issue の「やること」2 と「判定の基準」3): 入れていない。台帳 L-079(権限が無い時のトーストを出さない)、#1482(表示を無くした)、smoke `clipboard-no-toast` と input `clip-denied-no-notice`(denied の貼り付けでも表示が出ないことを検査)と矛盾する。依頼者の判断。
- 実機の focus: ウィンドウの活性化は、押下より前に focus を起こしうる。その場合は押下の保持中の判定に掛からない。E2E では再現しない。未確認。
- メニューの「貼り付け」の読み(readText)は、E2E の受け入れでは押していない。単体テストだけで確かめた。未確認。
- Ctrl+V の経路は変えていない(GetClipboardData → readText)。
