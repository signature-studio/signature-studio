# ci1 の検証キット（決定 0277）: 新しい本番 bundle の実ブラウザ確認

実エンジン（本番配信バケットの bundle）を実ブラウザ（Chrome 156 / Firefox 158）で動かす。代役・再生・推測なし。CI には載せない（ブラウザテストは CI に入れない方針）。

## 使い方（順番）
環境: `kit/` で作業する。`export NEW=<bundle> REF=<ページの git ref（= Worker のビルドの headSha）> WORKER=<Worker version>`、比較の相手は `PREV`（既定 `fl-engine-ci-ff5ee429`）。
Firefox は `DISPLAY=:77` の Xvfb が要る（WebGL。無いと 2D 表示の別の値になる）。

1. （背景）`exec ./start-xvfb.sh`
2. `./verify-bundle.sh fetch $NEW`; `./verify-bundle.sh fetch $PREV`: R2 から読み取りで取得し、manifest の全ファイルの size と sha256 を照合（不一致は停止）。ローカルの `.br` は品質 6 で作る。
   `./verify-bundle.sh proj`: `kit/proj` = 配信する `/assets/proj/`（参照プロジェクト 49 ファイル + zip + `Sampler.flp` + `SHP_Sawing Synth.wav`）。d1・d10・d11 は Sampler.flp を開く。**無いと 404 の本文が『プロジェクト』として開かれ、エンジンが名前つき停止する（0277 で一度これを踏んだ）**ので、preflight が 4 ファイルの 200 とサイズを確認し、違えば rc 3 で止まる。
3. `./verify-bundle.sh page $REF`: その ref の `app` をビルド（vite のキャッシュは `kit/vite-cache`。`app/node_modules` は読むだけで、書かない）。
4. （背景）`exec ./serve.sh 5040 $NEW $REF` と `exec ./serve.sh 5041 $PREV $REF`
5. `./verify-bundle.sh preflight`: Firefox の beacon が `webglRenderer` 非 null かつ `desk.mode = gl` でなければ rc 2 で停止（その場合は Firefox の値を取らず報告する）。
6. 単独の測定（他の自分の job を並べない）: `idle chrome 6`、`idle firefox 6`（前と新を交互、各 run の load・I/O 圧・webglRenderer・desk.mode）。
7. 2 本まで並行して可: `scenes chrome` ‖ `scenes firefox`（11 シーン + 追加プラグイン + Add メニュー/picker の fade。File ×100 は d8）。
8. `alpha7`（主窓の位置と大きさ + スクリーンショット）、`export`（参照プロジェクトを開き Ctrl+R、書かれた WAV の size と sha256）、`play chrome` / `play firefox`（参照プロジェクトの zip を 60 s 再生: 停止なし、Worker の busy ms/s、underrun の episodes / 最長）、`ab chrome` / `ab firefox`（Mixer ヘッダドラッグの Worker CPU/移動、前と新を交互 2 往復）。
9. `./verify-bundle.sh report` → `out/RESULTS.md`（全結果 1 つの表。各値に load と I/O 圧）。
10. `./verify-bundle.sh clean`（`readlink -f` で kit の下と確認してから削除）。サーバーと Xvfb は PID を `ps` で読んで `kill`。

## 判定基準（RESULTS.md の先頭の表）
- 停止なし（d7 は意図した終了）。
- Export の WAV が 22,863,828 B / sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` と一致。
- first idle の中央値が PREV に対し +500 ms 以内（Chrome、Firefox。WebGL の run のみ）。
- File メニュー ×100 が 100/100（Chrome、Firefox）。

## 測定の規則
- 各 run の開始・終了の load と `/proc/pressure/io` の avg10 を値の隣に書く。I/O 圧が主な条件で、load だけでは決めない。
- Firefox は 2D 表示（WebGL なし）の値と別の表にする。
- 重い job を並べた run は「参考値」。load を隣に書く。

## 中身
`verify-bundle.sh`（入口）、`env.sh`、`serve.sh`、`start-xvfb.sh`、`tools/`（`sdrive-a1.mjs` = シーン・再生・ドラッグ測定の駆動、`sdriveg2-export.mjs` = Export、`entry-check.mjs` = 第一アイドル、`beacon.py`、`report.py`、`bundle-files.py`、`mkbr.mjs`、`gd-wrap.sh`、`scn-*.json`）。
`kit/ffprof`（Firefox のプロファイル置き場）は空のまま残す。geckodriver は `--profile-root` が無いと /tmp にプロファイルを作り OPFS が壊れる。
