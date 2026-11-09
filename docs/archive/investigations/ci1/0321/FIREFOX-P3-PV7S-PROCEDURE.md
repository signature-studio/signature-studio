# Firefox 158 の p3 対 pv7s ×2 の手順書（未説明の『約 11% 遅い』を同一構成の 2 回で再現するか）

**先頭**: **実エンジンの実行は 0**（ci1、2026-11-07 07:1x machine date。手順書だけ。Firefox・Chrome・Wine・ビルドは起動していない）。実行は PM の合図（gpu1 と Chrome/Firefox を取り合わない窓）で。

## 1 何を確かめるか（事実と未確認）
- **事実（PM の 4e 06:42 の段落〔gpu1〕）**: Firefox 158 + P + V7 は画面・読み込み・30 s 再生・`opErrors` 空・first idle 4.3 s・present 34.7/s だが、perf の窓の仮想 s/実 s が 0.765、30 s 後の再生位置が P 単独（p3）の 26:08 に対し pv7s は 23:16 で**約 11% 遅く**（n = 1、原因未調査）。Chrome の P + V7 は逆に仮想 s/実 s 0.656 → 0.912 と改善した（0277-e14）。
- **未確認**: Firefox での遅さが (a) 再現するか（run の揺れか）、(b) P + V7 に固有か（P 単独との差）、(c) 原因（Firefox の `Atomics.waitAsync` の起床の遅れ、MessageChannel の task の順序、GL〔llvmpipe〕の費用）。本書は (a)(b) だけを決める。(c) は計器つきの別作業。
- 名前の対応は**未確認**: gpu1 の `p3` = P 単独、`pv7s` = P + V7（+ 計器なし）と読む。kit の page で同じ物は `page-e14-P`（port 5061）と `page-e14-PV7`（port 5063）。gpu1 の page と木が同じかは gpu1 に確認する。

## 2 構成（同じ bundle、page だけが違う）
| 腕 | page | port | bundle |
|---|---|---|---|
| prev = P 単独（p3 に相当） | `page-e14-P`（`lift/out-pmchk-e14c/tree/app` の複製） | 5061 | E13 `fl-engine-ci-0363b050` |
| new = P + V7（pv7s に相当） | `page-e14-PV7`（`lift/out-pmchk-e14f/tree/app` の複製） | 5063 | 同上 |
- 2 つの差は `src/engine/desk-worker.ts` の +4/−1 だけ（`PAGES-e14.md`）。`worker-QQJ6NzDL.js` は同一。
- `verify-bundle.sh pure` は奇数組を prev → new、偶数組を new → prev で回すので、**2 組 = P, PV7, PV7, P（ABBA）の 4 run**（各 約 2 分）。『pv7s ×2、p3 ×2』を順序の偏りなしで取れる。

## 3 手順（参考。実行しない）
1. **準備**: `ps` で他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export が 0 本、`free -m | sed -n 2,3p` と `vmstat 1 3 | tail -2`（si/so が 0、MemAvailable ≥ 8 GB）を `out/steps.log` に。Xvfb :77 が動いていること（`ps` で確認。無ければ `exec ./start-xvfb.sh` を背景で）。サーバー 5061 と 5063 が 200 を返すこと（`curl`）。`out/` が前の run と混ざらないよう、前の E の記録は `out-archive-eN` に退避済みであること（今の `out/` は E14 の物なので、新しい run の前に `mv out out-archive-e14; mkdir out` と chrome-count の観測の再起動）。
2. **環境**: `export NEW=fl-engine-ci-0363b050+PV7 PREV=fl-engine-ci-0363b050+P REF=e14-PV7 NEWPORT=5063 PREVPORT=5061 MAXWAIT=15 QUIET_LOAD=8`（`NEW`/`PREV` は表の見出しにだけ使われる）。
3. **preflight**: `./verify-bundle.sh preflight`（NEWPORT = 5063 の Firefox で `webglRenderer` が非 null。P の page は `desk.mode` を出さないので、改修後の `beacon.py` は『`desk.mode` が None で `present D` の run』を WebGL とみなす）。
4. **本体**: `./verify-bundle.sh pure firefox 2`（4 run、約 10 分。各 run の入口は `wait-quiet.mjs`。待ちと MemAvailable・swap は `wait.txt` と `steps.log`）。Firefox の run は `DISPLAY=:77`（WebGL あり = llvmpipe）。
5. **解析**: `python3 tools/p3-win.py firefox`（表は `out/P3-firefox.md`: 仮想 s/実 s、present/s、lagCuts/s、underrun、書き込み率、Worker CPU、音楽位置で揃えた bin の壁時計）。**再生位置の比較**（gpu1 の『30 s 後の再生位置』）は各 run の `audio-p3.json` から: 再生開始の印 `marks.play` の epoch を t0 として、`pulses` の `virt`（仮想時計、100 ns 単位）を t0 + 30 s で線形補間し、(virt(t0+30 s) − virt(t0)) ÷ 1e7 = 再生位置（秒）。`p3-win.py` が同じ補間（`at(t, 'virt')`）を持つ。
6. **記録**: `RESULTS` に 4 run の表（load・I/O 圧・待ち秒・MemAvailable・swap・他の Firefox/Chrome の数）。

## 4 判定（事前に決める。仮）
- **再現した**: pv7s の 2 run の仮想 s/実 s の**最大**が p3 の 2 run の**最小**より小さく、かつ組ごとの中央値の差が 0.05 以上（約 6%）。→ P + V7 固有の遅さ。次は計器つき（P の wake latency、Firefox の `waitAsync` の起床）。
- **再現しない**: 4 run の範囲が重なる（pv7s の最大 ≥ p3 の最小）。→ 前回の 0.765 は run の揺れ。『Firefox の速さは未決』を閉じる。
- **どちらとも言えない**: 開始 load が 8 を超えた、他の担当の Firefox/Chrome が重なった、Firefox の WebDriver が切れた run がある。→ 参考扱いと明記し、追加の 2 組。
- 絶対値は過去の run と比べない（基準が動いている。4e 06:42 の (4)）。同じ窓の P 対 PV7 の差だけを読む。

## 5 注意
- Firefox の `Atomics.waitAsync` は 145 以上で P の経路に入る（158 は入る）。P が使えない構成では旧経路に戻り、この比較は意味を失う。`webglRenderer` と `engine.present = D` を各 run の beacon で確認する。
- Firefox の WebDriver は E9・E13 で切断が起きた（エンジン停止ではない。原因不明）。切れた run は捨てずに記録し、再実行を足す。
- lat（入力の T）はこの手順に含めない。
