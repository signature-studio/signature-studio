# 0406: 実スレッド版と協調版を、基点の木で測った事実(R4-5c。第 3 回の測定)

分類: b(CreateThread と同期の契約を実スレッドで実装する)+ a(協調用の旗と診断の除去)。0393・0397 と同じ。この文書は採否を判定しない。

撤回条件(0393 の 3 行目と同文。引用):

> 撤回条件: 段 3 で、現 HEAD の上の実スレッド版が、同じ HEAD の協調版に対して、P3 の underrun の合計(条件 1・2・3 の各 n=3)が協調版以上で、かつ P2 の T の p50 が協調版より大きいとき。または、段 4 で deadlock の 3 回の再現を直せないとき。撤回は、この文書の末尾に追記する(本文は書き換えない)。

- この文書で測ったのは、上の条件のうち P3 の underrun と P2 の T の p50 だけ。deadlock(引用の後半)は未着手。
- 引用の『現 HEAD』は 0393 の時点の HEAD。測定の基点は `3bbbdb25` の木(§3)。
- 撤回の判断は reviewer の検査と main の決定。§8 に事実を書いた。

日付: 2026-11-16〜17。ROADMAP: R4-5a(要点を追記。状態は doing)。記録: `evidence/R4-5c.md`(表)、`evidence/R4-5c/`(各 run の report.json・lat・Export の json)、`evidence/R4-5c-p3-runs.tsv`、`evidence/R4-5c-recompute.txt`(P3 の再起動前)、`evidence/R4-5c-recompute-post.txt`(P3 の再起動後)、`evidence/R4-5c/p2/t-recompute.txt`(T)。

## 1. 検索した範囲と件数(設計の前の確認)

