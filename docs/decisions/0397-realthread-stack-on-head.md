# 0397: 0312 の積み重ね(S0+S1)を現 HEAD に当て、旗を除いた(R4-5b。第 2 段)

分類: (b) 0312 の積み重ねの CreateThread・同期の契約を実スレッドで実装する + (a) 協調 scheduler 向けの旗と getenv の除去。AGENTS §6 の (a)・(b) だけ。決定 0393 と同じ。
撤回条件: 0393 と同じ(段 3 で P3 の underrun 合計と P2 の T の p50 が協調版に対して悪化したとき。段 4 で deadlock の 3 回の再現を直せないとき)。本段の追加: native テストの失敗が基準(2428・0・1)より増え、分類 b か a の欠陥が直せないとき。
実験: ワークツリー `r4-5-rt3`(push しない)。基点 b4dc730、先端 5bc51d2。差分: `evidence/R4-5b-stack.diff`(79 ファイル、+2871/−279)。記録: `evidence/R4-5b.md`。

## 1. 検索した範囲と件数(設計の前の確認)
- 旧記録: 0393 が読んだ 16 件に加え、本段で読んだのは 9 件(旧ツリーの 0312 の表 3 件の先頭、`tools/apply-stack-0312.sh`、0312e の s3a(repo)と s3b(旧ツリー)、HEAD のテスト 3 件のソース検査と mimic の定義)。0 件の追加ではない。
- 差分: 0312-stack の 80 区間、旗・getenv・#if 系の追加行 140 件(`evidence/R4-5b-flags.tsv`)。
- 依頼者の発言: 0393 の 9 件を引用(範囲: ~/work/handover/evidence/01a-user-messages-full.md、263 発言)。本段で発言単位の再集計はしていない(未確認)。

## 2. 適用(区間ごと)
- 0312-stack の 80 区間: 当たる 70(`patch -p1 -F0`。dry-run の判定は `evidence/R4-5a-dryrun.tsv` と一致)、失敗 9(17 hunk)、不在 1。commit 3cdee87。
- 失敗 9 の統合(commit 86c1f62): (1) `imports_table.rs` 1 hunk: 0234 の 2 行を HEAD の 0372(CoInitializeEx)の後ろへ足した。0267 の後ろに足すと 0372 の行が動くため(0015 の append only)。(2) テスト 7 本(wall_fr4・fr5・fr13・fr15・fr16・fr19・fr20。15 hunk): HEAD の `rows_0372` と同じ数え方に `rows_0234` を足した。(3) `fle_callback.c` 1 hunk: fle_gil.h の include と TLS の depth 別名を足し、HEAD の 0319 の `extern fle_teb_cur` を `__EMSCRIPTEN__` でない時だけにした(fle_ctx_more.h のマクロと衝突するため)。分類は (1) (b) の append、(2) (a) の数え方の追従、(3) (a)+(b)。
- 訂正: 86c1f62 の本文の「前の commit で当たった」は誤り。`wall_fr15.rs` の 3 hunk 目は、統合の手順の patch で当たった(`Hunk #3 succeeded at 502 (offset 1 line)`)。
- 当たった hunk の位置ずれ(文脈は完全一致): fle_win.c +1 行、fle_thread_wasm.c +1・+1・+10 行。fuzz は 0。
- 不在の e2e 用の browser.mjs(旧ツリーにのみ有る。22 行追加・1 行の除去)は置かない。根拠: main の履歴に無い(`git log --all -- app/test/e2e/browser.mjs` の件数 0)。AGENTS §4(CI に自作のブラウザスクリプトを置かない)。旧設計の出典は AGENTS §4。依頼者の発言 ID は本段で特定していない(未確認)。その 22 行は FL_WORKER_CONSOLE・FL_PROFILE の診断だけ。

