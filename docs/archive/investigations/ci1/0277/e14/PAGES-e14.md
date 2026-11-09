# E14 の B 側 page 2 種の準備（ci1、00:56 machine date、ブラウザ 0 本、nice 19、vite build と curl だけ）

**事実（先頭）**: 実エンジンは動かしていない（ブラウザなし）。測ったのは『page の dist の sha256』と『サーバーが同じ bytes を返す』ことだけ。計時なし、依頼者の機械は 0 件。

| | 内容 | page（kit 内） | サーバー | 元の木 |
|---|---|---|---|---|
| A（prev） | main d0447902（E13 の本番の page） | `page-d0447902/app/dist` | 5053（起動済み） | `git archive d0447902 app` と byte 一致（diff 0 件） |
| B1 | P 単独（穴の修正入り、V4 なし） | `page-e14-P/app/dist` | **5061** | `lift/out-pmchk-e14c/tree/app`（`vite.kit.config.ts` の追加以外 byte 一致） |
| B2 | P + V4 | `page-e14-PV4/app/dist` | **5062** | `lift/out-pmchk-e14d/tree/app`（同上） |

bundle は 3 つとも E13 の本番 `fl-engine-ci-0363b050`（同じ `assets-fl-engine-ci-0363b050`）。`/assets/manifest.json` は 3 ポートとも 200・3,098 B・sha256 `57817472d5a0ea2d…` で同一。起動: `exec ./serve.sh 5061 fl-engine-ci-0363b050 e14-P`、`exec ./serve.sh 5062 fl-engine-ci-0363b050 e14-PV4`（perf の beacon は `out/perf-5061.jsonl`・`perf-5062.jsonl`）。

## page の dist の sha256（index.html と main の chunk を先頭に。『served-same』= そのファイルを該当ポートから curl で取って sha256 が一致）
| page | file | bytes | sha256 |
|---|---|---|---|
| A d0447902 | `index.html` | 3,736 | `a9a58c26693e5dfcd9ecc1655964b92671895625717898db9cfe4259104b14c8` |
| A | `static/index-KJzvmXMs.js`（main chunk） | 219,052 | `c1c34a022c3550a3f120396d31ba8a114602e89697cc904fa355015c0f3e0950` |
| A | `static/worker-BpuyBw_Z.js` | 44,811 | `65622b0c4b66c84bee6f2645550337b4595277bf7790c2b4d33ecee5aa6bbd76` |
| B1 e14-P | `index.html` | 3,736 | `bfb0fcd09953c991a37f9decb3ddeb3c687a40e88a579ca338ac96ea35526813` |
| B1 | `static/index-D_GaZxVn.js`（main chunk） | 219,706 | `eb32f992fca9fc00d9b63b7faf9dddb78a2573dfe6848f93a4208f93b0b58753` |
| B1 | `static/worker-QQJ6NzDL.js` | 47,841 | `c19672a042aa44d662599635d87522fb111f125c23bdd44edfa6648cb5b62399` |
| B1 | `static/desk-worker-CEZo7ZuQ.js`（新: present Worker） | 12,662 | `5e50ce8d94e28a9c506abb7ea813fa4a82abd42d48506cf65ddbb4d145a0846b` |
| B2 e14-PV4 | `index.html` | 3,736 | `fed05db353005e0f89853cbca5593bb50181fec716cce43343b772f3a498b6b9` |
| B2 | `static/index-CaYMPBtk.js`（main chunk） | 219,706 | `389b7b7df548d069927ab4400edc3275080fc2196ce9660378fbca50498cdc24` |
| B2 | `static/worker-8QJ_USPX.js` | 48,124 | `179762396bf28f6e96ad6a3ea7ea3848029967a1436a51a3862eeb1332bc95cb` |
| B2 | `static/desk-worker-CEZo7ZuQ.js` | 12,662 | `5e50ce8d94e28a9c506abb7ea813fa4a82abd42d48506cf65ddbb4d145a0846b`（B1 と同一） |
| 3 page 共通 | `static/fle-out.worklet-CbgfZo7e.js` | 7,234 | `15ddab9adde905618a977fd81b05c38427203bf5653f033c3cc7914a33b7637a` |
| 3 page 共通 | `static/index-bY1HsUKC.css` | 12,822 | `ff678f3853d353cadfa3019f40bc07a5fa6c7848d83a219a2fe2bed6464e6cb9` |

served-same: 3 ポートとも index.html と static の全ファイルが一致（A 5 本、B1 6 本、B2 6 本）。

