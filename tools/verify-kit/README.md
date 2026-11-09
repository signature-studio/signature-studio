# ci1 の検証キット（決定 0277）: 新しい本番 bundle の実ブラウザ確認

実エンジン（本番配信バケットの bundle）を実ブラウザ（Chrome 156 / Firefox 158）で動かす。代役・再生・推測なし。CI には載せない（ブラウザ試験は CI に入れない方針）。値は全て『その瞬間の load と I/O 圧 avg10』と一緒に書く。

## 環境変数（オプションではなく試験の入力）
`NEW`（新 bundle 名）、`REF`（ページの git ref = Worker のビルドの headSha）、`WORKER`（版）、`PREV`（比較の相手、既定 ff5ee429）、`NEWPORT`/`PREVPORT`（既定 5040/5041）、`MAXWAIT`（待ちの上限、分、既定 15）、`QUIET_LOAD`（report の閾値、既定 12。E11 以降は 8）、`IDLESTART`（idle の組の番号の開始）、`P3START`（pure の組の番号の開始）。

## 使い方（順番）
1. （背景）`exec ./start-xvfb.sh`（Firefox の WebGL に必要。無いと 2D 表示の別の値）。
2. `./verify-bundle.sh fetch $NEW` と `fetch $PREV`: R2 から読み取りで取得し、manifest の size と sha256 を照合（不一致は停止）。`.br` は品質 6 でローカルに作る。
3. `./verify-bundle.sh page $REF`: その ref の `app` をビルド（vite のキャッシュは `kit/vite-cache`。`app/node_modules` は読むだけ）。
4. （背景）`exec ./serve.sh $NEWPORT $NEW $REF` と `exec ./serve.sh $PREVPORT $PREV $REF`。**サーバーが落ちていると『ページが起動しない（no-app）』で 10 分ずつ待って失敗する**ので、`pure`/`lat`/`long` は run の前に 200 を検査して止まる（tool セッションの再起動でサーバーと Xvfb が落ちた事故があった）。
5. `./verify-bundle.sh preflight`: `kit/proj` の 4 ファイルが 200 とサイズで返ること（404 の本文を project として開く事故があった）、Firefox の beacon が `webglRenderer` 非 null かつ `desk.mode = gl` であること（違えば rc 2/3 で停止）。
6. 各モード（下）。測定 run は `wait-quiet.mjs`（load ≤8 かつ I/O 圧 0 を待つ、上限 `MAXWAIT` 分、超えたら『参考（開始 load 超過）』と明記して進む）で始まる。待ち時間は run ごとの `wait.txt` と表に残る。
7. `./verify-bundle.sh report` → `out/RESULTS.md`、`python3 tools/p3-win.py chrome` → `out/P3-chrome.md`、`python3 tools/lat-ab.py chrome` → `out/LAT-chrome.md`、`python3 tools/mkdoc0277eN.py` → `docs/…`（PM の取り込み用）。pure の後の解析とコピー（ブラウザなし）は `tools/post-pure.sh eN`（p3-win・report を実行し、run のファイルと表を `0277/eN/data` に写す）。
8. `./verify-bundle.sh clean`（`readlink -f` で kit の下と確認してから削除）。サーバーと Xvfb は PID を `ps` で読んで `kill`。**走っている `verify-bundle.sh` は編集しない**（bash が読み途中で壊れる）。

## モード
`fetch` / `proj` / `page` / `preflight` / `scenes chrome|firefox [SCENE]`（11 シーン + plus + Add メニュー・F8 picker の fade、File ×100 は d8）/ `alpha7`（主窓の位置 + 画）/ `export`（Ctrl+R の WAV の size と sha256、期待 22,863,828 B・`381cdb5a…f979`）/ `idle chrome|firefox [N]`（単独の first idle、冷間、前後交互、各 run の load・I/O 圧・webglRenderer・desk.mode）/ `pure|p3 chrome|firefox [R]`（参照 zip を開き Space で 60 s、10 s 窓、2 s ごとの snapshot、前後交互。壁時計と音楽位置〔仮想時計〕で揃えた窓、`p3-reference.json` に参考の run を書く）/ `lat chrome|firefox [R]`（再生中・idle の File クリック・Esc の入力の true T）/ `long`（5 分再生）/ `title`（最小化 → `#taskbar-restore` → 復元の 5 変種、CDP の実マウスクリック、題の判定）/ `logo`（起動直後の窓表）/ `minrestore` / `play` / `ab` / `report` / `clean`。

## 判定基準（RESULTS.md の先頭の表）
停止なし（d7 は意図した終了）、Export の WAV が 22,863,828 B・sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` と一致、first idle が比較の相手に対し +500 ms 以内（E11 以降の基準は +100 ms）、File ×100 が 100/100。

## 測定の規則と道具
- 開始時 load ≤8 かつ I/O 圧 0 の run だけ数える。他の Chrome が 2 本以上いた run は『参考（重なり）』。他の Chrome の数は `tools/chrome-count.mjs`（2 秒ごと、`out/chrome-count.tsv`）の観測から付ける（**手書きの重なり記録は、別の run に誤って当たる事故があったので使わない**）。他の担当の作業が重なった run は `out/p3-reference.json` に理由を書くと『参考』に分かれる。
- **メモリ/swap の規則（PM 2026-11-07 04:16）**: 計時の run の開始前に `free -m | sed -n 2,3p` と `vmstat 1 3 | tail -2`（si/so が 0 か）を `out/steps.log` に残し、swap の入出力が 0 でなければ run を始めない。`tools/wait-quiet.mjs` は load ≤8・I/O 圧 0 に加えて `/proc/vmstat` の `pswpin`/`pswpout` が 2 秒の標本で動かないことを待ち、MemAvailable・swap 使用量・swap 入出力を run の `wait.txt` と `steps.log` の行に残す。
- 窓の受け渡しは info 1 行。待機は背景の `tools/wait-until.mjs HH:MM` / `wait-file.mjs PATH` / `wait-no-wine.mjs`（opt1 の Wine が居ないことを確認、上限 15 分）で行い、sleep は使わない。
- `tools/`: `sdrive-a1.mjs`（シーン・Mixer・入力）、`sdriveg2-export.mjs`（Export）、`sdriveg2-lat.mjs`（pure・lat・long・5 分）、`entry-check.mjs`（first idle）、`beacon.py`、`p3-win.py`、`lat-ab.py`、`titlecheck.py`、`logocheck.py`、`pngdiff.py`（画素比較）、`report.py`、`mkdoc0277e*.py`、`scn-*.json`。

## 中身
`kit/ffprof`（Firefox のプロファイル置き場）は空のまま残す。geckodriver は `--profile-root` が無いと /tmp にプロファイルを作り OPFS が壊れる。
