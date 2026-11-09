# ci1 → PM 報告 0277-3（最終）: 新本番 bundle `fl-engine-ci-d9c160aa`（Worker 51f102d8、HEAD 0b228624）

全表: `RESULTS-final.md`（1 つの表。各値に load と I/O 圧）。第一報・全表の報告（0277-1、0277-2）の続き。

## 先頭の事実（実エンジン・実ブラウザ Chrome 156 / Firefox 158。代役・再生・推測なし）
- **判定基準 4 つは満たした**: 停止なし（15 シーン × 2 ブラウザ、再生 60 s × 2 ブラウザ、Export）、**Export の WAV が参照と一致**（22,863,828 B、sha256 `381cdb5a…f979`）、first idle が ff5ee429 に対し +500 ms 以内（新が速い）、File ×100 が Chrome・Firefox とも 100/100。
- **再生中の underrun（P3）は、Chrome の旧新 3 組すべてで新が多い向き**（下）。ただし旧の範囲（14〜486）と新の範囲（444〜946）が重なり、n=3 ずつ。**結論にはしない**（PM の指示どおり）。累計値で見ると旧新は区別できない。
- Firefox の Mixer ドラッグ（Worker CPU/移動）は旧新を**判定できない**（run ごとのばらつきが差より大きい）。Chrome は差なし（8.7 ms）。

## ① underrun の量の定義（opt1 の `underrunQuanta` と同じカウンタ、数えた窓が違う）
- カウンタ: ページの `window.__app.audio.detail()` の `underrunQuanta`（AudioWorklet の 128 フレーム ≈ 2.9 ms の描画単位で、出力リングが空だった数。**音のデバイスが鳴り始めてからの累計**。`app/src/audio/underrun.ts`、`ring-layout.ts`）と `episodes`（連続した underrun の塊の数）。opt1 の audio-b.json `detail.underrunQuanta` と**同じカウンタ**。
- 違いは窓: opt1 の 1,603 / 1,565（ff5ee429、60 s 再生）は**累計**で、プロジェクトを開く間の停止（私の run では再生前に 1,375〜2,118 quanta が既に付く）を含む。私の『再生中』は playing-a の終わりと停止後の読みの**差**（約 52 s の再生）で、開く間の分を含まない。累計で比べると私の値も旧 1,809 / 2,140 / 1,942、新 2,342 / 1,908 / 2,407 で、opt1 の 1,600 台と同じ桁、**旧新は重なる**。
- つまり、桁の違いは数えた窓の違い。再生前の停止は旧の方が長い run が多い（旧 1,684 / 2,118 / 1,456、新 1,375 / 1,432 / 1,522。3 組中 2 組）ため、累計では差が小さくなる。

## ② 旧新の再生 60 s（Chrome、同じページ 0b228624・同じ手順、参照 zip を開き Space で約 52 s 再生。時間順に r1→r3、各 run の load と I/O 圧）
| run | bundle | load 開始→終了 (I/O 圧) | Worker busy ms/s（再生 6 窓の最小/中央/最大） | **再生中の underrun: quanta / episodes** | エンジンが自分で書いた gap フレーム（`engineGapFrames` の差） | 累計（終了時） | 開く間の分 |
|---|---|---|---|---|---|---|---|
| r1 | 新 | 4.1→4.8 (0) | 679 / 760 / 862 | **946 / 90** | 113,348 | 2,342 / 94 | 1,375 / 1 |
| r1 | 旧 | 15.2→9.3 (0) | 819 / 878 / 900 | 116 / 30 | 12,268 | 1,809 / 32 | 1,684 / 1 |
| r2 | 新 | 8.8→8.6 (0) | 753 / 807 / 833 | **444 / 61** | 51,586 | 1,908 / 66 | 1,432 / 2 |
| r2 | 旧 | 8.3→11.4 (0) | 885 / 898 / 908 | 14 / 5 | 1,362 | 2,140 / 8 | 2,118 / 1 |
| r3 | 新 | 10.4→8.8 (0) | 683 / 798 / 885 | **852 / 82** | 102,004 | 2,407 / 86 | 1,522 / 1 |
| r3 | 旧 | 8.8→5.7 (0) | 745 / 824 / 894 | 486 / 60 | 57,048 | 1,942 / 61 | 1,456 / 1 |
- 時間順は 新 r1（10:54、静か）→ 旧 r1 → 新 r2 → 旧 r2 → 新 r3 → 旧 r3。I/O 圧は全 run 0。load は 4〜15 で旧新が揃っていない。
- 3 組すべてで新が多い（946 vs 116、444 vs 14、852 vs 486）が、旧 r3（486）は新 r2（444）より多い。旧の run 間で 14〜486 と 35 倍動く（PM が見た opt1 の 2〜3 倍より大きい）。
- Firefox の再生は新 1 回だけ（685 quanta・69 episodes、load 3〜6）。**Firefox の旧は未実施**。
- 『エンジンが自分で書いた gap フレーム』は、エンジンが publish に間に合わず無音で埋めた量（`page-audio.ts` の 0211）。underrun の多い run で多い（新 113k / 52k / 102k、旧 12k / 1.4k / 57k）= **遅れはページ側のリングではなくエンジン側で起きている**（私の読み）。