## 木の差（ソース。`diff -rq`）
- B1・B2 と A（main d0447902 の app）の差（両方同じ 12 件）: `src/engine/{client,desk-gl,protocol,worker}.ts`、`src/main.ts`、`tsconfig.json`、`tsconfig.webworker.json`、`test/unit/worker-diag.test.ts` が違う。新規: `src/engine/desk-link.ts`、`src/engine/desk-worker.ts`、`test/unit/{desk-gl-link,desk-link}.test.ts`。
- B1 と B2 の差: **`src/engine/worker.ts` の 1 ファイルだけ**（V4）。dist では `worker-QQJ6NzDL.js`（47,841 B）と `worker-8QJ_USPX.js`（48,124 B）、main chunk の hash（index.html の参照）が違う。

## 読み取ってほしい点
- 計時（E13 pure → E14 first idle 10 組）は gpu1 の窓の後に PM の info で出す。E14 の first idle は A = 5053（PREV）対 B1 = 5061（NEW）で `idle chrome 10`、B2 = 5062 は別の組（PM の指定による）。
- 本資料はビルドと配信の確認だけで、P の動作（present Worker の経路が実際に画面を出すか）はブラウザで見ていない。

## B3 = P + V7（`page-e14-PV7`。03:38 machine date、nice 19 の vite build 1 回、ブラウザなしで作成）
- 木: `lift/out-pmchk-e14f/tree/app`（PM が検証: tsc 4 構成 rc 0、vitest 1,320 passed / 8 skipped / 0 failed）。bundle は E13 の本番 `fl-engine-ci-0363b050` のまま（5053 と同じ `assets-fl-engine-ci-0363b050`、manifest は 200・3,098 B・sha256 `57817472d5a0ea2d…` で同一）。サーバー: **5063**（`exec ./serve.sh 5063 fl-engine-ci-0363b050 e14-PV7`）。A は 5053（E13、d0447902）のまま。
- 木とページの差: `diff -rq lift/out-pmchk-e14f/tree/app kit/page-e14-PV7/app -x node_modules -x dist` は `vite.kit.config.ts` の追加だけ。**P 単独の `page-e14-P` との差は `src/engine/desk-worker.ts` の 1 ファイルだけ**（`diff` で +4 / −1: 『アニメーション frame の scene 要求を frame の処理の後の task で出す』コメント 1 行と `MessageChannel` 2 行を足し、`L.requestScene()` を `requestAfterFrame.port2.postMessage(0)` に替えた）。
- dist の sha256（served-same = 5063 から curl で取った bytes と一致。6 ファイルすべて一致）:

| file | bytes | sha256 |
|---|---|---|
| `index.html` | 3,736 | `84667441857a1e76ed4950c4d222f4538cfa5e7bbb3771646c91e98115694275` |
| `static/index-V7sD7FGQ.js`（main chunk） | 219,706 | `951188b14a74e5a6e0590af996c04f5400f422c91bd9397bd6b5aeafd556292c` |
| `static/desk-worker-gHF_7GvZ.js`（P の present Worker + V7） | 12,737 | `92ad2a66125e78c03a93116b829d3633506447a0fb035b04b0d4e50675cf86d9`（`page-e14-P` の `desk-worker-CEZo7ZuQ.js` は 12,662 B・`5e50ce8d…`） |
| `static/worker-QQJ6NzDL.js` | 47,841 | `c19672a042aa44d662599635d87522fb111f125c23bdd44edfa6648cb5b62399`（`page-e14-P` と同一） |
| `static/fle-out.worklet-CbgfZo7e.js` | 7,234 | `15ddab9adde905618a977fd81b05c38427203bf5653f033c3cc7914a33b7637a`（全 page 共通） |
| `static/index-bY1HsUKC.css` | 12,822 | `ff678f3853d353cadfa3019f40bc07a5fa6c7848d83a219a2fe2bed6464e6cb9`（全 page 共通） |

### B3 の計時でない確認（03:39〜03:53、Chrome 156、5063、bundle は E13 の本番。実エンジン、この機械、模擬入力。依頼者の機械は 0 件。load はこの間 mt3 の冷間 build が走っていたので 1.4〜8.1 で、参考値）
| 項目 | 結果 |
|---|---|
| 15 シーン（Chrome） | **15/15 ok**（d7 は意図した終了）。first idle の値は load 1.4〜8.1 の中の単独 run で参考（4,025〜5,902 ms） |
| File メニュー ×100（d8） | **100/100** |
| Export（Ctrl+R、参照プロジェクト） | **22,863,828 B・sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` で参照と一致**（319 s、load 5.2 → 3.0） |
| alpha7（主窓の配置と画） | 主窓 `TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080`、見える窓 1。**画は E13（`out-archive-e13/alpha7/`）と 0 画素差**（`main-1920x1080.png`・`scenario-alpha7-look.png` とも） |

置き場所: `0277/e14/RESULTS-PV7-functional.md`、`0277/e14/data-pv7/`。first idle・pure・lat は未実施（mt3 の build が終わった後の PM の info を待つ）。
