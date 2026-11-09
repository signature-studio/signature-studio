# 0344-2: RT-H を入れた E13 / RT-F / RT-H の 3 者 G2 — ci1, 2026-11-08

## 状態: 未実行(RT-H の bundle 待ち)

- PM の指示(06:33): mt3 の RT-H(g9_hook を診断スイッチ ON のときだけ設定する版)が届いたら(PM が知らせる)、E13 / RT-F / RT-H の 3 者で同じ測定。
- 2026-11-08 07:10 時点で、RT-H の bundle の連絡は私には届いていない(mt3 の最後の連絡は RT-G の bundle。RT-H は「PM の指示で作業中」との記載)。
- 用意済み: `tools/scn-lat-ff.json`(0342 と同じ)、`tools/g2-multi.py`(任意の run を並べて、最初の run との差を出す)、`tools/run-rta2.sh lat COND`(COND に rth を足すだけ)。bundle が届き次第、`mk-bundle-assets.sh BUNDLE rth`(診断 OFF の本番形の manifest)→ `serve.sh <port> rth rt1` → 同じ gate で E13 / RT-F / RT-H を 1 組(RT-F の assets は `assets-rtf`、port 5122 が現存)。
- 結果が出るまで、RT-A の対照(REPORT-1)は「フックを持つ実スレッド版に共通」までしか言えない。**フック由来か実スレッド由来かの分離は RT-H の結果待ち**。
