# 0365: R2-7 の「基準 +100 ms」の定義。取り込み前の bundle(base)と同じ窓で比べた中央値の差とする
分類: 整備(測定の方法の決定。lift の処理ではないので (a)(b)(c) の対象外。0351 と同じ区分)
撤回条件: 同じ bundle の中央値の窓間の揺れが +100 ms 未満に収まると、別の測定(各 n=10 以上、3 窓以上)で示されたとき、E13 を基準に戻して比べ直してよい。
日付: 2026-11-14   ROADMAP: R2-7   一覧: なし(測定の定義)   コミット: 本文書の commit(`git log --oneline -n 1 -- docs/decisions/0365-r2-7-baseline-definition.md`)
番号: 最大は 0364(`ls docs/decisions`)。0365 が次。

## 決定
- 比べる値: 同じ測定窓で、交互・直列に測った `base`(取り込み前)と `work-r25`(取り込み後。`evidence/R2-7-prep.md` の (b) の表で、`4ba7ce1` を含む bundle)の `timelineIdle` の中央値(各 n=10)。差 = work-r25 − base。E13 は同じ窓に入れて差を並べるが、判定には使わない(参考)。
- 順序: 1 周 = base → work-r25 → E13(参考)を 10 周。各回は 1 つのシェル呼び出しの中で、直前の `cat /proc/loadavg` と `head -1 /proc/pressure/io` を読んでから `node tools/boot-check/boot-bundle.mjs <bundle の絶対パス>` を前面・直列に実行する。
- 静かさ: R2-7e と同じ。直前の 1 分 loadavg > 8、または I/O pressure の avg10 ≠ 0 の回は数えず撮り直す。
- 語: 差 > +100 ms なら「超過の事実を記録」。差 ≤ +100 ms なら「+100 ms 以下」と事実だけ書き、窓ごとの範囲(最小・最大)の重なりと、有意性は測っていないことを併記する。「効かない」「問題ない」とは書かない。
- 適用先: ROADMAP の R2-7 の行、0353・0354・0355 の「基準 +100 ms」は、この定義で読む。文の書き換えは R2-7 の測定の後、reviewer の受理の後の別の依頼で行う(この決定では変えない)。
- 開示: この決定の前に、`evidence/R2-7b.md` の値(work-r25 − base = 26.78 ms、work-r25 − E13 = 141.70 ms、base − E13 = 114.92 ms。各 n=5)は main が見ていた。決定は、基準の選び方(窓間の揺れ)に基づき、その値の向きに合わせたものではない。
決定: 「基準 +100 ms」は、同じ窓で測った work-r25 − base の中央値の差(各 n=10)で判定する。根拠: E13 の中央値は窓ごとに 360.70 ms 揺れ、窓をまたぐ絶対値の比較は +100 ms の判定に使えないため。

## 実測(日付・n・取り方)
- 出典と取り方: `git show HEAD:evidence/<file>` で読んだ(`R2-7b.md`・`R2-7c.md` は未コミットの変更があるため)。計算は `node -e` で、2026-11-14 に実行した。
- E13 の中央値(`timelineIdle`、ms。記録の値の引き写し。再計算はしていない): R0-2 4166.3(n=5。2026-11-13。`evidence/R0-2.md:5`)/ R2-7a 4154.755(n=5。2026-11-14 06:45:57〜06:49:51Z。`evidence/R2-7a.md:2`)/ R2-7b 4111.56005859375(n=5。14:03:52〜14:07:45Z。`evidence/R2-7b.md:2`)/ R2-7e 4472.262451171875(n=10。14:34:50Z 始まり。`evidence/R2-7e.md` §5)。
- 同じ窓の値(記録の値の引き写し): R2-7b の base 4226.47998046875 ms、work-r25 4253.260009765625 ms(各 n=5。`evidence/R2-7b.md:2`)。R2-7e の base 4524.247314453125 ms(n=10。`evidence/R2-7e.md` §5)。
- 計算(式は自分で書いた):
  - (i) 4472.262451171875 − 4111.56005859375 = 360.702392578125 ms(E13 の 4 つの中央値の最大と最小の差)
  - R2-7b: base − E13 = 4226.47998046875 − 4111.56005859375 = 114.919921875 ms。work-r25 − base = 4253.260009765625 − 4226.47998046875 = 26.780029296875 ms。work-r25 − E13 = 141.699951171875 ms
  - R2-7e: base − E13 = 4524.247314453125 − 4472.262451171875 = 51.98486328125 ms
  - 同じ組(base − E13)の 2 窓の差: 114.919921875 − 51.98486328125 = 62.93505859375 ms
