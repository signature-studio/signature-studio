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

## 補い(2026-11-15、reviewer の差し戻しへの対応。測定の結果を見る前に書く)

時点の開示: この補いを書いた時点で、R3-1d の測定は始まっていた。この補いを書いた worker は、測定の出力(`evidence/R3-1d.md`・`tools/verify-kit/out/`)を開いていない。main も測定の数字を見ていない。位置のどこまで終わっていたかは未確認。

### 1. 数えない回と n
- 「1 つの位置」= 交互の順序(前・後・前・後 … の計 10 個)の n 番目の枠 1 つ。
- 1 つの位置の試行は最大 3 回(最初の 1 回 + 撮り直し 2 回。上の「数えない回」の撮り直し上限と同じ)。
- 3 回とも数えない基準に当たったら、その位置は「数えない」として残し、次の位置へ進む。
- 数えた回が前・後のどちらかで 5 に満たない場合、「語の定義」と「判断の規則」は適用しない。その場合の判断は「判断しない(n 不足)。ブランチは残す。追加の測定は別の依頼」とする。

### 2. 分離の端
- 「後の最大 < 前の最小」は厳密な不等号。等しい場合は「範囲が重なる」と書く。

### 3. `desk_scene` の条件
- この条件は ROADMAP の R3-1 の行(`docs/ROADMAP.md` 38 行)に無い。`desk_scene` は ROADMAP に 0 件。main が測定の依頼を作ったときの決定。
- 意味: `lift/crates/fe-lift/tests/desk_scene.rs` のテスト関数 `the_scene_description_loses_nothing_and_a_move_or_restack_sends_no_pixel` が、後の commit(`abbb315`)で通っていること。
- 検査の仕組み(事実): `CHECK` は `desk1_compose.rs` の 78 行で定義されたマクロで、失敗を数える(`failed++`)だけで、止まらない。プログラムは最後に `failed != 0` で非ゼロを返す(`desk_scene.rs` 187 行)。テスト関数は、終了が成功であり、出力に `failed 0` があることを要求する(217〜218 行)。1 つでも失敗すればテストは落ちる。
- `apply(sc, &nu) == 0`(鏡と面の一致)の検査は `desk_scene.rs` の 89・101・111・126・131 行の 5 箇所。乱択の 2000 段では 183 行で数え、185 行で `bad_total == 0` を検査する。`abbb315` は、この検査を変えていない(`git show abbb315 -- lift/crates/fe-lift/tests/desk_scene.rs` の差分は 93・96・100 行の 3 か所だけ)。
- `abbb315` の変更: `desk_scene.rs` は 3 行追加・3 行削除(`git show --stat` の `6 +--`)。内訳はコメント 1 行(93 行)と、期待値 2 件。96 行は「同じ画素を書き直す」の期待値を 0 件から「書いた矩形 1 件 (0,0)-(100,50)」へ、100 行は「1 画素の変更」の期待値を (20,10)-(21,11) から (0,0)-(100,50) へ変えた。この 2 件は、除去後のコードの動き(書かれた矩形をそのまま報告。`evidence/R3-1a.md` §2)に合わせて変えたもの。影の機能そのものを検査していた。
- この 2 件の期待値は、除去の効果(P3)の判断の根拠に使わない。
- 通過の出典: `evidence/R3-1a.md` §3 の 43 行(`desk_scene` が `ok`)と、補い (3) の 85 行(元ログの該当行の記述)。保存物 `evidence/R3-1a-cargo.txt` には該当行が無いと補い (3) が書く。この補いでは cargo を実行していない。通過は記録の読みであり、再実行は未確認。
- 参考: `docs/archive/decisions/0263-ui-thread-cpu-where-it-goes.md` の 218 行付近は、影比較を外すと `desk_scene.rs` の「同じ画素の再書き込みは報告しない」検査 2 件が落ちると書く(旧の別の測定の記録の記述。判定に使わない)。

### 4. P2
- この測定は P2 の frame/s・入力遅延を測らない。
- 判断の規則が決めるのは「main に入れる案を別の依頼で作る」ことまで。P3 の分離だけでは main に入れない。
- main に入れる前に、撤回条件が参照する P2 の frame/s の測定(ROADMAP R4-2 の行の基準プロジェクトで、画素が変わった frame/s。前後を同じ窓で交互・各 n=5)を、案の依頼の中で行う。
- その測定の定義(手順・閾値・「入れる前」の測定の範囲)は、測る前に別の決定文書(または本書の補い)に固定する。この補いでは閾値を定めていない(未定義のまま)。

### 5. 判定の行
- 「I/O avg10 が 0 でない」は、各回の直前に読む `/proc/pressure/io` の `some` の行の avg10 で判定する(`full` の行は使わない)。
- `tools/verify-kit/tools/wait-quiet.mjs` が読むのは、`/proc/loadavg` の先頭の値(1 分の load)、`/proc/pressure/io` の最初の `avg10=` の値(1 行目の `some` の行。2 行目の `full` は読まない)、`/proc/vmstat` の `pswpin`・`pswpout` の 2 秒の差。`/proc/meminfo` は表示のみに使う。本書の「swap の入出力が 0 でない」は、この 2 秒の差に当たる(読み)。
- 両方を記録に残す(`wait-quiet.mjs` の出力行と、各回の `/proc/pressure/io` の 1 行目)。

### 6. 参考(旧の別の測定。判定に使わない)
- `docs/archive/decisions/0322-lift-fidelity-audit.md` の 3-1 の行(64 行)は、分類 (c)、「この群は方針上『残す理由が測定で示されている』ので抹消を勧めない」と書き、根拠として `0245` の実測(native の合成 8.87 → 0.016 ms/フレーム、Chrome の Worker CPU 973 → 585 ms/s)を挙げる。
- `0245`(`docs/archive/decisions/0245-skip-identical-blit.md`)には後の訂正がある(124・125 行。PM の訂正、2026-11-05)。973 → 585 は旧 HEAD・高負荷の値で、現 HEAD では定常の Worker CPU が 647 → 593 ms/s(−8%)。native の値と Chrome の値は見ている物が違うので、同じ効果として読まない。
- `0263` は、影比較を外した測定専用のビルド(212 行・218 行)で、書き込み率・Worker CPU の差を雑音の内と読んでいる(218 行)。
- 2 つは別の測定・別の指標。向きの比較はしない。事実の並べだけ。
