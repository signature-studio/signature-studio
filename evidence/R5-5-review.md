reviewer の検査の報告(2026-11-16、対象 6abb3a4・b7b7b5f・e84a13f・9fbf6fb・4da68cb・8b5ff31)。判定: 差し戻し。要点の保存(原文ではない)

## 一致した点

- 6 commit の変更ファイル: ci/・.github/・docs/ は 0 件。追加行の合計 +2,240/−67。
- 旧パッチの行数 1,223/1,238/365/65 は決定 0396 §1 と一致。
- 製品に残る旗: 6abb3a4 の追加行は `#ifndef`/`#define`/`#endif FLE_HARNESS_VFS_LAZY_H`(インクルードガード)の 1 件のみ。getenv・process.env・URLSearchParams・location.search・workflow_dispatch は 0。
- 画像 9 枚を目視: ctl1〜4 は「8 plugin(s)」で Balance・Filter あり、new1〜5 は「6」で無し。
- 全 report.json が engine-idle・error null・stopped false・opErrors []。loadStart は 1.34〜4.30 で全件 ≤ 8。
- 取り違え無し(netlog の wasm .br の ETag と Content-Length で確認)。
- Export: r54-r55new-chrome-1/export.json は 22,863,828 B・94c93666…4090。基準 381cdb5a…f979 と R5-4 の 4 回が一致。
- S1 の wasm 143,118,360 → 158,494,081 B・比 1.1074 は ls と manifest に一致。
- cargo の FAILED は wall_wrp2(getmodulehandleexw)1 件のみ。

## 差し戻しの指摘

- 3-1(事実に反する): evidence/R5-5.md の「Balance と Filter の処理が音に入った」。同じ節が引く 0313 追記(docs/archive/decisions/0313-g9a-balance-module.md:843)は「原 FLP では Filter が音に効かない(Insert 47 に Agogo の信号しか来ない)ので判別できない」と書き、94c93666 は Balance だけの版と全バイト一致と書く。0396 §7 も「Filter の効果は WAV に出ていない(読み)」。読みが記録と食い違う。
- 3-2: evidence/R5-5.md の firstIdleMs 4,842 は -chrome-1 の値(4842.2)。画のある -chrome-2 の report.json は 4726.5。
- 3-3: evidence/R5-5.md の「0313 の §9 の追記 09:48」は、§9 ではなく「追記(2026-11-07 09:48…)」(0313 の 836 行)。
- 3-4: evidence/R5-5.md の「hyb-gen・opt 計 約 3 分 40 秒」は、stages.log の該当 16 段の合計 190 秒(3 分 10 秒)。「hyb-n0 約 170」は 176 秒。
- 3-5: evidence/R5-5.md の「line 436」は app/scripts/build-engine-bundle.sh:437。
- 3-6(出典なし): evidence/R5-5.md の vmstat の si・so。保存された出力が無い。数える条件は evidence/R4-2.md:18 の 4 条件だが、R5-5 は si・so だけで、I/O avg10・他の Chrome 数の記録が無い。
- 3-7(出典なし): evidence/R5-5.md の npm test 1,308 通過・9 スキップ。この実行の保存ログが無い(tmp/g9/npm-build.log は typecheck と vite build だけ)。
- 3-8: evidence/R5-5.md の 2,426 → 2,427(+1 通過)の説明が無い。R5-5 の追加行に新しい #[test] は見当たらなかった。
- 3-9: evidence/R5-5.md と ROADMAP R5-5 行の「旧 RMS 5,325.57」は、0313 追記の 844 行では Insert 1 に Filter を足した変種 FLP の値。参照プロジェクトの値ではない。
- 絶対パス: app/test/unit/lazy-tree.test.ts の 2 箇所(337・381 行。b7b7b5f)に、ユーザー名を含むホームの絶対パスがある。
- 決定 0396: 「(§3)」は「(§2)」の誤り。

## 未確認(reviewer が残したもの)

- cargo の test result の合計、旧パッチの + 行数と比、engineKind(report.json に無い)、I/O avg10・他の Chrome の数、Balance・Filter の窓を開いた動作、Wine・依頼者機の比較、P1 への影響(R5-5b で別に測定)、0315 の未適用 4 ファイル(dwm_timing.rs・sweep1.rs・wall_fr9t.rs・sweep1_cases.c)、manifest の modules=8、6 件の警告の残り(Fruity Wrapper・FLEX・Frequency Shifter・Soundgoodizer・Fruity Parametric EQ 2・Fruity Delay 2)、1 回目の bundle(rc 2)の影響。ROADMAP の完了条件「7 に減るか」と実測 6 の注記が無い。

---

(以下は作成者の注。報告の一部ではない)
この報告への対応(2026-11-16、同日の直し)。直しの後の reviewer の再検査は未実施。

- 3-1: evidence/R5-5.md の該当文を「Balance が音に入ったと読める(読み)」に直し、Filter は 0313 追記(2026-11-07 09:48…)では判別できないと書いた。決定 0396 §7 の参照も同じ追記に直した。
- 3-2: firstIdleMs を -chrome-2 の 4,726.5 に直し、-chrome-1 の 4,842.2 を別に書いた。
- 3-3: 「追記(2026-11-07 09:48…)」に直した。
- 3-4: 段の秒数を stages.log から再計算して直した(hyb-gen・opt 190 秒、hyb-n0 176 秒)。bundle の秒数は見出し時刻の差(07:56:31Z〜08:02:20Z)の 349 秒に直した。元の 346 秒は出典を確認できなかった。
- 3-5: 437 行に直した(決定 0396 §5 も同じ)。
- 3-6: 保存された vmstat の出力は無いと明記し、数える条件 4 つのうち記録があるものを書いた。
- 3-7: 保存ログが無いと明記し、1,308 は報告の値として書いた。
- 3-8: +1 は R3-5 の 3ded666 が desk1_compose.rs に足した #[test] の 1 件と一致すると書いた。基準の個別のテスト名は保存されていないため、差が他に無いことは未確認と明記した。
- 3-9: 「旧 RMS 5,325.57」は変種 FLP(Insert 1 に Filter を足した FLP)の値だと注記した(evidence/R5-5.md と ROADMAP)。
- 絶対パス: app/test/unit/lazy-tree.test.ts の 2 箇所を os.homedir() からの組み立てに直した。lazy-tree のテストは 18 通過・2 スキップで、変更前と同じ。他の 3 ファイル(exit-message・failure-beacon・perf-beacon の各テストの入力例)は直していない。
- 決定 0396 の「(§3)」を「(§2)」に直した。