- 旧決定・記録: 0393・0397・`evidence/R4-5a.md`・`evidence/R4-5b.md` を全文で読んだ(4 件)。0234・0315f などは R4-5a の読みを使い、再読していない。
- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md`(`grep -c '^## seq'` = 263 発言)。語(スレッド・worker・ワーカー・音切れ・ミキサー・mixer・マウス・途切れ・並列・thread。大文字小文字を区別しない)の一致は 36 行(Cloudflare の Workers を含む)。発言単位の 13 件・9 件は `evidence/R4-5a.md` §7 のとおり。本測定で新しく読んだ発言は無い。
- 実装の確認(読み): `lift/runtime/fle_thread.c` の `wk_start`(1138 行。1142 行で `fle_sys_clock_real_enable`。呼び出しは 1267 行で `if (RT_ON)` の中)。`fle_sys.c` の `fle_sys_clock_real_enable`(60 行)。rt1 は `nworkers_cfg = 1` で `RT_ON` が 0 になるため、`wk_start` は呼ばれず、時計は仮想のまま。
- P3 の条件 3(「Mixer を表示したまま」)について: 条件 1 でも Mixer は既定で表示されている(`s3a-mode.png` を目視。`evidence/R4-5c.md` §3)。そこで、条件 3 は「F9 で Mixer を前面に出し、マウスは動かさない」とした。最初に作った「F9 で隠し、マウスを動かす」は、F9 が隠さず前面に出すため、条件 4(前面 + 移動)として残した。

## 2. 診断コードの除去(実験のブランチ `r4-5-rt3` の commit)

- hash の注: 記録の `2c3c569` は現在のリポジトリに無い(object が見つからない)。同じ件名の commit が `03238820` として、同じ変更(8 ファイル・+4/−71。`git show --stat 03238820`)で存在する。`03238820` の親は `56dbdebe`(R4-5b の最終の記録 `5bc51d2` の現在の hash)。hash が変わった理由は未確認。bundle の manifest の HEAD は `2c3c569`。
- rt の commit の鎖(`git log --reverse 3bbbdb25..03238820`): `a5c06100`(R4-5b。0312 の積み重ねを現 HEAD に機械的に当てた。当たる 70 ファイル)→ `427ad258`(R4-5b。失敗の 9 ファイル(17 hunk)を手で統合)→ `0862035d`(R4-5b。旗の除去)→ `56dbdebe`(R4-5b。FLE_DBG_REPORT_S の getenv の除去)→ `03238820`(R4-5c。診断の除去)。

除いた(分類 a): `fle_dbg_note` の呼び出しと定義、フライトレコーダ(`fle_dbg_ring`・`fle_dbg_ri`・`fle_where`)、`DBG1`(定義と呼び出し)、スライスの JS スタック診断、`switch_to`・`schedule` の `fle_dbg`、「main started」「starts W」の `fle_dbg`、GIL の競合の計数(`fle_gil_contended`・`fle_gil_spins`)と報告の 2 項目、3 秒待ちの診断、`fle_thread_wasm_idle_wait` の診断、futex の wrap(`gen_wrap.py` の `D`・`dbgn`・`__wrap_emscripten_futex_wait` と rsp の 1 行)、`ExitProcess` の weak 診断。

残した(除いていない):
- `fle_env.c` の `fle_stop` の 1 行と `harness/trace.c` の STOP の 1 行。pthread の Worker が停止の理由を、その Worker のコンソールに出す受け皿で、停止の報告の内容に関わる。この 2 件のために `fle_dbg`(`fle_thread_wasm.c`)と宣言を残した。`fle_dbg` の注釈に「spike diagnostic」の語が残っている(未修正)。
- 不変条件の監査の assert(`fle_gil_assert`・`fle_gil_assert_fail`・`fle_gil_release_slow` の stop by name)。ログではなく、停止の条件なので残した。
- `fle_sys_cs_owner`(`lift/runtime/fle_sys.c` 376 行)。注釈に spike diagnostic。呼び出しは 0 件(`git grep fle_sys_cs_owner 03238820`)。`a5c06100`(0312 の積み重ね)に由来する。上の『除いた』の一覧に無い。除いていない理由: 除くと bundle の中身が変わり、この測定の rt の bundle と別の物になるため。除く場合は、単独の commit と bundle の作り直しと再測定が要る。

条件付きコンパイルの追加行(`git diff 3bbbdb25 03238820 -- lift app` の追加行を数えた):
- 開く行は計 53: `#if` 1・`#ifdef` 39・`#ifndef` 13。うち 50 行は `__wasm__`・`__EMSCRIPTEN__` の条件、3 行は `#ifndef`(include guard 2: `FLE_CTX_MORE_H`・`FLE_GIL_H`、既定値 1: `FLE_DISPATCH_CACHE_BITS`)。
- 閉じる行(`#else` 16・`#endif` 52)を含めた条件付きの行は計 121。
- これらは実験のブランチの中の行で、main には入っていない。main に入れる場合は AGENTS §6 の『新しい `#ifdef` を作らない』に従う分類が要る。本文書では分類していない。

検査:
- 変更した C の 6 本を `emcc -fsyntax-only -msimd128 -matomics -mbulk-memory`: 0 error。
- `gen_wrap.py` の生成物(2 件)を `emcc -fsyntax-only`: 0 error。
- cargo(`CARGO_TARGET_DIR=~/.cache/signature-studio/cargo-rt`): fe-pe・fe-decode・fe-wasmgen 296 passed・0 failed・1 ignored、fe-lift 2145 passed・0 failed・0 ignored。R4-5b の最終(296/0/1 と 2145/0/0)と同じ。
- 03238820 の commit の範囲では app/ は変更していない(その 8 ファイルは lift/ だけ)。ただし rt の鎖の全体(3bbbdb25 から 03238820)では app/ の 8 ファイルが違う(§3)。npm は再実行していない。

## 3. 測定の対象(bundle)

| 名前 | 木(commit。manifest の HEAD) | engine wasm(B) | sha256 | 作成(UTC)・rc |
|---|---|---|---|---|
| rt(実スレッド版) | `r4-5-rt3`(manifest の HEAD `2c3c569`。現在の先頭 `03238820`。§2) | 158,265,892 | `3a3bb44d51e94f6fc254b6d04a482ca24a5429ddfcdb9f622442f632d2569a5f` | 08:50:16〜09:18:51・0 |
| co(協調版。基点) | `r4-5-co`(manifest の HEAD `b4dc730`。8 桁で `b4dc7305`。木は `3bbbdb25` と同一 `9dba9aa6`) | 158,494,081 | `e5fecd8d93d5f92c806d5d055362459cbd143e1d0de8c98254744400c0e83602` | 09:19:07〜09:47:17・0 |
| rt1(対照。Worker 1 に固定) | `r4-5-rt1`(manifest の HEAD `1f9c22b`。現在の commit `1f9c22b3`。`03238820` に `lift/runtime/fle_thread.c` の変更を 1 つ足した木。記録の `5048d69` は現在のリポジトリに無い) | 158,262,917 | `a20ef39183ecf442a2e9df09044d4ce6cd4fefd67166b2af38bb9c371d33b2cb` | 09:57:29〜10:25:37・0 |

