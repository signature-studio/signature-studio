実エンジン: 実行なし(`cargo test`。lift の検査)
取り込み後(スタブ追加まで): 通過 2416・失敗 1・SKIPPED 0
失敗名の集合: 取り込み前と一致(`getmodulehandleexw_returns_in_every_build` の 1 件)
判定: 取り込んだ(L01〜L04 と sweep1 のスタブ 2 つ。commit `77dc3d8`〜`4e719a5`)

# 1 取り込みの手順と検査

- 前提: `git status --short lift` → 空、`git diff --stat -- lift` → 空、ブランチ main(`evidence/R1-final.log` の前の HEAD は `39a1db6`)。
- パッチ 4 本を順に当てた(最初の確認と、スタブ込みの本番の確認)。各 dry-run rc 0、本適用 rc 0、`.rej` 0。`.orig` は GNU patch が offset 時に作った作業物で、`ls -la` で確かめて削除(`lift/runtime/fle_input.c.orig`。L01・L02 の試行と当て直しのたびに 1 件ずつ作られ、その都度削除)。
- `grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' <パッチ>` → 4 本とも 0。取り込んだ commit の `+` 行(`git diff 39a1db6 HEAD -- lift`)でも 0。
- スタブ 2 つの差分(`git diff -- lift/harness/sweep1/sweep1_cases.c lift/crates/fe-lift/tests/sweep1.rs`):
  - `lift/harness/sweep1/sweep1_cases.c` に `+void fle_win_input_call_hooks(void) {}`・`+void fle_win_input_hit_list(void) {}` の 2 行(`fle_win_input_showwindow` の次)
  - `lift/crates/fe-lift/tests/sweep1.rs` の 161 行の末尾に、同じ 2 つを `+void fle_win_input_call_hooks(void) {} void fle_win_input_hit_list(void) {}` の 1 行として追加
  - 他の行・テストの期待値・clang の引数・`OBJS` は変えていない(`git diff` の `+` 行は 3 行だけ)。
- 一度 `git checkout -- lift` で戻し、commit のために 4 本を 1 本ずつ当て直した(新規の `lift/harness/c0323/` は `ls -la` で作成時刻を確かめてから削除)。スタブ 2 行は戻したあと同じ 2 行を同じ位置に入れ直し、`git diff` の `+` 行が前回の検査と同じであることを確かめた。戻した後の `cargo test` は再実行していない。

# 2 `cargo test`(スタブ追加まで・取り込み後)

コマンド: `cd lift && FLENGINE_PE=$HOME/work/signature-studio/binaries/win64/FLEngine_x64.dll cargo test --workspace --offline --no-fail-fast > ../evidence/R1-final.log 2>&1; echo rc=$?`
- rc=101
- `grep -c SKIPPED evidence/R1-final.log` → 0
- `grep '^test result' evidence/R1-final.log | wc -l` → 219
- `grep '^test .* FAILED$' evidence/R1-final.log` → `test getmodulehandleexw_returns_in_every_build ... FAILED`(1 件)
- `awk '/^test result/{p+=$4; f+=$6; i+=$8} END{print p, f, i}' evidence/R1-final.log` → `2416 1 1`(通過・失敗・無視。無視は取り込み前と同じ 1 件)
- `grep -c 'undefined reference' evidence/R1-final.log` → 0(sweep1 の 2 件のリンク失敗は消えた)

取り込み前の同じ数字(`evidence/R1-1-before.log`): `awk` → `2416 1 1`、`grep -c SKIPPED` → 0。失敗名も `getmodulehandleexw_returns_in_every_build` の 1 件で同じ。

# 3 途中の commit の数字(`evidence/R1-1.md` の段ごとの実測。表の「失敗」は取り込み前からの 1 件を含む合計)

| 状態 | ログ | 通過・失敗(合計)・無視 | うち新規の失敗 |
|---|---|---|---|
| L01 だけ | `evidence/R1-1-after.log`(再利用) | 2411・6・1 | 5 |
| L01〜L02 | `evidence/R1-2-after.log` | 2409・8・1 | 7 |
| L01〜L03 | `evidence/R1-3a-after.log` | 2411・6・1 | 5 |
| L01〜L04 | `evidence/R1-3b-after.log` | 2414・3・1 | 2(sweep1 の 2 件) |
| L01〜L04 + スタブ(最終) | `evidence/R1-final.log` | 2416・1・1 | 0 |