- 窓の直前の 1 分 loadavg(E13 の回): R0-2 0.12〜1.13(I/O avg10 は 0.00〜0.03。生の出力は保存していない。`evidence/R0-2.md:11`)/ R2-7a 0.11〜1.75(R2-7a の表に I/O の列は無い。静かさの条件を全 15 回が満たしたと書かれている。`evidence/R2-7a.md:29`・`:35`〜`:51`)/ R2-7b 0.05〜1.72(I/O は全回 0.00。`evidence/R2-7b.md` の表)。R2-7e は全 40 回で静かさの条件を満たし、1 分 loadavg の最大は 1.99(`evidence/R2-7e.md:104`)。

## 読み(解釈。実行していない)
- E13 と base は同じ bundle ではない。seed pack の件数が 225 対 228、wasm が 143,087,706 B 対 143,087,622 B(84 B の差)(`evidence/R2-7c.md:23`・`:117`)。よって E13 との差は R2 の取り込みだけの差ではない。
- base と work-r25 も同じ bundle ではない。wasm の sha の頭が 0286bdf1…(base)と 921bb7e2…(work-r25)、glue の sha が 07e7646f…(base)と b063ddc0…(work-r25)。glue の大きさは 744,946 B で同じ。seed pack は両方 7cd2eaecbdba…(manifest の読み)。
- base は R0-5 の出力と読む。base の manifest の `created` は 2026-11-13T16:35:11.303Z で、R0-5 の 9 段の完了時刻(`evidence/R0-5.md:3` の 16:35:11Z)と一致する。base の manifest の description の HEAD は 818b5b0。`docs/decisions/0351-engine-bundle-workflow.md` の未確認の節は、元の木を HEAD d0447902 と書く。両者の食い違いの原因は未確認。
- ROADMAP の「基準(R0-3)」は、`evidence/R0-3.md:7` によれば R0-2 の E13 の中央値 4166.3 ms。0353・0354・0355 の「基準 +100 ms」の基準は、この読みだけでは一つに定まらない。本文書の定義で読む。
- 旧記録 `docs/archive/progress.md:27` の「前の bundle 比 +100 ms 以内」は、旧の別の定義として引用する。判定には使わない。

## 未確認
- 窓の間の揺れの理由。測定の時刻(R2-7a は 06:45Z、R2-7b は 14:03Z、R2-7e は 14:34Z、R0-2 は 2026-11-13・時刻は未確認)と直前の負荷は違う。どれが効いているかは未測定。
- R2-7a・R2-7b・R2-7e のプロファイルの扱い(冷間か、どの版を使ったか)は、読んだ範囲では確かめていない。
- n=10 でも有意性は測っていない。
- R2-7e の組み合わせ A・B(seed を入れ替えた測定用の組)の値は、この決定では使わない。
- 窓間の揺れが +100 ms 未満かどうかは、この記録では判定しない。

## 出典
`docs/decisions/README.md`、`docs/decisions/0351-engine-bundle-workflow.md`、`docs/decisions/0353-l01-l04-and-sweep1-stubs.md`(3 行目)、`docs/decisions/0354-callback-stack-limit.md`(4 行目)、`docs/decisions/0355-desk-describe-lazy.md`(4 行目)、`docs/ROADMAP.md`(R0-2・R0-3・R2-5・R2-7 の行)、`docs/archive/progress.md:27`、`evidence/R0-2.md`・`evidence/R0-3.md`・`evidence/R0-5.md`・`evidence/R2-7a.md`・`evidence/R2-7b.md`・`evidence/R2-7c.md`・`evidence/R2-7e.md`(`git show HEAD:` で読んだ)、`~/.cache/signature-studio/work/bundle/fl-Oz/manifest.json`(base。読み)。
