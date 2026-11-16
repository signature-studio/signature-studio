# 0393: 実スレッド版を現 HEAD の上に作り直し、P2・P3 を同じ HEAD の協調版と比べる(第 1 段は材料と dry-run。PM の決定)
分類: b(CreateThread と同期の契約を実スレッドで実装する)+ a(協調 scheduler と旗の除去)。AGENTS.md §6 の (a) の除去と (b) の拡充だけに当たる。
撤回条件: 段 3 で、現 HEAD の上の実スレッド版が、同じ HEAD の協調版に対して、P3 の underrun の合計(条件 1・2・3 の各 n=3)が協調版以上で、かつ P2 の T の p50 が協調版より大きいとき。または、段 4 で deadlock の 3 回の再現を直せないとき。撤回は、この文書の末尾に追記する(本文は書き換えない)。
日付: 2026-11-16   ROADMAP: R4-5a   一覧: evidence/R4-5a.md・evidence/R4-5a-dryrun.tsv・evidence/R4-5a-dryrun-repo.tsv   コミット: 本文書の commit(後続)

## 決定
1. 決定: 実スレッド版を作る(PM の決定を採る)。根拠: 協調版の P3 の上限は、窓の外の呼び出しが 1.87%、C の割合の和が 8.81%(evidence/R4-4j.md)。0234 の冒頭は、分割だけでは P3 に足りず、混成の拡大とミキサーの別スレッド化の両方が要ると書く。
2. 決定: 第 1 段は材料・dry-run・計画だけ。bundle は作らない。根拠: 段の順序(PM の指示)。dry-run は文脈の一致だけを見るので、コンパイルとテストの前に bundle を作らない。
3. 決定: 割り当ては 0312 の積み重ね(patches/mt3w/0312/impl/0312-stack-on-main-e08b4589.diff)の形。UI は Worker 0、他のゲストの thread は Worker 1 以降へ空いた順に。数は FLE_WORKERS の値(既定は 0 = 協調)。根拠: 当たる差分がこの形で、0312e の S3 の既定(clamp(n−1, 2, 8))と一致する。1 スレッド 1 Worker(0234i)は段 4 の比較の候補に残す。
4. 決定: 製品に旗を残さない。FLE_GIL・FLE_TLS_MORE・FLE_WORKERS(getenv)・FLE_DBG_REPORT_S・FLE_PROCESSORS・GIL=・SPIKE_CFLAGS は、段 2 で定数にするか除く。根拠: AGENTS §6 と、0234i の旗の除去(docs/archive/decisions-unmerged/0234i-stage-b-design-tables.outgit-newer.md)。
5. 決定: 行の ID は R4-5a。R4-5 は依頼者機の計測で使用中(ROADMAP の R4-5 行)。根拠: 同じ ID の行を二つ作ると、次の一手の機械的な選択が壊れる。
6. 決定: 「2 件の破綻」は、実スレッド版で観測した 1 件(pthread mutex deadlock。RT-B の o1b で 3/3)と数える。Mixer 表示・マウス移動は、本番に対する依頼者の報告で、実スレッド版では未測定と書く。根拠: evidence/R4-5a.md の §2。

## 検索した範囲と件数
- 読んだ旧決定・記録 16 件(0234・0289・0306・0301・0305・0312・0312e・0315f・0263・0234i・0320・2026-11-07 の owner-observations の該当節・R4-3・R4-4i・R4-4j・performance.md)。旧パッチ 19 件(旧ツリー)と repo の 10 件を dry-run した。
- 依頼者の発言: S33 の all.md(20 件の抜粋)で「スレッド」「Worker」「ミキサー」「音切れ」は 0 件。全文(263 発言)で 13 件に当たり、スレッド・Worker の分割・音切れ・マウス移動の条件に関わるのは 9 件(msg_19d3675dc001TY8JunuqBVp4Pu ほか。evidence/R4-5a.md の §7 の 2)。

## 実測の要点(詳細は evidence/R4-5a.md の §3・§4)
- 0312 の積み重ね(80 ファイル)は、HEAD(31ba7df)に対して 53 が当たり、17 が新規、9 が失敗(17 hunk)、1 が不在。失敗の理由は HEAD の後の決定(0372 の行、0319 の include)。
- 0312 の stage C の単独(13 ファイル)は、すべて当たる。
- 追加行の旗: FLE_GIL 46・FLE_TLS_MORE 31・FLE_WORKERS 6。runtime の getenv は 2 件(FLE_WORKERS・FLE_DBG_REPORT_S)。
- HEAD には、共有メモリと atomics の link 旗が既にある(lift/harness/web/fruity/link-web.sh:23、app/scripts/build-engine-bundle.sh:133)。GIL と、Worker を増やす経路は無い。

## 読み(解釈。実行していない)
- 0312 の積み重ねは、協調の scheduler を残し、Worker を増やす経路を opt-in で足す形。協調の経路(save_ctx・仮想時計の大半)は変えていない。
- 実スレッド版の時計は、最初の CreateThread の後で実時計になる(0312 の差分の fle_sys.c の注記)。Export の決定性と P3 の仕組みに効く。
- 0315f の deadlock の原因は未特定。分類(a か b か)は未確認。