- 基点(`git` で確かめた事実): co の木は `3bbbdb25` の木(`9dba9aa6`)。`b4dc7305` は同じ木を持つ commit で、main の祖先ではない。`3bbbdb25` は main の祖先。rt は `3bbbdb25` の上に R4-5b の 4 commit と `03238820` を積んだもの(§2)。測定の基点は `3bbbdb25` の木。
- main との関係: `origin/main` の `30f05821`(この文書の時点)と `d557c422` は lift/・app/ が 16 ファイル違う(`e1f0e8ec` では 15 ファイル)。`3bbbdb25` と `d557c422` は 57 ファイル違う(`git diff --stat 3bbbdb25 d557c422 -- lift app`)。0393 の『現 HEAD』は、この測定の基点とは違う。
- rt と co は lift/ のほかに app/ の 8 ファイルも違う(`git diff --stat 3bbbdb25 03238820 -- app`: 8 files、+149/−61)。内容は `77ab7773^:app/scripts/build-engine-bundle.sh`、`app/src/audio/`(`fle-out.worklet.js`・`page-audio.ts`・`worker-hook.ts`)、`app/src/engine/fruity.ts`、`app/src/main.ts`(+2 行)、`app/test/unit/` の 2 件。`a5c06100` と `0862035d` が入れた(`03238820` は app/ に触れていない)。P3 の差が app/ の差に由来しうるかは未確認(lift/ と app/ を分けて測っていない)。
- rt1 の木(`1f9c22b3`)は `03238820` と `lift/runtime/fle_thread.c` だけが違う(`git diff --stat 03238820 1f9c22b3`: 1 件、+2/−1)。その 1 件の変更は、`nworkers_cfg` の式を `1u` に替え、`(void)n;` を足したもの(`git show 1f9c22b3 -- lift/runtime/fle_thread.c`)。測定だけに使い、製品に残さない。
- co の大きさ 158,494,081 B は、R5-5 が記録した取り込み後の bundle と同じ(読み。sha256 は R5-5 に無い)。
- build-lock は 3 回とも 1 回目で取得(待ち 0 秒)。最小 MemAvailable: rt 12,505 MB、co 11,881 MB、rt1 13,680 MB。工程の時間は `logs/stages.log`(wasm-Oz-units 618〜636 s、wasmopt 193〜205 s)。
- 各木の `npm run build` は rc 0。起動の確認では全 bundle で `crossOriginIsolated` = true、`SharedArrayBuffer` = true。

## 4. 起動(P1)と Problems 窓

- 1 分 load ≤ 6 で数えた。`timelineIdle`(ms)。中央値と範囲は `evidence/R4-5c.md` §2。
  - rt: 中央値 7,599(7,436〜7,832。n=5)。
  - co: 中央値 4,851(4,764〜5,020。n=5)と 5,238(5,169〜6,217。n=5。rt1 と交互)。
  - rt1: 中央値 5,599(5,483〜5,799。n=5。co と交互)。