## ③ 傾向と次
- 傾向は『新が多い向き』で 3 組残った（n=3 ずつ、範囲は重なる）。PM が決める通り、opt1 の audioDump 側（同じ run 内で write rate・consumed の時系列がある）の同じ比較が、私の beacon 差分より情報が多い。私はこの結論を出さない。

## ④ 機構の仮説（私の読み。**仮説であって検証していない**）
- H1: Worker busy が新で低い（680〜890 vs 旧 750〜910）のに underrun が多いのは、私が測っている『エンジンの Worker』の CPU が、音を作る別スレッド（ゲストの mixer スレッド。mt3 の f_310fb40 は mixer スレッドの関数）の遅れを含まないから。新は mixer スレッドが主 Worker を待つ/待たせる位相が変わった、という読み。確かめるには mixer スレッド側の CPU を別に取る必要がある。
- H2: 新は Worker が空く（IdleDriver の lead 待ちが増える）ため、publish の周期が揃わず、重い区間（audio 時刻 63〜93 s の 10 s×3 窓）で 1 周期遅れると hole になる。旧は飽和して常に書いているので周期が一定になる。この区間に underrun が集中する（Chrome r1: +228 / +389 / +308 quanta の 10 s ごと、他の窓は ~0〜21）。
- どちらも『一部の窓だけ重い』という観測とは合うが、他の読み（run の順序・時刻、ブラウザの背景負荷、メモリ配置）も排除していない。

## first idle（Chrome の静かな取り直しを含む。単独 run、冷間、旧新交互、各 run の load・I/O 圧・webglRenderer・desk.mode は RESULTS-final.md）
| ブラウザ | 旧 ff5ee429 | 新 d9c160aa | 差 |
|---|---|---|---|
| Chrome 静かな run（load ≤12・I/O <1、旧新 6 組ずつ） | 中央値 5,996 ms | **5,403 ms** | **−593 ms** |
| Chrome 全 run（取り直し 4 組を含む、新旧 10 run ずつ） | 6,396 ms | 5,958 ms | −438 ms |
| Firefox（全 12 run が load 8〜15・I/O ≤0.1） | 5,276 ms（n=6） | **4,900 ms**（n=6） | −376 ms |
- 取り直し（11:27、load 5.5・I/O 0）の Chrome 4 組: 新 5,457 / 5,349 / 4,830 / 4,946、旧 6,243 / 5,349 / 5,529 / 5,749 ms。第一報の 6 組は run 3〜6 が load 9〜20 で参考値。
- P1（起動 10 s 以内）: この環境の静かな window では Chrome・Firefox とも 4.8〜6.2 s。**オーナーのマシンの値は 0（未測定）**。

## 自己申告の逸脱（0277-1・0277-2 の分に追加）
1. 旧新の再生比較は Chrome だけ、順序は新→旧→新→旧→新→旧で交互になっているが、load が揃っていない（4〜15）。Firefox は新だけ。
2. 再生中の『episode ごとの最長』は beacon に無く未測定（`longestRunQuanta` は開く間の停止の値）。
3. `sleep` / `&` / `pkill -f` / `bash` ツールは今回の作業で使っていない。無効 run（Sampler.flp 欠落）は `out/invalid-noproj/` に残し、表に数えていない。

## 後始末
`verify-bundle.sh clean`（bundles・page・assets・ffprof・tmp・vite-cache）、n1/bundle、サーバー（5040/5041）と Xvfb を停止、`lift/out-ci1` は 500 MB 以下、`c5/ffprof` は空。**実施済み（11:29）**: `lift/out-ci1` = 138 MB、サーバー（5040/5041）と Xvfb :77 は停止、`c5/ffprof` は空、`kit/ffprof`・`kit/tmp`・`kit/appwork` は空で再作成。`kit/out/`（74 MB: 結果の JSON・TSV・スクリーンショット、Chrome・Firefox のプロファイルは削除）と `kit/proj/`（配信するプロジェクトの symlink）は残した。E8 の確認は fetch・page・サーバー起動からやり直す（約 1 分 + 22 s × 2）。

## 成果物（`lift/out-git/ci1-results/0277/`）
`REPORT-0277-1/2/3-to-PM.md`、`RESULTS-final.md`、`kit/`（検証キット。README に順番と判定基準）、`shots/`、`data/`（`export.json`、`ab-*.tsv`、`play-*-report.json`、`idle-*.tsv`、`scenes.json`、`steps.log`、`fetch-*.txt`、`preflight.txt`）。E8 が出たら同じキットを流す（`export NEW=… REF=… WORKER=…`、`fetch`・`page`・`proj`・サーバー・`preflight` の順）。
