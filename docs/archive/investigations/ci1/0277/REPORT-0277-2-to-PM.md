# ci1 → PM 報告 0277-2（全表）: 新本番 bundle `fl-engine-ci-d9c160aa`（Worker 51f102d8、HEAD 0b228624）

全ての値は `RESULTS-1115.md`（1 つの表。各値に load と I/O 圧）。生データ・キットは同じディレクトリ。

## 先頭の事実（実エンジン・実ブラウザ Chrome 156 / Firefox 158。代役・再生・推測なし）
- bundle は R2 から読み取りで取得、manifest の size と sha256 が全て一致（第一報のとおり）。ページは CI の headSha 0b228624 の app。Firefox は Xvfb :77、beacon で `webglRenderer` = llvmpipe、`desk.mode` = gl（preflight と first idle の全 Firefox run）。シーン・再生・Mixer の run ごとの beacon の webglRenderer は表に取っていない（`DISPLAY=:77` は全 run で設定済み）。
- **Export は Chrome の実エンジンで書かれた WAV が参照と一致**: 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`（私の `sha256sum` でも同値）。所要 358 s、load 8.5→4.3、I/O 圧 0。
- **再生 60 s は停止しなかった（Chrome・Firefox）が、underrun は出ている**（下の表）。『音が途切れない』（P3）は、この環境の静かな窓（load 3〜6、I/O 圧 0）でも満たされていない。

## 判定（決定 0277 の基準）
| 基準 | 結果 | 判定 |
|---|---|---|
| 停止なし | 15 シーン × 2 ブラウザ（11 シーン + plus + Add メニュー fade + F8 picker 2 種）、再生 60 s × 2、Export: 停止 0（d7 は意図した終了） | **満たす**（Firefox d7 の WebDriver 切断 1/4 と d1・d10・d11 の無効 run は下の逸脱） |
| Export sha256 = 381cdb5a…f979 / 22,863,828 B | 一致 | **満たす** |
| first idle ≤ ff5ee429 + 500 ms | Firefox: 新 4,900 / 旧 5,276 ms（中央値、各 6、差 −376 ms）。Chrome: 新 6,514 / 旧 7,946 ms（各 6、差 −1,432 ms）。静かな run（load ≤12・I/O <1）だけ: Firefox 新 4,945 / 旧 5,297（n=4/3）、Chrome 新 6,132 / 旧 6,574（n=2/2） | **満たす**（新が速い方向。Chrome の run 3〜6 は load 9〜20 で参考値） |
| File ×100 が 100/100 | Chrome 100/100、Firefox 100/100（取りこぼし・popup 欠落・不可視 0） | **満たす** |

## 再生 60 s（参照プロジェクトの zip、Space で再生、静かな窓。Worker busy は CPU ÷ wall）
| | Worker busy ms/s（再生中 6 窓: 最小 / 中央 / 最大） | underrun（playing-a の終わり→停止、約 52 s の再生） | load / I/O 圧 |
|---|---|---|---|
| Chrome（新） | 679 / 760 / 862 | **946 quanta・90 episodes**（1 episode 平均 10.5 quanta ≈ 30 ms） | load 3.3〜5.0、I/O 0 |
| Firefox（新） | 706 / 866 / 991 | **685 quanta・69 episodes**（平均 9.9 quanta） | load 3.2〜6.3、I/O 0 |
- 再生前（プロジェクトを開く間にエンジンが止まる分）に Chrome 1,375 quanta・1 episode、Firefox 2,538・1 episode が既に付く。`longestRunQuanta`（最長）はこの『開く間』の値で、再生中の最長ではない。再生中の episode ごとの最長は beacon に無く、**未測定**（平均だけ）。
- underrun は再生の開始から約 10〜40 s 目（audio 時刻 63〜93 s）に集中し、最初の約 10 s と最後の約 10 s はほぼ増えない（Chrome: +0 / +228 / +389 / +308 / +21 quanta の 10 s ごと、Firefox: +0 / +80 / +267 / +338 / +0）。曲の位置で重さが違う。
- 旧 ff5ee429 での同じ再生の値は、この表には**まだ無い**（今、Chrome で旧を流している。load が高い窓なので参考値になる）。

## Mixer ヘッダドラッグ: Worker CPU / 移動（1920x1080、旧新を交互）
| ブラウザ | 旧 ff5ee429 | 新 d9c160aa | 判定 |
|---|---|---|---|
| Chrome（静か: load 5〜8、I/O 0） | 8.75 / 8.76 ms（n=2） | 8.69 / 8.67 ms（n=2） | **差は −1 %（読み取れる差ではない）**。E7b の mixer CPU の効果（locshare・hyb +18）はこの測定では見えない |
| Firefox（3 セッション計 8 組） | 中央値 9.86 ms（範囲 5.75〜13.36） | 中央値 9.96 ms（範囲 8.32〜13.93） | **判定できない**（run ごとのばらつきが中央値の −42 % 〜 +35 % で、旧新の差より大きい）。静かな run だけ: 旧 7.82（n=4）、新 8.38（n=3） |
- Firefox の run は load 5〜24 の間で変動した（他の作業者の job）。個々の値と load は RESULTS の表。

## alpha7
主窓 @0,0 1920x1080（Chrome、1920x1080 のゲスト）。スクリーンショットは `shots/alpha7-d9c160aa-main-1920x1080.png`（ブラウザ枠・チャンネルラック・ミキサー・プレイリスト・右上の窓ボタンが全て見える。ff5ee429 と同じ）。

## 自己申告の逸脱（第一報の 1・2 に追加）
1. （再掲）d1・d10・d11 の最初の run は Sampler.flp 欠落で無効。是正済み。無効 run は `out/invalid-noproj/`（結果表には数えない）。
2. （再掲）Firefox d7 の WebDriver 切断は 4 回中 1 回、原因不明。
3. Firefox の Mixer 測定は 3 回に分けた（最初の 2 往復は load が 5→29 に上がって参考値、次の 3 往復も load 9〜24）。表は全部を載せ、静かな run だけの中央値を別に付けた。**『差が無い』ではなく『Firefox では判定できない』**。
4. 静かな窓で取り直せていない物: Chrome の first idle の run 3〜6、Firefox の Mixer（load ≤12 の run が 4 組だけ）。PM の E9 検証と他の作業者の job で、検証中の load が 6〜29 を上下した。
5. 作業中に `bash` ツール・`sleep`・`&`・`pkill -f` は使っていない。

## 次（11:30 の最終まで）
旧 ff5ee429 の再生（Chrome）を今流している。load が許せば Chrome の first idle の静かな取り直しと Firefox の旧再生。最終報告で RESULTS を更新する。

## 成果物（`lift/out-git/ci1-results/0277/`）
`REPORT-0277-1-to-PM.md`、`REPORT-0277-2-to-PM.md`、`RESULTS-1115.md`、`kit/`、`shots/`、`data/`（`export.json`、`ab-*.tsv`、`play-*-report.json`、`idle-*.tsv`、`scenes.json`、`steps.log`、`fetch-*.txt`、`preflight.txt`）。
