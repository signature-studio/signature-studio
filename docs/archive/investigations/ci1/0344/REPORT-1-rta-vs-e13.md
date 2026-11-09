# 0344-1: G2 を E13 対 RT-A(フックあり実スレッド)で測る — ci1, 2026-11-08

## 実際に実行した物

- 実 Chrome 156 headless(CDP)で実エンジン。シナリオは 0342 と同じ `tools/scn-lat-ff.json`、同じ driver(`sdriveg2-lat.mjs`)、同じ解析の選択規則(`g2-multi.py` = 0342 の `g2-analyze.py` と同じ「直前の dump より後の record」)。
- **RT-A は現存**(`assets-rta`、bundle `fl-engine-test-rt1-91bf78f6`、wasm 158,362,507 B、port 5101、page-rt1)。RT-B への代替は不要。wasm の `g9-diag` 文字列は E13 本番(`fl-engine-ci-0363b050`)0 件、RT-A 5 件、RT-F 5 件(私も `grep -a -o` で確認)。**g9_hook が診断 OFF でも常時 install される点は PM の実測**で、私は確認していない(RT-A の wasm に name section が無く、profile で g9_hook を見ていない。RT-F の trace では g9_hook が phase C の 73–75% の包含時間として見えている = 0343)。
- counted の 2 組(E13 → RT-A の順に 1 組ずつ。gate は run ごとに記録):

  | run | 条件 | 開始〜終了 | 開始前 load | MemAvailable | swap si/so | Wine | 他 Chrome | 結果 |
  |---|---|---|---|---|---|---|---|---|
  | lat-chrome-prev-5 | E13 | 06:34:33–06:38:46 | 0.61 | 18056 MB | 0/0 | 0(開始時)。**06:38:20 頃に opt1 の Wine が起動し、run の最後の約 26 s と重なった**(下の「重なりの記録」) | 0 | ok |
  | lat-chrome-new-5 | RT-A | 06:39:21–06:43:01 | 4.13 | 34726 MB | 0/0 | 0 | 0 | **ok=False: 停止(下記)。counted から除外** |
  | lat-chrome-new-6 | RT-A(再実行) | 06:47:58–06:52:17 | 2.79 | 19591 MB | 0/0 | 0 | 0 | ok |
  | lat-chrome-prev-8 | E13 | 06:59:02–07:03:25 | 3.74 | 21722 MB | 0/0 | 0 | 0 | ok |
  | lat-chrome-new-9 | RT-A | 07:03:46–07:08:10 | 2.39 | 24085 MB | 0/0 | 0 | 0 | ok |

  - 組 a = prev-5 / new-6、組 b = prev-8 / new-9。各 run の開始前 gate(swap si/so と MemAvailable)は `out/steps.log` と各 run の `gate.txt` にある。gate が swap si=4〜8 や Wine で止めた起動は、run していない(下の「起動しなかった試行」)。
- 起動しなかった試行(gate による停止): 06:32 swap si=8、06:38 Wine 7 プロセス(opt1 の FL64.exe)、06:44 他 Chrome(gpu1 の Chrome)、06:58 swap si=4。いずれも待って再実行。
- 中止した run: 06:52:28 に始めた RT-B(`lat-chrome-new-7`)は、私の判断で 06:54 に止めた(指示は RT-A だけで、RT-B は RT-A が走らなかった場合の代替だったため)。結果なし。ログのみ `lat-chrome-new-7.killed.log`。
- 重なりの記録: E13 組 a の最後(06:38:20 頃〜)に opt1 の Wine(FL64.exe)が起動した。**E13 組 a の再生中 record 35 件のうち、Wine 起動後の 10 件(File click)を分けて見ると中央値 21.99 ms、起動前の 25 件は 19.84 ms(全体 20.14)**。差は +2 ms で、組 b(Wine なし)の E13 は 21.35 ms。この重なりは E13 の値を大きくは動かしていない。ただし厳密には「Wine 0」を満たさない区間を含むので、組 a の E13 の再生中の値は **参考**とし、組 b を基準にする。

## 注意: RT-A の run lat-chrome-new-5 が停止した