- 各 commit は単独では緑ではない。L01〜L04 の commit では sweep1 の 2 件のリンク失敗が残る。スタブの commit で失敗は取り込み前と同じ 1 件になる。
- 「L01 だけ」の段は R1-1 の段の再実行をせず、`evidence/R1-1-after.log` を再利用している(`evidence/R1-1.md` の記録どおり)。途中の commit ごとの `cargo test` は、この作業では個別に再実行していない。

# 4 commit の並び

`git log --oneline -8`(実行時点):
```
4e719a5 lift: sweep1 のリンク用スタブに fle_win_input_call_hooks・fle_win_input_hit_list を足す(L01 の呼び出しのため)
21b978b lift: L04 古くなった native テストの守りを直す(0323d)
f2e1a35 lift: L03 古くなった native テストの期待を直す(0323c)
2834b02 lift: L02 無効化された窓への入力を取り込む(0323b)
77dc3d8 lift: L01 契約修正(背景消去・HTTRANSPARENT・フック)を取り込む(0323)
39a1db6 R1-link: sweep1 の 2 件のリンク失敗を一時コピーで切り分け(lift は未変更。fle_win.c を足すだけでは通らない)
cee5cd4 R1-1〜R1-3b: 取り込みの判定を記録(取り込まず。R1-3b 後も 2 件の失敗が残る)
8f4f5dc R6-4: 既定の入口の差し替えワークフローの草案(evidence/ の外に置く。実行なし。R2・Worker に書き込んでいない)
```
- commit のファイル数: L01 = 16、L02 = 5、L03 = 2、L04 = 3(`evidence/R1-dryrun.md` の表と一致)。スタブ = 2。

# 5 未確認

- 実エンジン(FLEngine)は実行していない。取り込んだ 4 本のパッチの意味上の正しさ(Wine との差、実エンジンでの挙動)は未確認。
- 取り込み後の `cargo test` は、スタブを入れた状態で 1 回だけ実行した。途中の commit ごとの実行はしていない(上の 3 は `evidence/R1-1.md` の既存の実測)。
- 戻して当て直した後の `lift/` の中身は、`git diff` の `+` 行で前回と同じことを確かめたが、`cargo test` は再実行していない。
- `getmodulehandleexw_returns_in_every_build` の失敗の原因は未確認(取り込み前から残っている 1 件。本作業の対象外)。
- ビルド(`app/scripts/build-engine-bundle.sh`)は走らせていない。push 後の engine-bundle の実行 `43485443530`(headSha `b6dd943`、event push)は、この作業中に `gh run view 43485443530` で status completed・conclusion success(createdAt 2026-11-13T18:43:50Z)を確認した。engine-bundle が作った成果物の中身は、この記録では確かめていない。
- 詳細は `evidence/R1-engine-bundle.md`(実行の status・conclusion・所要、9 段の結果、wasm の SHA-256、伏せ検査)。

# 6 HEAD での再確認(取り込み後の木 = HEAD の木)

- `git status --short lift` → 空。`git rev-parse HEAD` → `807222f074eff44cb0d395623fc22a7164e8960a`。
- コマンド: `cd lift && FLENGINE_PE=$HOME/work/signature-studio/binaries/win64/FLEngine_x64.dll cargo test --workspace --offline --no-fail-fast > ../evidence/R1-head.log 2>&1; echo rc=$?` → rc=101。ログ `evidence/R1-head.log`(226,871 B)。
- `grep -c SKIPPED evidence/R1-head.log` → 0
- `grep '^test result' evidence/R1-head.log | wc -l` → 219
- `grep -c 'undefined reference' evidence/R1-head.log` → 0
- `grep '^test .* FAILED$' evidence/R1-head.log` → `test getmodulehandleexw_returns_in_every_build ... FAILED`(1 件)
- `awk '/^test result/{p+=$4; f+=$6; i+=$8} END{print p, f, i}' evidence/R1-head.log` → `2416 1 1`
- 期待との比べ: 一致(失敗 1 件・SKIPPED 0・test result 219 行・undefined reference 0)。
- この節の実行で、§1 と §5 の「戻して当て直した後の `cargo test` は再実行していない」は解消した(HEAD の木での 1 回)。
- 未確認: 実エンジン(FLEngine)は実行していない。失敗 1 件の原因は未確認(§5 と同じ)。
