# 0420: R4-5e: rt の Worker の CPU の内訳を記録した結果(製品は変えない)

分類: 測定。a か b かは決めない(main が決める)。製品のコードは変えない。
撤回条件: 候補の原因(Thread<NN> の所属、sys の内訳、待ちの空回りの回数)が後の計数で確定したら、決定 2・4・6 を改める(判定は main)。
日付: 2026-11-17   ROADMAP: R4-5e   一覧: `evidence/R4-5e.md`(§0〜§8)、`evidence/R4-5e/`(表と報告のコピー)、生データ(gitignore された生の出力。commit されない)、コミット: 52b954cc(R4-5e の記録)

## 決定
1. 決定: 製品は変えない。根拠: 測定の道具と記録だけを足した。bundle は作り直していない(ビルドは無し)。道具と駆動の変更は `tools/verify-kit/`(`evidence/R4-5e.md` §1・§8)。
2. 決定: rt の Worker の CPU(P3 の 5 分の窓、plain)は 567.88 s、co は 271.47 s(比 2.09)。増分 +296.4 s は、sys +95.9 s と user +200.5 s に分けて記録する。根拠: `evidence/R4-5e/cpu-p3.tsv`(`evidence/R4-5e.md` §2)。
3. 決定: user の増分のうち、プロファイルの標本で見えるのは次のとおり(割合を見る。以下の秒はプロファイル run の秒であり、plain の user 増分 +200.5 の内訳ではない): rt だけの関数(`fle_gil_wake` 約 31 s・スケジューラの走査 約 26 s・GIL の遅い経路 約 19 s)と、guest 関数 7350・7158・7351 の遅さ(1.36〜1.53 倍。1367 は 0.95 倍)。その他の wasm の差は約 +100 s で、この中に 7350・7158・7351 の差が約 +17 s 含まれる(分けていない)。根拠: `evidence/R4-5e/split-rt3.tsv`・`split-co2.tsv`・`notify-rt3.txt`(`evidence/R4-5e.md` §3)。
4. 決定: GIL の遅い経路の眠り 96%(R4-5d)は、CPU の内訳ではない。眠りは時間だけで CPU を使わない。遅い経路の wall 時間のうち眠り以外は約 4%(引き算。スピンの CPU 時間は未測定)。根拠: `evidence/R4-5d.md` §3.3・`evidence/R4-5e.md` §4。
5. 決定: 他のスレッド(Chrome の `Thread<NN>`)の増分 +230 s は未分類。所属は未確認。根拠: `evidence/R4-5e.md` §2・§3(上位 6 本の rt 249 s・co 164 s は `evidence/R4-5e/r45e-mixplain-p-chrome-{1,2}/report.json` の `profs[0].cpuS.top`)。
6. 決定: Windows の契約との照合: Sleep(0)・Sleep(ms>0)・YieldProcessor(PAUSE)は契約どおり。SwitchToThread は戻り値は契約どおりだが、切り替えられなかった呼び出しが通算 100 万回で `fle_stop` になる(実機に無い)。待ちの空回りの候補(0 ms の待ちが期限を過ぎて返り続ける)は、未計測で未確認。根拠: `evidence/R4-5e.md` §5。
7. 決定: a か b かは決めない。候補と、その分類の候補は `evidence/R4-5e.md` §6 に並べた。根拠: 分類の前提(所属・回数)が未確認のため。
8. 決定(提案。実行は main が決める): 次に必要な計測は、Worker ごとの futex・notify・wait の回数、待ちの 0 ms の戻りの回数、Thread<NN> の所属の確認。計測にはビルド(build-lock)が要る。根拠: `evidence/R4-5e.md` §7。
9. 決定: この記録は完了ではない。120 fps・耳(0 件)・依頼者機(0 件)は未確認のまま。

## 数字の扱い
- 絶対値の CPU は plain の run(プロファイラ無し)。プロファイルの run は標本の割合にだけ使う(p3-play の窓で、プロファイルの run の CPU は rt +14.5%、co +2.1% 多かった)。
- rt の sys は plain の run でだけ分けた(プロファイルの run は sys を分けていない)。Worker の番号は未確認(記録は大きい順に並べ替えて保存される)。

## 追記(2026-11-17): main の分類(本文は変えない)
- 決定(main): a の候補は 3 件。(1) `fle_gil_wake` の、解放ごとの notify(自己時間 約 31 s)。(2) `poll_waiters` の O(n²) の走査(約 26 s)。(3) SwitchToThread の通算 100 万回の停止。(3) は R6-43 で除去中(別の worker)。
- 決定(main の判定): rt は引き続き main への取り込みを行わない。根拠: 0393 の撤回条件は underrun と T の連言で当たらない。ただし T が 2.15〜2.35 倍(`evidence/R4-5c.md` §4)、Export の sha が 4 種、起動が 7.6 秒、CPU が 2.09 倍の 4 点が採用条件を満たさない。
- 決定(main): 未分類の 3 件は次の調査の候補(必要なら R4-5f)。(a) sys の増分 +96 s。(b) 他のスレッド(Thread<NN>)の増分 +230 s。(c) 同じ guest 関数の 1.36〜1.53 倍の遅さ。
- 読み(main の過去の読みの訂正。木の版 03238820 の runtime 配下の fle_gil.h の冒頭の不変条件で確かめた): GIL は、ゲストのコードの実行中は解放される(保持するのは、スレッドが RUNTIME のコードを実行しているあいだだけ。fle_gil.h の 3 行目)。ゲストのコードは複数の Worker で並行に動きうる。したがって、音声処理を別スレッドに出して UI を軽くする効果は GIL では否定されない(効果は未測定)。是正の記録は `docs/failures/0031-gil-semantics-asserted-unread.md`。
- 出典(main の 4 点): T は `evidence/R4-5c.md` §4(rt 44.710 ms、co 20.825 ms と 19.010 ms。比は 2.15 と 2.35)。Export の sha は同 §5(rt の 4 回は 4 種)。起動は同 §2(rt の中央値 7,599 ms、n=5)。CPU の 2.09 倍は `evidence/R4-5e.md` §2。