- 結果: `STOPPED: first non-background pixel on a drawing surface (decision 0126) ... [guest return 0x6d8d02]`(この 1 行目は最初の画素の証拠の文で、停止の原因は次の行)、`exit code 134 (SIGABRT)`、`fle: DeleteTimerQueueTimer: CompletionEvent is not NULL while the callback is running: waiting for a callback is not modelled`。
- **起きた場所**: a-idle-menu・c-mixer-drag・d-space の dump は書かれ、**b-play-menu の dump は無い**。report の `inputs` の最後は 0.3 s 前の `button L up at screen (18, 20)`(File メニューのクリック)で、windowsEnd に `TQuickPopupMenuWindow` が見えている。シナリオの順序(Space を押して再生 → 10 s 待つ → File メニュー 35 回 → b-play-menu の dump)から、**再生中の File メニュー操作の途中**で止まったと読める(順序と dump の有無からの推定。どの回で止まったかは未確認)。
- 実エンジンが止まった事実で、**未調査**(原因は確認していない)。同じ構成の RT-A の再実行(new-6、new-9)は 2/2 で停止せず完走した。**3 本中 1 本**。
- 停止メッセージの文面は「DeleteTimerQueueTimer … waiting for a callback is not modelled」で、(b) 外部 API の契約の不足(CompletionEvent を待つ契約が未実装)を指す形。**これは停止の文面から読めることで、この run だけでは因果を確認していない。** PM / mt3 に原因調査を依頼したい。
- この run の途中までの record(idle / drag / Space)は、解析に使っていない。

## 結果(post-play の File メニュー操作。中央値、p90、n)

組 a(E13 = prev-5、RT-A = new-6)

| 入力 | 指標 | E13 | RT-A | 差(RT-A − E13) |
|---|---|---|---|---|
| idle File click | T | 6.54 (7.88) n=36 | 8.26 (9.62) n=36 | +1.72 |
| idle key down | T | 2.06 (2.90) n=33 | 2.32 (3.29) n=35 | +0.26 |
| **再生中 File click** | **T** | 20.14 (27.29) n=35 | 46.38 (54.46) n=35 | **+26.24** |
| 再生中 File click | page→inject | 0.83 (3.78) | 1.98 (8.22) | +1.15 |
| **再生中 key down** | **T** | 4.78 (7.02) n=36 | 9.76 (20.75) n=36 | **+4.98** |

組 b(E13 = prev-8、RT-A = new-9)

| 入力 | 指標 | E13 | RT-A | 差(RT-A − E13) |
|---|---|---|---|---|
| idle File click | T | 7.12 (9.39) n=36 | 9.10 (12.45) n=36 | +1.98 |
| idle key down | T | 2.40 (3.19) n=33 | 2.74 (3.50) n=35 | +0.34 |
| **再生中 File click** | **T** | 21.35 (25.13) n=35 | 45.12 (57.18) n=35 | **+23.77** |
| 再生中 File click | page→inject | 0.50 (3.22) | 1.38 (8.70) | +0.88 |
| **再生中 key down** | **T** | 5.82 (9.00) n=35 | 12.91 (22.59) n=34 | **+7.09** |

組ごとの表は `g2-pair-a.md`、`g2-pair-b.md`。run 間のばらつき: E13 同士(組 a と b の間)は再生中 File click +1.21 ms、key down +1.05 ms。RT-A 同士は File click −1.26 ms(46.38 → 45.12)、key down +3.15 ms(9.76 → 12.91)。**ばらつきは E13 で約 1 ms、RT-A で 1〜3 ms**(各 n=1 組なので、幅としては粗い)。

pointermove の queue 投入(Mixer drag、`tInject − tEvent`)は 0342 の指標どおり: 組 a で E13 5.13 / RT-A 5.14(差 +0.01)。ほぼ同じ。

## 判定(測れた範囲)

1. **RT-A(フックあり実スレッド、RT-F より前の bundle)でも、再生中 File click は E13 比で +26.24 / +23.77 ms、key down は +4.98 / +7.09 ms。** RT-F の +23.37(0342)/ +10.71 ms と同程度。**RT-A も g9_hook(census hook)を持つ**ので、RT-F の遅れが RT-A と同程度なのは、「フックだけが原因」ではなく「フックを持つ実スレッド版に共通」だと読める。ここで **フック由来か実スレッド由来かの分離はできていない**(RT-A も RT-F も両方持つため)。分離には、フックなしの実スレッド版(mt3 の RT-H)が要る。
2. 再生中の key down は +5〜+7 ms で、+10 ms の範囲内(RT-A の 2 組)。File click は +24〜+26 ms で範囲外。0342 の RT-F(key +10.71)と並べると、key down は RT-A の方が小さいが、標本が 34〜36 件で、E13 の run 間のばらつき約 1 ms と RT-A の組間の差 3.15 ms(key down)を考えると、**RT-A と RT-F の key down の差は、この測定では区別できない**。
3. 判定に使う前提: RT-A と RT-F は bundle が違う(RT-F は O20 の仮想 pointer・show-descendants などが入っている)ので、「RT-A も同程度」は「その後の変更で悪化した/改善した」を否定も肯定もしない。

## 未取得 / 未確認

- フックなしの実スレッド(RT-H)の G2 → RT-H の bundle 待ち(PM の連絡待ち)。届いたら同じ `scn-lat-ff.json` で E13 / RT-F / RT-H の 3 者を測る。
- lat-chrome-new-5 の停止の原因(未調査)。
- E13 のフック有り版(E13 に g9_hook を足した版)は存在しないので、「E13 + フック」の対照は未取得。