- rt1 には起動の束が 2 つある。表に入れたのは 2 つ目(co と交互)。最初の束の 3 件(5,618・6,139・5,648 ms。うち 6,139 ms は state `running`)は、後の再試行でファイルが上書きされ、記録が一部失われた。
- 起動の確認: 全 run で `engineKind` = `emscripten-fruity-v1`、`error` = null、`vfs seed: list /data/seed/vfs-seed-idle1.tsv, 245 files`。`state` は `engine-idle`(上の最初の束の 1 件を除く)。その 1 件の state が `running` なのは、`first_pixel` の印が `idle` の後に来ると、app が state を `running` にするため(`app/src/main.ts` の 569 行。rt(03238820)と rt1(1f9c22b3。app/ は 03238820 と同じ)の木の行。co と main(3bbbdb25。`app/src/main.ts` は両者で同一)では 567 行)。
- ページの Worker の数(`Target.getTargets` の type `worker`): rt 8、co 1、rt1 1。
- Problems 窓(目視): rt 6 plugin(s)、co 6 plugin(s)。同じ 6 件。
- 生の起動 JSON は `/tmp` の消失で失われた(値は実行出力の転記)。

## 5. P3(5 分再生。Worker の CPU を含む)

- 条件: 1 = 放置(`scn-s3a-song.json`)、2 = Mixer 表示 + マウス移動(`scn-p3-mixmove-300.json`)、3 = F9 で前面 + 移動なし、4 = F9 で前面 + マウス移動。3 と 4 は、1 と 2 の台本の最初の `audioSnap w0` の後に `key F9`・`wait 2000`・`shot` を挿入した(台本は各木の `tools/` と同じ内容)。
- 指標: `p3-recompute-dirs.py`(既存の検証用の道具。0〜300 s の窓。underrun の quanta の合計)。再起動前の表: `evidence/R4-5c-recompute.txt`。再起動後の 10 run: `evidence/R4-5c-recompute-post.txt`(入力は各木の `tools/verify-kit/out/<run>/audio-p3.json` と `report.json`。`audio-p3.json` は evidence に保存していない)。全 run の表: `evidence/R4-5c-p3-runs.tsv`。

| 条件 | rt | co | rt1 |
|---|---|---|---|
| 1 放置 | 0 / 0 / 0 | 2 / 119 / 0 | — |
| 2 Mixer + 移動(再起動前) | 0 / 0 / 0 | 1837 / 1745 / 1832(追加 1113 / 862) | 1458 / 1266 |
| 2 Mixer + 移動(再起動後) | 0 / 0 / 0 | 0(完了の 1 回)/ 6 / 6 / 4 | 12(完了の 1 回)/ 19 / 52 |
| 3 F9 前面・移動なし | 0 / 0 / 0 | 2 / 2 / 262 | — |
| 4 F9 前面 + 移動 | 0 / 0 / 0 | 1497 / 1643 / 1501 | — |

- 合計(再起動前の条件 1〜4、各 3 回): rt 0、co 10,442(条件 2 の追加 2 回を除く)。
- 異常(数えたまま記録): co の条件 3 の 3 回目は最初の書き込みが −50.8 s、presents/s 9.7(他の co は 15〜16)。rt の条件 3 の 3 回目は underrun 0 だが、書き込み率の最小 0.446 で 0.95 未満の窓が 15。原因は未確認。
- 再起動の前後で数字が大きく変わった。co の条件 2 は 1,745〜1,837 から 4〜6 へ。rt は前後とも 0。原因は未確認。各 run の開始時の 1 分 load(`report.json` の `loadStart`)は記録しているが、他の作業の負荷の内訳は記録していない。
- Worker の CPU(`report.json` の `profs[0].cpuS.workers`。Worker ごとの秒。値は JSON の転記):
  - rt 再起動前(条件 2 の 3 回。合計 572.9〜576.6 s): 例 `p3-rtc2r1-chrome-1`(`evidence/R4-5c/p3/p3-rtc2r1-chrome-1.report.json` の 31〜40 行): 210.41・133.86・49.12・44.06・36.22・34.40・34.31・34.21(合計 576.59)。
  - rt 再起動後(条件 2 の 3 回。合計 477.6〜480.1 s): 例 `p3-mixmove-rtc2post1-chrome-1`: 197.49・103.71・37.86・33.19・27.44・26.07・26.04・25.83(合計 477.63)。
  - co(全 18 run。1 本): 252.5〜275.0 s(最小 `p3-coc1r2-chrome-1` 252.53、最大 `p3-coc4r2-chrome-1` 275.00)。
  - rt1(全 5 run。1 本): 263.4〜276.3 s(最小 `p3-mixmove-rt1c2post2-chrome-1` 263.35、最大 `p3-rt1c2r1-chrome-1` 276.3)。
