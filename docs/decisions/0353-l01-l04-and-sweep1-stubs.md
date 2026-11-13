# 0353: L01〜L04 の 4 本のパッチと sweep1 のリンク用スタブ 2 つを取り込む
分類: L01・L02 = (b 契約の実装)。L03・L04・sweep1 スタブ = 3 分類のどれにも当たらない(テストの期待・守り・リンク用スタブ。新しい処理・分岐を足していない。製品に入らない)
撤回条件: R1-5 の実エンジンの起動で `engine-idle` に達しないとき、または R2-7 の P1 が基準 +100 ms を超えたとき、該当パッチ(L01 または L02)を外す。sweep1 が `fle_win.c` をリンクする作りに変わったら、sweep1 のスタブ 2 つを外す。
日付: 2026-11-14   ROADMAP: R1   一覧: -   コミット: 77dc3d8..4e719a5

## 決定
決定: 次の 4 本を `lift/` に取り込む。`patches/ci1/0323/diffs/0323-contract-fixes.diff`(L01)、`patches/ci1/0323/diffs/0323b-disabled-input.diff`(L02)、`patches/ci1/0330/0323c-test-expectations.diff`(L03)、`patches/ci1/0331/0323d-test-guards.diff`(L04)。あわせて `lift/harness/sweep1/sweep1_cases.c` と `lift/crates/fe-lift/tests/sweep1.rs` の既存のスタブの並びに、空スタブ 2 つ(`void fle_win_input_call_hooks(void) {}`・`void fle_win_input_hit_list(void) {}`)を足す。根拠: L01 は `fle_input.c` に 2 つの呼び出しを足し、定義を `fle_win.c` に置く。sweep1 は window 層をリンク用の空スタブで置き換える小さなハーネスで、`sweep1_cases.c` の 20 行に「these cases never reach」と明記している。同じ理由で同じ形のスタブが要る。`fle_win.c` を sweep1 のリンク対象に足す案は、ハーネスの前提に反し、生成物(`fle_dispatch_table` など 56 件)の供給が要るため採らない(`evidence/R1-link.md` §3)。

## 分類の根拠
- L01(0323)・L02(0323b): (b 契約の実装)。外部 API(Win32 の窓・入力の契約)の不足を埋める。`getenv`・`#ifdef FLE_` の `+` 行は 0 件(`git diff 39a1db6 HEAD -- lift | grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_'` → 0)。逆スイッチ・旗は作っていない。
- L03(0323c)・L04(0323d): テストの期待・守りを直す。製品に入らない。
- sweep1 のスタブ 2 つ: テストのリンク用スタブの追加。新しい処理・分岐を足していない(0352 のテスト入力パスの修正と同じ扱い)。

## 途中の commit では `cargo test` が失敗する
4 本を 1 本ずつ別の commit にしたため、各 commit は単独では緑でない。実測(`evidence/R1-1.md` の段ごとの表。通過・失敗(合計)・無視):
- L01 だけ: 2411・6・1(新規の失敗 5 件)
- L01〜L02: 2409・8・1(新規 7 件)
- L01〜L03: 2411・6・1(新規 5 件)
- L01〜L04: 2414・3・1(新規 2 件。sweep1 の `cases_equal…`・`undefined_inputs…` のリンク失敗。`fle_win_input_call_hooks`・`fle_win_input_hit_list` の未定義)
- L01〜L04 + スタブ(この commit `4e719a5`): 2416・1・1(失敗は取り込み前と同じ `getmodulehandleexw_returns_in_every_build` の 1 件。`evidence/R1-final.md` §2)

## commit の並び
- `77dc3d8` lift: L01 契約修正(背景消去・HTTRANSPARENT・フック)を取り込む(0323)
- `2834b02` lift: L02 無効化された窓への入力を取り込む(0323b)
- `f2e1a35` lift: L03 古くなった native テストの期待を直す(0323c)
- `21b978b` lift: L04 古くなった native テストの守りを直す(0323d)
- `4e719a5` lift: sweep1 のリンク用スタブに fle_win_input_call_hooks・fle_win_input_hit_list を足す(L01 の呼び出しのため)
- この文書と `evidence/R1-final.md`・`evidence/R1-final.log` の commit

## 実測(コマンド・条件・日付・n)
- 取り込み前: `evidence/R1-1-before.log` → 2416・1・1(SKIPPED 0)。
- 取り込み後(スタブ込み): `cd lift && cargo test --workspace --offline --no-fail-fast`(`FLENGINE_PE` は `binaries/win64/FLEngine_x64.dll`)→ rc 101、通過 2416・失敗 1・無視 1・SKIPPED 0。`grep -c 'undefined reference' evidence/R1-final.log` → 0。ログ `evidence/R1-final.log`(227 KB)。
- 取り込み件数: L01 = 16 ファイル、L02 = 5、L03 = 2、L04 = 3(`evidence/R1-dryrun.md` の表と一致)。
- パッチの当て方: dry-run rc 0 → 本適用 rc 0。`.rej` 0。GNU patch が `fle_input.c` の隣に作る `.orig` ファイルは作業物として削除。

## 未確認
- 実エンジン(FLEngine)は実行していない。4 本のパッチの意味上の正しさ(Wine との差、実エンジンでの挙動)は未確認。
- HEAD での再実行は `evidence/R1-head.log`(HEAD `807222f`。期待どおり: 失敗 1 件・SKIPPED 0・test result 219 行・undefined reference 0)。
- `getmodulehandleexw_returns_in_every_build` の失敗は取り込み前から残る 1 件。原因は未確認(本作業の対象外)。
- R1-4 以降のパッチ(`patches/alpha22/…`)は取り込んでいない。

## 出典
- `patches/ci1/0323/diffs/0323-contract-fixes.diff`、`patches/ci1/0323/diffs/0323b-disabled-input.diff`、`patches/ci1/0330/0323c-test-expectations.diff`、`patches/ci1/0331/0323d-test-guards.diff`
- `evidence/R1-1.md`(段ごとの結果表)、`evidence/R1-link.md`(リンク失敗の切り分け。案 A・案 B)、`evidence/R1-dryrun.md`(適用可否)、`evidence/R1-final.md`・`evidence/R1-final.log`(最終の検査)
- `docs/core/win32-contract.md` §3(契約の不足 (b))
- `docs/decisions/0352-write-only-vars.md`(分類の書き方の前例)
- FLE_HTERROR の二重定義(0323b が足した重複。値は同じ `(-2)`)を 1 つにした。分類: (a)。`evidence/R1-hterror.log`
