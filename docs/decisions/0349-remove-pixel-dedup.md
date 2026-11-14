# 0349: 画素の重複排除(窓の影 `sh_*`・`sh_trim`)の除去実験
分類: (a 除去)。除去するのは `lift/runtime/fle_desk.c` の窓の影(`sh_*`)と `sh_trim` だけ。`fle_r_changed` は今回除かない(別の依頼)。
撤回条件: 除去を main に入れた後、本書の「測定」の条件 2 と同じ手順で underrun quanta(0〜300 s)の中央値が入れる前より多い側に分離した、または P2 の frame/s が減ったなら、除去を戻す。

日付: 2026-11-15   ROADMAP: R3-1   一覧: F-2(除去実験)   コミット: 前 `4ba7ce1`(main が `5328714` まで進んだ後も、bundle `fl-Oz` の HEAD はこれ)・後 `abbb315`(ブランチ `r3-1-remove-dedup` のみ。main には入っていない)

## 決定
決定: 測定は本書の定義どおりに行う。判断は下の「判断の規則」だけで決め、測定前に数字を見て決めない。根拠: 測定の前に規則を固定し、後から基準を動かさないため。測定は次の依頼で行う(本書の時点では測っていない)。

## 比較の定義(測定の前に固定)
- 前 = `~/.cache/signature-studio/work-r25/bundle/fl-Oz`(HEAD `4ba7ce1`。manifest の wasm sha256 の頭 `921bb7e2`)。
- 後 = `~/.cache/signature-studio/work-r31/bundle/fl-Oz`(HEAD `abbb315`。wasm の頭 `5d9c82dd`)。glue・seed・dll の頭は前と一致(`evidence/R3-1b.md`)。
- 同じ窓で**交互・直列**に測る: 前・後・前・後 … 各 n=5(計 10 回)。
- 条件 = 条件 2(マウス移動。`tools/verify-kit/tools/scn-p3-mixmove-300.json`、5 分再生)。条件 1(放置)は今回測らない(測っていないと書く)。理由: 条件 1 の underrun は旧記録で 2〜4 quanta と小さく、差が出る余地が小さいため。影の比較が効く再描画(マウス移動での Mixer の描画)を含むのは条件 2 のため。
- 道具: `tools/verify-kit/serve-once-p3-bundle.mjs <bundleDir の絶対パス> <label>`。出力は `tools/verify-kit/out/p3-mixmove-<label>-chrome-<N>`。計算は `tools/verify-kit/tools/p3-recompute-dirs.py <run...>`(`p3-recompute.py` と同じ式)。
- 各回の前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10` を通し、直前に `/proc/loadavg`・`/proc/pressure/io`・`vmstat 1 3` を読む。
- 数えない回: 開始前の swap の入出力が 0 でない、1 分 loadavg が 8 を超える、I/O avg10 が 0 でない。数えない回は参考として残す。同じ条件で撮り直す。撮り直しは 1 つの位置につき最大 2 回まで。それでも静かにならなければ、その旨を記録して次へ進み、数えない回として残す。

## 指標
- 主指標: underrun quanta(0〜300 s)の中央値。
- 副指標(判定に使わない): 書き込み率の最小・0.95 未満の窓数・presents/s・lagCuts・engine gap frames・Worker CPU。
- BitBlt CPU: `report.json` に項目が無い(`profs[0].cpuS` は worker・main・compositor・gpu・other・top のみ)。取れない。未測定と書く。新しい診断を製品に足さない。

## 語の定義(測定の前に固定)
- 「範囲が重ならない向きで後が少ない」= 後の n=5 の最大 < 前の n=5 の最小(underrun quanta)。このときは「後の underrun が少ない側に分離した」と事実だけ書く。それ以外は「範囲が重なる(または後が多い)」と事実だけ書く。
- 有意性は測らない(n=5)。報告には必ず「n=5。有意性は測っていない」と併記する。

## 判断の規則(測定の前に固定)
- ROADMAP の「P3 が戻るなら外す」は次のときだけ「外す」と判断する: 後の underrun が少ない側に分離し、かつ後の mirror の一致の検査(`cargo test` の `desk_scene`)が通っている。このときは「外す(main に入れる案を別の依頼で作る)」。
- 範囲が重なる、または後が多いときは「残す(main に入れない。ブランチは捨てず記録する)」。
- 「外す」を選んでも、その時点で P3 が戻ったとは書かない。測ったのは 1 条件・n=5・この機械だけ、と併記する。

## 読み(解釈。実行していない)
- 除去後は、同じ画素の矩形も報告される(`evidence/R3-1a.md` の補い (4)。コードの読み)。
- 影の比較が P3 に与える向きは未確認。旧の測定では、影比較を外した測定専用のビルドで、書き込み率・Worker CPU の差は雑音の内と読まれている(参考。下の出典)。

## 参考(旧の別の測定。判定に使わない)
- `docs/archive/decisions/0263-ui-thread-cpu-where-it-goes.md`(§8 の影比較を外した測定専用のビルド。書き込み率・Worker CPU の差は雑音の内、と書かれている)。
- `docs/archive/decisions/0322-lift-fidelity-audit.md`(3-1 の `fle_desk.c:486-575` を分類 (c) とした記述、置き換え案)。
- 旧の R4-3 の条件 2(n=3)の underrun 566〜909 は、この比較の基準ではない(別の測定。`evidence/R4-3.md`)。

## 未確認
- 除去が P3 に与える向き。
- `fle_desk.c` の除去による未使用の警告の有無(`evidence/R3-1a.md` §5)。
- 後(`abbb315`)の bundle の実測(まだ測っていない)。

## 出典
- `docs/ROADMAP.md` R3-1 の行(「P3 が戻るなら外す」・「依頼者に尋ねない」)。
- `evidence/R3-1a.md`・`evidence/R3-1b.md`・`evidence/R4-3.md`(条件 2 の手順・指標・§6 の不具合)。
- `docs/decisions/0364-remove-unreachable-dpi-branch.md`(雛形)。