- 仮説(未確認): rt で underrun が 0 になるのは、音の処理が UI の処理と 1 つの CPU を奪い合わないためか。Worker と音の処理の対応を記録していないため、確かめていない。rt と co は app/ の 8 ファイルも違う(§3)。その差が P3 に影響した可能性は未確認。
- 耳の確認: 0 件。

## 6. P2(入力遅延 T と frame/s。R4-2c の腕 A。既定の起動フラグ)

- T の生データ: 各木の `tools/verify-kit/out/r42c-*/` にあった(git の管理外)。20 run の `lat-a-idle-menu.json`・`lat-c-mixer-drag.json`・`lat-d-space.json`・`lat-b-play-menu.json` を `evidence/R4-5c/p2/lat/<run>/` に保存した。c と d は前回の保存に無く、今回足した(`lat-ab.py` の `load` は 4 ファイルの seq で閾値を決めるため必要)。
- T の再計算: `tools/verify-kit/tools/r45c-lat-recompute.mjs`(Node。`lat-ab.py` の `load`・`pct` を移植)。出力は `evidence/R4-5c/p2/t-recompute.txt`(20 run の n・p50・p90・p99 と、群ごとの中央値)。前回の検査に使った Python の `tcheck.py` は、この Node の道具に置き換えた(Python の使用の制限は AGENTS の規則と決定 0415(依頼者の恒久指示 2026-11-17)による)。
- 独立の再計算: `tools/verify-kit/tools/r45c-lat-check-py.py`(Python。検証用で、製品・ci・lift の経路には使わない。`lat-ab.py` の定義を読んで書いた別の実装)。20 run × 4 行 = 80 行を Node の出力と比べ、80 行すべて一致(`evidence/R4-5c/p2/t-recompute-check.txt` の最後の行)。
- counted 条件: `lat-ab.py` は loadStart ≤ 8 と ioStart = 0 と、他の Chrome との重なり(`chrome-count.tsv`)で数える。Node の道具と Python の確認が扱うのは load と I/O だけ。20 run とも loadStart ≤ 8(最大 5.77)かつ ioStart = 0(report.json の値)。重なりは `chrome-count.tsv` を取っていないので確かめていない(未確認)。
- 120 の条件は作っていない(この機械の rAF は 60 Hz。`engineHz` 60、`rafHz` 60)。120 fps は未確認。
- 中央値(範囲。5 回ずつ)。presents/s は `measures[0].presentsPerS`(出典 `evidence/R4-5c/p2/r42c-*.report.json`)、T は `t-recompute.txt`:
  - presents/s: rt 48.12(47.63〜49.83)、co 束 1 48.37(39.74〜51.41)、co 束 2 47.87(47.00〜49.63)、rt1 48.33(45.40〜48.63)。
  - 再生中の File click の T の p50(ms): rt **44.710**(42.945〜48.915)、co 束 1 20.825(19.140〜23.280)、co 束 2 19.010(17.955〜20.695)、rt1 **20.600**(20.050〜22.890)。
  - 再生中の Esc の T の p50(ms): rt **23.055**(20.880〜23.455)、co 束 1 5.665(5.490〜5.915)、co 束 2 5.040(4.865〜6.100)、rt1 5.275(4.820〜5.960)。
  - 停止中の File click の T の p50(ms): rt 7.400、co 束 1 6.780、co 束 2 6.775、rt1 6.840。
- 束の対応: 束 1 = `r42c-cop2r*-chrome-1`(更新時刻 2026-11-16 22:25〜23:05。機械の時刻)、束 2 = `r42c-cop2r*-chrome-2`(2026-11-17 00:02〜00:43)。rt1 は `r42c-rt1p2r*-chrome-1`(束 2 と交互に測った)。対応は名前と更新時刻による。presents/s の中央値では決めていない。
- 読み: 再生中の File の中央値は、rt が co の約 2.1〜2.4 倍、Esc は約 4.1〜4.6 倍。rt1(Worker 1 つ。GIL と TLS の経路だけ)は co と同じ範囲。

## 7. Export(sha256。R5-4 の手順)