## 未確認
- 実スレッド版のコンパイル・テスト・bundle・実エンジンの動作(0 件)。
- Export の決定性。Mixer 表示の条件(実スレッド版も協調版も未測定)。依頼者機の値(0 件)。crossOriginIsolated の実測値(記録に無い)。
- bundle の作成時間(PM の指示は 33〜45 分。ROADMAP の R0-5 は旧 CI の冷間 17〜22 分と書く)。

## 出典
- docs/archive/decisions/0234-guest-threads-on-workers.md(冒頭の三点)、0289-realthread-go-nogo-1.md、0306-gonogo2-realthread-e11.md、0301-rt-on-e11-rebase.md、0305-realthread-go-prep.md、0312-s0-s1-stage-c.md、0312e-s3-host-cpus-and-worker-count.md、0315f-pthread-mutex-and-gil.md、0263-ui-thread-cpu-where-it-goes.md。
- docs/archive/feedback/2026-11-07-owner-observations.md(O19 と o1b の節)。
- 依頼者の発言: msg_1b41d28ed001RdYrCKewGqbEZa(JST 2026-11-07 12:46:13)、msg_19d3675dc001TY8JunuqBVp4Pu(JST 2026-11-03 02:02:33)。

## 追記(2026-11-16)

本文は変えない。根拠の出典(a)と、時計の分類(b)を足す。

### (a) 依頼者の発言の出典

- 検索の範囲と件数: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md`(263 発言。`grep -c '^## seq'` = 263)。語: スレッド・Worker(Cloudflare の Workers を除く)・Mixer・途切れ・音切れ・マウスを動かし・マウス移動(大文字小文字を区別しない)。当たった発言は 13 件。ID の一覧は evidence/R4-5a.md §7 の 2 と一致する。
- 9 件(スレッド・Worker の分割・音切れ・マウス移動の条件に関わる)は、evidence/R4-5a.md §7 の 2 の分類による(再現できるのは 13 件の ID まで)。境界は msg_1b2d107a00017VUCYlmjAI9Tub(JST 2026-11-07 06:47:46。Mixer の語は貼り付けたログの行で、本文にネイティブでの Playlist のマウス掴み動作の記述がある。01a の 1652 行)。9 件か 10 件かは分類による。
- 3 件(01a の全文で ID・日時・本文を照合):
  - msg_1a2ba25fa001KucAOXwpOPz7tj(JST 2026-11-04 03:44:29。01a の 1190 行)「少なくともwasm→worker→メインフレーム→canvas、のようなバケツリレーは避けること」。命令形の語『避けること』を含むが、Worker 間の経路の方針で、スレッドの分割を求める文ではない。
  - msg_1a87f185300136lxaf5p23SThF(JST 2026-11-05 06:37:43。01a の 1565〜1567 行)。マルチスレッドと wasm・worker 分割の現状を尋ねる質問。
  - msg_1b41d28ed001RdYrCKewGqbEZa(JST 2026-11-07 12:46:13。01a の 1768〜1769 行)。「Mixerウインドウが表示されている」「マウスを動かしている」などの条件で再生が破綻する。オーディオ処理と GUI 処理を別 Worker で動かす案は、推測として優先度が高いと書かれている。
- 3 件のうち、スレッドの分割を求める命令形は無い。msg_1a2ba25f は『避けること』という方針、残り 2 件は質問と推測。evidence/R4-5a.md §7 の 2 の『命令形ではない』は、msg_1a2ba25f に当たらない(訂正の注を同じ節に足した)。S33 の all.md(先頭に『件数: 20』の抜粋)には、この 3 件は含まれない。

### (b) 時計

- 実スレッド版(`patches/mt3w/0312/impl/0312-stack-on-main-e08b4589.diff`)の時計は、最初の CreateThread で Worker 1 が起動したとき(差分の 4796 行 `wk_start`。4420 行の注記)から、「その時点の仮想の経過時間＋実時間」になる(4178〜4181 行の注記。`fle_sys_clock_real_enable`)。協調版の時計は仮想時計だけ(`lift/runtime/fle_sys.h` 14〜17 行。決定 0017)。
- Export の sha256 が協調版と違えば、原因を a(我々の処理の欠陥)か b(契約の不足)に分類してから直す。分類が書けない近道は足さない(AGENTS.md §6)。
- 暫定(段 3 の測定の後に決める): 仮想時計そのものは a(我々の代行)に当たる疑いがある。根拠: 仮想時計は FL が読む QueryPerformanceCounter・GetTickCount・timeGetTime の値を我々の計数で決める(`lift/runtime/fle_sys.h` 14〜17 行。読み出しのたびに一定量を進める)。決定 0017 は、実時計を「実行ごとに結果が変わる」として却下した(`docs/archive/decisions/0017-sync-time-memory.md` 55 行)。Windows の時刻 API が実時間に従うという点の出典は未確認(MSDN の本文は未取得。`lift/runtime/fle_sys.c` 106〜110 行の引用のみ)。記録: `docs/archive/decisions/0147-virtual-clock-and-concurrency.md`。未確認: 実時計にしたとき Export の決定性が保てるか。