## 3. 旗の分類(`evidence/R4-5b-flags.tsv` の 140 行。種類: directive 124・env 14・text 2)
- オプション(除去)→ 定数にした 114 行: FLE_GIL・FLE_TLS_MORE の `#ifdef`・`#ifndef`・`#if defined(...)` と、対の `#else`・`#endif` を `__wasm__` に置き換えた。wasm32 の製品では真。native(cargo の clang と mimic テスト)は HEAD の形のまま。分岐は除かず置き換えた(native の振る舞いを変えない)。
- オプション(除去)→ 除いた 5 行: getenv(FLE_WORKERS)を除き、Worker 数は 0312e S3b の式 `n<3 ? 2 : n>9 ? 8 : n-1`(n = hardwareConcurrency。分類 b: GetSystemInfo の値の契約)。getenv(FLE_DBG_REPORT_S)と報告を除く(a)。gen_wrap.py の生成物の囲み(#ifdef FLE_GIL)を除く。注釈の環境変数 FLE_PROCESSORS を S3b の形(navigator.hardwareConcurrency)の記述に直す。
- 残す(旗ではない)8 行: include guard 4(FLE_GIL_H・FLE_CTX_MORE_H)、既定値 2(FLE_DISPATCH_CACHE_BITS)、既存の型 2(`__EMSCRIPTEN__` の trace.c)。
- テスト専用・製品に入らない 9 行: harness の node 駆動 `mt2/node-bundle-run-rt.mjs`(FLE_RT_MAX_S・PARITY_SEED_TSV・PARITY_REPO と K=V の環境)。app・ci・build スクリプトからの参照 0 件。
- 検査表の文字列 2 行(check_i1.py)。環境変数の読みではない。
- build-engine-bundle.sh: GIL=・SPIKE_CFLAGS・`-DFLE_GIL -DFLE_TLS_MORE` を除き、`GIL=1` の 6 分岐は常に通る形にした。`[ -n "$MXTLS" ] || hyb_guard merged …` の 1 行は、MXTLS が常に空でない形では通らないので除いた(a)。

## 4. 検査(実測。bundle は作らない)
| 項目 | 基準 b4dc730 | 最終 5bc51d2 |
|---|---|---|
| cargo test: fe-pe・fe-decode・fe-wasmgen(passed/failed/ignored) | 294/0/1 | 296/0/1 |
| cargo test: fe-lift | 2134/0/0 | 2145/0/0 |
| app: npm run build | rc 0 | rc 0 |
| app: npm test(tests。files は 66 passed・1 skipped) | 1308 passed・9 skipped | 1310 passed・9 skipped |
| runtime .c 91 本: emcc -fsyntax-only(wasm の flags)・clang -fsyntax-only(-DFLE_DESKTOP) | — | 0 error・0 error |
既知の失敗 `getmodulehandleexw_returns_in_every_build` は基準でも最終でも ok。`helpers_match_the_host_cpu` は止まらなかった。

## 5. 読み(解釈。実行していない)
- wasm の製品は、常に 2〜8 Worker(rt_mode)で動く。協調の経路は wasm では通らなくなる。この挙動の実行結果は未確認(段 3)。
- native のテストは GIL・TLS の経路を通らない(HEAD と同じ形)。wasm の正しさは構文検査だけで、実行は 0 件。
- 残る getenv 2 件(FLE_DWM_HZ・FLE_DISPLAYSETTINGS_FREQUENCY)は HEAD 由来で、差分の対象外。完了条件の「FLE_ の getenv 0 件」は満たさない(事実)。

## 6. 提案(依頼の条件外のため本段では除かない)
- 診断の除去: fle_dbg 系・フライトレコーダ(fle_dbg_note・fle_dbg_ring)・futex の wrap(gen_wrap.py)・fle_sys_cs_holder・trace.c の fle_dbg。`grep -rn 'fle_dbg\|fle_where\|fle_gil_contended\|fle_gil_spins\|fle_dbg_ring' lift/runtime lift/harness` で 51 行。FLE_GIL の常時化で wasm の製品でも動く。段 3 の bundle の前に、診断だけの差分で単独に除く(AGENTS §6)。

## 7. 未確認
- wasm のリンク・bundle・起動・P1〜P3・Export の sha(段 3)。依頼者機の値 0 件。
- 0315f の deadlock の再現(段 4)。Mixer 表示の条件は未測定。
