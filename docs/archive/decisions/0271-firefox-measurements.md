# ci1 → PM 決定 0271 報告2（最終）: Firefox 158 での本番 bundle `fl-engine-ci-ff5ee429`（Worker a3fc8c43 のページ）の実測 — 入力遅延・Mixer ドラッグ・再生中の Worker・underrun・first idle（cold/warm）・deskDump の差分

## 先頭の事実（実エンジン・実ブラウザ。依頼者の機械は 0 件。値の横に load と I/O 圧 avg10 を併記）
1. **Firefox の測定条件を訂正**: 08:15 の再起動後の機械には `DISPLAY` が無く、Firefox が WebGL を作れず 2D 描画で動いていた（報告1 の Firefox first idle は 2D の値）。以後の Firefox の測定は `Xvfb :77` + `DISPLAY=:77`（WebGL = llvmpipe、`desk.mode = gl`）で、再起動前と同じ構成。各 run の beacon で `webglRenderer` と `desk.mode` を確認した: Xvfb を使い始めた後の Firefox の first-idle beacon は **34/34 が `llvmpipe`・`gl`**（ff5ee429 25 件、df0512b0 9 件）、Chrome も gl（SwiftShader）。下の表の Firefox の値は全て gl。
2. **Firefox の再生中の Worker は 1 コアを使い切る**: 参照プロジェクト（serina1-serum2.zip）の再生中の Worker CPU は **970 / 999 / 1,002 ms/s**（10 s 窓 3 つ、load 7.5〜8.6）。停止中（プロジェクトを開いた後）484 ms/s、メイン画面の idle 200 ms/s。同じ手順の Chrome 156 は 895 / 910 / 915 ms/s（load 9.46 / 9.25 / 8.83）、df0512b0 の Firefox は 973 / 995 / 974 ms/s（load 8.49 / 9.43 / 13.95）で、ff5ee429 の Firefox（969 / 999 / 1002 ms/s、load 8.54 / 8.57 / 7.48）とほぼ同じ。**Firefox は再生中、Worker が壁時計の 97〜100% を使う（余裕がほぼ無い）**。
3. **Firefox の入力遅延**（gpu1 の方法: ページの `LatencyTracker`、T = ページが送ってから Worker が提示するまで、`lat-summary.py`、zip でプロジェクトを開く経路。load 7〜12、I/O 圧 ≈ 0、WebGL あり。他の run と一部重なる）— 中央値 / p90 / p99（ms）:

| 場面 | 入力 | n | T 1 回目（load 8.4/8.0/6.9/11.9、他の run と一部重なる） | T 2 回目（load 6.3/6.0/3.7/**15.0**） | gpu1 の Chrome（低 load 7〜11） |
|---|---|---|---|---|---|
| idle | File クリック（button down） | 36 | **11.1 / 16.3 / 28.7** | **10.9 / 16.8 / 31.0** | 11〜14 |
| idle | Esc（key down） | 35 | 3.4 / 5.8 / 11.1 | 3.2 / 5.2 / 12.5 | — |
| Mixer ヘッダードラッグ | 1 移動 | 80 | 9.5 / 25.9 / 46.2 | 10.1 / 26.0 / 44.0 | — |
| 再生の開始（Space） | key down | 16 | 36.5 / 75.9 / 95.8 | 32.8 / 54.7 / 58.9 | — |
| 再生の停止（Space） | key down | 16 | 36.7 / 78.3 / 103.4 | 36.3 / 68.6 / 72.7 | — |
| 再生中 | File クリック（button down） | 35 | **197.6 / 398.3 / 419.2** | **240.7 / 515.6 / 831.6** | 41（p90 174）・高 load 238〜298 |
| 再生中 | Esc（key down） | 36 | 117.6 / 438.5 / 493.6 | 109.1 / 424.8 / 530.0 | — |
  - 再生中の T の内訳（平均）: ページが送ってからエンジンが受け取るまで（B）が 62%、エンジンが idle に戻るまで（C）が 38%（ボタンを押した入力）。idle の File クリックは C が 92%（エンジンの処理）。**Firefox の再生中のクリック T は Chrome の低 load の値（41 ms）の約 5 倍**。
  - **再生中のクリックの T は load に強く依存**（Firefox: load 12 で 198 ms、load 15 で 241 ms、p99 は 419 → 832 ms）。**Firefox で load ≤ 7 の再生中の値は取れなかった**（再生中の場面は 2 回とも他の担当の run が重なって load 12〜15 だった）。idle の入力（File クリック 11 ms・Esc 3 ms）は load 3〜8 で gpu1 の Chrome と同じ水準。
  - 生データ: `data/lat/`（1 回目 = `lat-*.json` と `lat-summary-overlap-run.txt`、2 回目 = `ff-new-2/`）。
4. **Mixer ヘッダードラッグ・ピッカー・F9（Firefox、df0512b0 と交互、WebGL あり）**:
  - Firefox 158、WebGL あり（DISPLAY=:77）、df0512b0（old）と ff5ee429（new）の交互 **2 往復**（時間切れ。load は他の担当の Chrome run が重なって 5.5〜13.6、I/O 圧 avg10 ≈ 0〜1.4）。Worker CPU / 1 移動（ms）、ピッカー（ms/s）:

| scenario | measure | bundle | n | median | runs (load at start) |
|---|---|---|---|---|---|
| move-1920 | mixer-drag | new | 2 | 9.46 | 9.65 (7.96), 9.26 (5.53) |
| move-1920 | mixer-drag | old | 2 | 8.37 | 9.37 (6.64), 7.37 (9.43) |
| move-3240 | mixer-drag | new | 2 | 11.43 | 12.50 (8.21), 10.35 (8.43) |
| move-3240 | mixer-drag | old | 2 | 11.59 | 11.84 (7.49), 11.35 (7.69) |
| picker-cost | fadein-0.8s | new | 2 | 850.00 | 830.00 (9.7), 870.00 (13.55) |
| picker-cost | fadein-0.8s | old | 2 | 855.00 | 840.00 (9.35), 870.00 (9.38) |
| picker-cost | fadeout-1.2s | new | 2 | 800.00 | 780.00 (9.7), 820.00 (13.55) |
| picker-cost | fadeout-1.2s | old | 2 | 795.00 | 890.00 (9.35), 700.00 (9.38) |
| picker-cost | idle-after | new | 2 | 402.50 | 425.00 (9.7), 380.00 (13.55) |
| picker-cost | idle-after | old | 2 | 375.00 | 455.00 (9.35), 295.00 (9.38) |
| picker-cost | idle-main | new | 2 | 195.00 | 195.00 (9.7), 195.00 (13.55) |
| picker-cost | idle-main | old | 2 | 172.50 | 185.00 (9.35), 160.00 (9.38) |
| picker-cost | steady-3s | new | 2 | 416.65 | 433.30 (9.7), 400.00 (13.55) |
| picker-cost | steady-3s | old | 2 | 356.65 | 403.30 (9.35), 310.00 (9.38) |

  - F9 の古い帯（x1903-1912, y548-1019）: **old・new とも 0 画素**（各 2 回）。下端（y1022-1040）の差: old 1,478 / 1,478、new 876 / 876（Chrome の値と同じ）。
  - 読み: Mixer ドラッグは 1920x1080 で new 9.46（9.26〜9.65）、old 8.37（7.37〜9.37）、3240 で new 11.43、old 11.59。**差は n=2 では判定できない**（範囲が重なる／new がやや高い側）。Firefox は Chrome（6.4〜6.8 / 4.9〜5.1 ms）の 1.4〜2.4 倍。再起動前の 4 往復（load 20〜28、WebGL あり、ゲスト 1920x994）は 10.62 / 11.01（1920）、9.11 / 8.83（3240）で差なし。
5. **再生中の underrun と beacon**（Firefox、zip の再生 30 s、`audio.detail` の統計、ff5ee429 と df0512b0）:

| 構成（load 7.5〜14） | underrun（量子 / エピソード）再生前 | 再生 30 s 後 | 再生中の増加 | engineLateWrites（再生前 → 後） | fillFrames |
|---|---|---|---|---|---|
| Firefox ff5ee429 | 3328 / 1 | 3328 / 1 | **+0 / +0** | 16 → 90 | 2646 |
| Firefox df0512b0 | 5469 / 2 | 5469 / 2 | **+0 / +0** | 28 → 91 | 2646 |
| Chrome ff5ee429 | 1842 / 5 | 1913 / 13 | **+71 / +8** | 31 → 103 | 2646 |
  - 再生前の underrun はプロジェクトを開く間（エンジンが音を書く前に AudioContext が走っている間）の値。**再生中の 30 s で Firefox は underrun 0（ff5ee429・df0512b0 とも）、Chrome は他の担当の Chrome run と重なる load 9〜13 で短い underrun が 8〜9 回（停止後まで含めて +9）**。`audio.detail` は今日の first-idle の beacon 102/102（Firefox 2D・WebGL、Chrome）に入っている。
6. **first idle の cold / warm（WebGL あり、I/O 圧 ≈ 0、load 3.5〜5.9、1 セッション 7 回読み込み）**: Firefox cold 6.24 s、warm 5.31 / 6.16 / 5.59 / 5.89 / 5.64 / 6.53 s（中央値 5.74）。Chrome cold 7.42 s、warm 5.60 / 6.59 / 5.88 / 7.31 / 5.66 / 6.21 s（中央値 6.05）。新しいプロファイルの単独 run（WebGL あり、cold、I/O 圧 ≈ 0）は、load 4 で 5.19 s（1 回）、load 12.3〜13.6 で **5.98 / 5.59 / 9.03 / 6.36 / 7.44 / 6.32 s（中央値 6.34 s、n=6）**。2D（WebGL なし）の Firefox の単独 cold は 4.63 s（n=19）。→ **Firefox の first idle は WebGL ありで 5.2〜6.3 s（load 4〜5、I/O が静かなとき）、load 12〜14 で 5.6〜9.0 s（中央値 6.3 s）**（どちらも P1 の 10 s 以内）、I/O が混むと 16 s 前後・最大 37 s（報告1）。
7. **deskDump の差分** `ci1-deskdump.diff`（HEAD cf0f7cff に dry-run 清潔、`-` 行 1）: `app/src/engine/worker.ts` の `deskDump` の `mem.buffer.slice(a, b)`（wasm メモリは SharedArrayBuffer で、その `slice()` も SharedArrayBuffer を返すため、transfer list に入れると `DataCloneError: Shared memory objects must not be in the transfer list`。WebGL なしの Firefox の 2D 経路で発生）を `new Uint8Array(mem.buffer, a, b - a).slice().buffer`（非共有の ArrayBuffer に複製）にする 1 行。単体テスト `app/test/unit/desk-dump-copy.test.ts`（2 件: SharedArrayBuffer の `slice()` は共有のまま transfer できない / Uint8Array の `slice()` の複製は通常の ArrayBuffer で transfer できる）。診断用の複製ページで Firefox（WebGL なし）の `deskDumpBase64(true)` が 75 ms で OK（直す前は無限待ち。`window.__app.error` が `DataCloneError`）。vitest 2 件 ok、`npm run typecheck` ok。影響は `window.__app.deskDumpBase64`（テスト補助）だけで、ユーザーの操作には出ない。

## 未確認・できなかったこと
- 依頼者の機械（Windows の Firefox 159・実 GPU）の値は 0 件。この機械は headless、WebGL は llvmpipe（ソフトウェア）。
- Firefox の入力遅延は 2 回の run。再生中の場面は 2 回とも load 12〜15（他の担当の run が重なった）で、**Firefox の低 load（≤7）での再生中の値は無い**。
- Mixer ドラッグの Firefox 4 往復の静かな窓での取り直しは 2 往復まで（時間切れ）。再起動前の 4 往復（load 20〜28、WebGL あり）は報告1 の末尾に記載。
- 同一セッション内の再読み込み（warm）が新しいプロファイルの単独 run より遅い理由は未確認。

## 自己申告の逸脱
- 再起動後の Firefox の遅さ（first idle 12〜43 s）を『I/O 圧との相関』として報告1 に書いたが、その run は WebGL なし（2D）で、WebGL ありの値と構成が違った（I/O 圧との相関自体は run ごとの記録で確認できた事実）。
- 再起動後、sdrive 系の Firefox 測定が約 40 分止まった（2D 経路の deskDump の DataCloneError。ツールが無限に待つ作りだった）。その間に空いた静かな窓（08:55〜09:12）で、WebGL なしの first idle を多数取った。
- `sleep 1` を含むコマンドを 1 回出して拒否された（実行されず）。
- c5/ffprof を以前の後片付けで消していたため、再起動直後の最初の Firefox の run が空振りした（再作成）。
- OPFS の分解測定の scenario は戻らず強制終了（原因は上記の deskDump 待ちではなく別の `execute/async`。結果なし）。

## 成果物（`lift/out-git/ci1-results/0271/`）
`REPORT-0271-1`、`REPORT-0271-2`、`INFO-0271-2`、`ci1-deskdump.diff`、`data/`（`first-idle-runs.json`、`first-idle-all.json`、`warm/`、`lat/`、`ab/`、`play/`）、`tools/`。



## PM の決定と取り込み(2026-11-06 10:15、docs のみ)

取り込み: ci1 の報告 1・2・INFO を `docs/investigations/ci1/0271/`(data と tools は 400 KB 超の `audio-bp.json` 2 本を除く。原本は `lift/out-git/ci1-results/0271/` に残す)に置いた。測定は実 Firefox 158・Chrome 156・この機械(headless、WebGL は llvmpipe / SwiftShader)で、**依頼者の機械(Windows の Firefox 159・GTX 980)は 0 件、本番 R2 の `perf/` に beacon は 0 件**。

判断:
1. **報告 1 の Firefox の first idle(静かな窓 4.63 s、I/O が混む窓 16.07 s)は、`DISPLAY` が無く WebGL なしの 2D 描画の値**。WebGL あり(Xvfb :77)は cold 5.19〜6.3 s(静かな窓)、load 12〜14 で 5.6〜9.0 s、warm の中央値 5.74 s(n=6)。Chrome は cold 5.21 s(n=15)、warm 6.05 s(n=6)。**いずれも静かな窓では P1(10 s)内**。I/O が混むと 16 s 前後・最大 37 s(2D の値)。以後の Firefox の測定は `DISPLAY=:77` で beacon の `webglRenderer` と `desk.mode` を各 run に記録する。
2. **Firefox の再生中は Worker が壁時計の 97〜100%(970〜1,002 ms/s)を使い切る**(Chrome 895〜915 ms/s)。再生中の File クリックの T は load 12〜15 で中央値 198〜241 ms(p99 419〜832 ms)。Firefox の低 load の再生中の値は無い。idle の File クリックは 11 ms で Chrome と同水準。再生中の underrun は Firefox が +0(ff5ee429、df0512b0 とも)、Chrome が +71 量子 / +8 エピソード(load 9〜13)。
3. `ci1-deskdump.diff`(`app/src/engine/worker.ts` の `deskDump` の `mem.buffer.slice` を非共有コピーにする 1 行 + 単体テスト 2 件)は採る。**app だけの差分**で、PM が app の連鎖を回して、CI が空いたときの app だけの push に載せる。影響は `window.__app.deskDumpBase64`(テスト補助)だけ。
4. 未確認: 依頼者の機械、Firefox の低 load の再生中の入力遅延、Mixer ドラッグの Firefox 4 往復の静かな窓での取り直し、warm が新しいプロファイルの単独 run より遅い理由。
