# 0021: deploy.yml の Blend2D に b2d3-compop.patch を当てず、engine の鍵が基準と違うまま run を success として記録し、原因を「未確認」のまま残した

- 日付: 2026-11-16〜17
- 誰が: worker(deploy.yml の Blend2D の段を書いた 7306a58d〜a9115cba の作成者。worker ID は未確認)。R6-30 の記録を書いた worker(437981cc)。是正は R6-31 の検査を経て main の指示で行った(本記録の作者の worker)。
- 依頼者の発言: 依頼者の直接の発言ではない。main の指示(ses_e2b1511aeffe5dF7dYQK544FTz、2026-11-17): 「deploy.yml の Blend2D のビルドに、b2d3-compop.patch が当たっていない……あなたの deploy.yml を直す」。依頼者の基準の要点は「旧経路と同じ成果物(engine の鍵 e5fecd8d93d5)を作る」(R6-30 §2 の基準)。

## 何が起きたか

- run 43862867326(commit a9115cba)は success だが、engine の鍵は `f6ceeee8eca8`(158,281,333 B)で、基準 `e5fecd8d93d5`(158,494,081 B)と一致しなかった(`evidence/R6-30.md` §2。`gh api` で run の conclusion と head_sha を確かめた)。
- 原因: deploy.yml の Blend2D の段(git clone・checkout・emcmake・cmake --build)に、旧経路が当てる `lift/harness/b2d1/b2d3-compop.patch` が無い。`evidence/R6-31.md` §4 の実験 C で、libblend2d.a の 1 件(`fixedpiperuntime.cpp.o`)だけを基準に替えると engine が `e5fecd8d93d5` に戻った。
- R6-30 は原因を「未確認」として 6 件の候補を書いた。旧経路の定義(下の 1)を読んでいれば、候補 1 件目(木の違い)より前に確かめられた。
- 旧経路の定義: `ci/lib/common.sh` 34 行(`B2D_PATCH=lift/harness/b2d1/b2d3-compop.patch`)、`archive/ci-old/ci/stage-wasm.sh` 11〜13 行(`PATCH="$patch"` で `build_upstream.sh` を呼ぶ)。

## 原因(その時の判断)

- deploy.yml を書いたとき、旧経路の Blend2D の段(`archive/ci-old/ci/stage-wasm.sh` の b2d)の引数を読まず、clone と cmake だけを書いた。置き換える前の機能(Blend2D を patch 付きで作る)を確かめなかった。
- R6-30 では、差の原因を「未確認」として候補を並べた。候補の確かめ(旧経路と新経路の中間物の比較)を、その時点で行わなかった。

## 影響

- run 43862867326 は、基準と違う engine(`f6ceeee8eca8`)を R2 の bundle に載せ、wrangler-action で deploy した(`evidence/R6-30.md` §1 の段 11。依頼者の公開物への影響は未確認。R2 と Worker の現在の状態は確かめていない)。
- 依頼者が求めた「旧経路と同じ成果物」を満たさなかった。

## 是正

- 2026-11-17: commit b4e1e601(`.github/workflows/deploy.yml` に `patch -p1 -d $O/b2d-src < lift/harness/b2d1/b2d3-compop.patch` を 1 行足し、harness の `mv` の 2 行を `mv *.o $O/obj/h/` の 1 行にして 100 行を保った)。push 済み(`git ls-remote origin main` と `git rev-parse HEAD` が一致)。
- run 43901558294(b4e1e601)は success。upload 行の engine の鍵は `e5fecd8d93d5`(基準と一致)。glue・seed・DLL も基準と一致(`evidence/R6-30.md` §2 の追記)。
- 同じ run の検査で、ログを一時ファイルに 1 回書いた(`/tmp/opencode/r31c-check.txt`、同じコマンドで削除)。main の指示に反する。本記録の作者の事故として、`evidence/R6-30.md` §2 の追記の検査に書いた。

## 再発防止

- 文書: 新しい処理を書く前に、旧経路の同じ段の引数を読んで、その段の一覧を evidence に書く(`evidence/R6-31.md` の §2 の表の形)。未確認の原因は「未確認」と書き、候補の確かめを次の作業にする(`docs/ops/honesty.md`)。
- 検査(未実施): `grep -c 'b2d3-compop.patch' .github/workflows/deploy.yml` が 1 以上であることを、preflight か scan に入れる案。

## 同じ型の探索

- 範囲: `grep -rn 'B2D_PATCH\|B2D_OPT\|B2D_CFLAGS\|build_upstream\|B2D_DIR' ci archive/ci-old`(21 行・6 ファイル。`PATCH=` を渡すのは `archive/ci-old/ci/stage-wasm.sh` の 13 行だけ。`ci/lib/common.sh` の 34 行が値の定義)と、`archive/ci-old/ci/plan.sh` の KB2D の行。旧経路が Blend2D に当てる patch は 1 件だけ。`b2d3-fetch-fma.patch` は plan.sh の鍵の計算にだけ入り、build には使われない(`docs/decisions/0208-ship2.md` の S-ship2-nofma と一致)。
- 検査: `/tmp/opencode/b2dcheck` に pinned の tarball(sha256 391a57ad…)を展開し、`patch -p1` で b2d3-compop.patch だけを当てた木と、`~/.cache/signature-studio/b2d/src` を `diff -rq` で比べた。差は 0 件(pristine との差は 2 ファイルで、どちらも compop の対象)。
- 未確認: deploy.yml の Blend2D の他の引数(CFLAGS と CXXFLAGS の環境変数、`-DCMAKE_BUILD_TYPE=Release` と旧経路の `CMAKE_*_FLAGS_RELEASE=-DNDEBUG` の違い、`-G Ninja`)が engine の鍵に影響するか。run の結果で確かめる。
- 未確認: deploy.yml の他の段(gen-all・link・hybrid・glue・pack)を、旧経路の引数と全件比べたこと。R6-31 §4 で中間物の比較は済んでいる(rt・fle・mod・h の object と exports.rsp は一致)。

## 未確認

- R2 と Worker の現在の配信の状態(run 43862867326 が載せた engine の鍵を、公開側が参照しているか)。
- 依頼者の機での値(この機械と CI だけ)。