- 期待値の出典: R5-4 の 381cdb5a…f979 は取り込み前の協調版の値で、どの bundle とも一致しない。co の値 94c93666…4090 は R5-5 の取り込み後の値と一致した(co 4/4)。
- rt: 4 回とも値が違う(`e20aa98a…57d5`・`92d211ef…49cb4e`・`adbf3a58…6fa34`・`513876a3…47f9`)。rt1: 1 回だけ違う(`67eb4a9e…8ad88`)。
- WAV の比較(`wavcmp.py`): rt の 1 回目と 2 回目は 5,715,911 フレーム中 27 フレームが違う(最大 |差| 1 LSB。最初の違い 10.364 s)。co の 1 回目と rt の 1 回目は 18 フレームが違う(最大 1 LSB。12.219 s)。
- 分類: **未確認**。候補は 2 つ。a(我々の処理。仮想時計を実時計に替えた箇所)と b(スレッド間の同期の契約)。rt1 は仮想時計のまま 1 回違ったので、時計だけでは説明できない(読み)。どちらかは確かめていない。近道は足していない。
- Wine との比較は 0 件(未確認)。「Wine の実スレッドでも同じ入力で出力が変わる」は、本測定では確かめていない(未確認)。

## 8. 撤回条件に照らした事実(判定は main)

- (a) P3 の underrun の合計(条件 1・2・3 の各 n=3。再起動前): rt 0、co 5,801(= 121 + 5,414 + 266)。rt < co。
- (b) 再生中 File click の T の p50(n=5 の中央値): rt 44.710 ms、co 20.825 ms(束 1)・19.010 ms(束 2)。rt > co。
- 撤回条件の (a) は『rt の合計が co 以上』。実測は rt 0 < co 5,801 で、この条件は成り立たない。(a) と (b) の両方が成り立つときに撤回条件に当たる(冒頭の引用)。判定は main。
- 参考(採否の判定には使わない): 条件 1〜4 の合計は rt 0、co 10,442(条件 4 を含む)。
- 参考: 再起動後の条件 2(各 3 回。co と rt1 は完了の 1 回を含む。出典 `evidence/R4-5c-recompute-post.txt`): rt 0・0・0、co 0・6・6・4、rt1 12・19・52(合計 83)。
- 範囲: n=3、この機械、headless Chrome、耳の確認 0 件。120 fps は未確認。

## 9. 次の一手(1 件)

- rt の再生中の T が大きい原因の切り分け。候補は、複数 Worker の GIL の競合(rt1 の T は co と同じなので、GIL と TLS の定常費用は候補から外れる。読み)と、Worker 間の入力経路。確かめ方: GIL の待ちの時間を、対照のブランチ(製品に残さない計数)で同じ台本の run に当てて測る。未確認。

## 10. 未了・未確認・ROADMAP

- 未確認(9 件。`evidence/R4-5c.md` §6 と同じ一覧):
  1. 120 fps(この機械の rAF は 60 Hz。120 の条件は作っていない)。
  2. 耳の確認(0 件)。
  3. 依頼者機の値(0 件)。
  4. Wine との比較(0 件)。
  5. Export の不一致の分類(§7。a か b かは確かめていない)。
  6. rt の P3 の underrun が 0 になる原因と、Worker と音の処理の対応(§5)。rt と co は app/ の 8 ファイルも違う(§3)ので、P3 の差が app/ の差に由来しうる。
  7. 再起動の前後で P3 の数字が変わった原因(他の作業の負荷の内訳は未記録。loadStart は記録済み)。
  8. 生データの消失: 起動の JSON と pre-run の記録、rt1 の最初の束の一部(値は転記。§4)。
  9. T の counted 条件のうち、他の Chrome との重なり(`chrome-count.tsv` は取っていない。§6)。
- 未決定の事項: `fle_sys_cs_owner` を除くかどうか(§2)。
- ROADMAP の R4-5a の行: 同じ commit で、要点を追記した(状態は doing のまま)。
- 未実施: 実験のブランチ `r4-5-rt3`・`r4-5-co`・`r4-5-rt1` は push していない。後片付け(ワークツリーと bundle)は未実施。
