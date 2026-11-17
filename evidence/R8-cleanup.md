※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。この記録は文書の掃除で、機能の進捗ではない。

# R8 掃除: 削除された CI(commit 7157394)への参照の整理

## 0 経緯と前提
- commit `7157394`(2026-11-16 09:44:36 +0900、件名 `Remove ai slop`)で `.github/workflows/` の 4 ファイルが削除された(`ci.yml` 41 行・`cloudflare-check.yml` 486 行・`engine-bundle.yml` 156 行・`measure.yml` 27 行、計 710 行)。出典: `git show --stat 7157394`。
- 依頼者の指示: OpenCode も動かさず、Workers のデプロイもしない CI は不要。CI の役割は、(1) OpenCode のエージェントを Actions で動かすこと、(2) Workers へのデプロイ、の 2 つだけ。
- 削除の時点の `.github/` は空で、この掃除の検査(`scripts/check-consistency.sh`)は 0 件だった(作業の開始時の実測)。
- 作業の途中で、別の作業が `.github/workflows/deploy-worker.yml` を含む commit `d22d557`(R6-5)を main に積んだ。この掃除はその commit の上で行った(下の 6 を参照)。

## 1 洗い出し(`expire_grep`)
検索: `grep -rnE '\.github/workflows|engine-bundle\.yml|cloudflare-check|measure\.yml|ci\.yml|workflows'` を `AGENTS.md`・`README.md`・`app/README*`・`docs/`(`docs/archive/`・`docs/decisions/`・`docs/investigations/` を除く)・`scripts/`・`prompts/`・`ci/` に対して実行。行番号は変更前の行。`scripts/`・`prompts/`・`ci/`・`app/README*` に該当は 0 件。

| 場所 | 内容 | 扱い |
|---|---|---|
| `docs/requests/0001-cloudflare-actions-secrets.md:3` | 状態の行に「確認用ワークフロー `cloudflare-check.yml` は存在し」 | 直した(削除の注記) |
| `docs/requests/0001-cloudflare-actions-secrets.md:37` | 「実行の時刻は、削除済みのため確かめられません」 | 変えない(既に削除を書いている) |
| `docs/requests/0001-cloudflare-actions-secrets.md:73` | 確認用ワークフローの実行結果 | 直した(削除の注記) |
| `docs/ROADMAP.md:18`(R0-4) | 「`engine-bundle.yml` の `paths` に当たり」 | 追記(done のまま) |
| `docs/ROADMAP.md:57`(R6-1) | `cloudflare-check.yml` の実行を読む | 追記(done のまま) |
| `docs/ROADMAP.md:64`(R7-4) | `.github/workflows/` の YAML を完了条件に含む | 追記。todo に戻した |
| `docs/ROADMAP.md:68`(R8-2) | `.github/workflows/cloudflare-check.yml` を書く | 追記(done のまま) |
| `docs/ROADMAP.md:69`(R8-3) | `.github/workflows/engine-bundle.yml` の設計 | 追記(done のまま) |
| `docs/ROADMAP.md:70`(R8-4) | `.github/workflows/engine-bundle.yml` を書く | 追記(done のまま) |
| `docs/ROADMAP.md:71`(R8-5) | `measure.yml` の実行を完了条件に含む | 追記。todo に戻した |
| `docs/ops/environment.md:16`(workflow の行) | `cloudflare-check` が 1 つ。`engine-bundle` は未作成 | 直した(削除の事実。2026-11-16 の実測を追記) |
| `docs/ops/environment.md:71`(旧 `.github/workflows/`) | 旧ツリーの `deploy.yml` の記録 | 変えない(旧ツリーの記録。4 ファイルの外) |
| `docs/ops/successor-risks.md:12` | 日付つきの例(`git ls-tree -r origin/main -- .github/workflows` の出力の例) | 変えない(過去の例) |

## 2 広い検索(「ワークフロー」「Actions」「workflow」「CI」)で見た行
直した行(前提が消えた CI の役割だったもの):
- `AGENTS.md:44`(§4): 「重い実行(ビルド・測定・並列 worker)は GitHub Actions を使う想定(基盤の整備は ROADMAP の R8)」→ 「GitHub Actions の役割は 2 つだけ: OpenCode のエージェントを動かすことと、Workers のデプロイ。ビルド・測定・読み取りの確認だけのワークフローは置かない(7157394 で削除済み)」。
- `AGENTS.md:50`(§4): 「デプロイ・確認は GitHub Actions と…」→ 「デプロイは GitHub Actions と…。読み取りだけの確認のワークフローは 7157394 で削除済みで、置かない」。
- `docs/ops/launch.md` P4・P6(依頼者の決定の記録): 文言は変えず、追記を足した(配信と確認・ビルド・測定を Actions で行う想定は無効)。
- `docs/ops/environment.md:15`(Secrets の行): 確認用ワークフローの実行は削除前の記録、と注記。

残した行(理由つき):
- `AGENTS.md:47`・`README.md:27`・`docs/ops/environment.md:12`・`docs/ops/launch.md` P2: PUBLIC の理由(GitHub Actions を最大 60 並列で動かす)は依頼者の決定 P2 の文言。変えない。
- `AGENTS.md:79`: Secrets の使用可否を GitHub Actions の実行結果で確かめる。デプロイのワークフローで残るため変えない。
- `docs/ops/environment.md:27`・`:29`: 「確認は GitHub Actions の結果か Issue の確認コマンド」。一般の規則で、ワークフロー名を指さないため残す。
- `docs/ops/authority.md:22`: 「GitHub Actions(R8)と…で行う(配信)」。配信は残るため変えない。
- `docs/ROADMAP.md:31`(R2-3)・`:66`(R7-6): 過去の CI run(engine-bundle)の結果の記録。前提ではないため変えない。
- `docs/ROADMAP.md:51`(R5-3)・`:52`(R5-4): 別の作業が扱う。変えない(依頼どおり)。
- `docs/ROADMAP.md:48`(R4-5)・`:19`(R0-5)・`:58`(R6-2): 消えたワークフローを指さない。変えない。
- `app/wrangler.jsonc:5`(コメント): 「the Worker deployed by .github/workflows/deploy.yml」。`deploy.yml` は旧ツリーのファイル(4 ファイルの外)。範囲外のため変えず、報告に回す。
- `scripts/`・`prompts/`: 参照なし。変更なし。`scripts/check-consistency.sh`・`scripts/preflight.sh` も変更なし(検査の定義も変えていない)。

## 3 ROADMAP の各行の扱い(行 / 旧 → 新 / 理由 / 出典)
判定の基準: 完了条件が、消えたファイル・ワークフローの存在または実行を要求するなら todo に戻す(a)。完了条件が記録(run の ID・evidence の記録・決定文書)で成り立つなら done のまま、追記だけにする(b)。

| 行 | 旧 → 新 | 理由 | 出典 |
|---|---|---|---|
| 18 R0-4 | done → done (b) | 完了条件は sha256 の記録。やること中の「`engine-bundle.yml` の `paths` に当たり」は前提の説明で、追記した | `docs/ROADMAP.md` 18 行、`evidence/R0-4.md` |
| 44 R4-1 | done → done (b) | 本番の読み戻しは `cloudflare-check.yml` の実行(commit `86bacf9` の R4-1b)。完了条件は記録(`evidence/R4-1b.md`) | `docs/ROADMAP.md` 44 行、`evidence/R4-1b.md` の記載 |
| 57 R6-1 | done → done (b) | 完了条件は `evidence/R6-1.md` の記録 | `docs/ROADMAP.md` 57 行、`evidence/R6-1.md` |
| 59 R6-3 | todo → todo | 「ワークフロー(R8-3 の設計・R8-4 の実装)」が指すのは削除された `engine-bundle.yml`。Worker の版の作成は別の作業の `deploy-worker.yml` を想定と注記 | `docs/ROADMAP.md` 59 行 |
| 60 R6-4 | todo → todo | 同上(`wrangler versions deploy`) | `docs/ROADMAP.md` 60 行 |
| 64 R7-4 | done → **todo** (a) | 完了条件が `.github/workflows/` の YAML(実在の値 0)を要求。その YAML は削除済み。build・test だけの job は依頼者の指示と両立しない(main が扱いを決める) | `docs/ROADMAP.md` 64 行、`evidence/R7-4.md`(run 43460019558 は削除前の実行) |
| 68 R8-2 | done → done (b) | 完了条件は固定の run 43092725843 の記録(`evidence/R8-2.md`) | `docs/ROADMAP.md` 68 行、`evidence/R8-2.md` |
| 69 R8-3 | done → done (b) | 完了条件は決定文書(`docs/decisions/0351-engine-bundle-workflow.md`)の記録。0351 の本文は変えず、末尾に 1 行追記 | `docs/ROADMAP.md` 69 行、`evidence/R8-3.md` |
| 70 R8-4 | done → done (b) | 完了条件は `evidence/R8-4.md` の記録(削除前の実行) | `docs/ROADMAP.md` 70 行、`evidence/R8-4.md` |
| 71 R8-5 | done → **todo** (a) | 完了条件の `gh run list --workflow measure.yml --limit 1` は、いま実行しても対象の workflow が無い。依頼者の指示(ビルド・測定のワークフローは置かない)と両立しない(main が扱いを決める) | `docs/ROADMAP.md` 71 行、`evidence/R8-5.md`(run 43459428108 は削除前の実行) |

R8-4・R6-3・R6-4 の依存(`R8-4`)は、依存先の行が done のまま。依存の列は変えず、注記で説明を正した。

## 4 直した行(差分)
`git diff --stat`(commit 前の状態):
```
 AGENTS.md                                        |  4 ++--
 docs/ROADMAP.md                                  | 20 ++++++++++----------
 docs/decisions/0351-engine-bundle-workflow.md    |  1 +
 docs/ops/environment.md                          |  4 ++--
 docs/ops/launch.md                               |  4 ++--
 docs/requests/0001-cloudflare-actions-secrets.md |  4 ++--
 6 files changed, 19 insertions(+), 18 deletions(-)
```
ROADMAP の変更は 10 行の置換(行 18・44・57・59・60・64・68・69・70・71)。R5-3(51 行)・R5-4(52 行)は変えていない(`git diff -U0 docs/ROADMAP.md | grep '^@@'` の結果: `-18`・`-44`・`-57`・`-59,2`・`-64`・`-68,4`)。

決定文書 `docs/decisions/0351-engine-bundle-workflow.md` は末尾に 1 行追記のみ(本文は不変)。追記は、削除済みのファイル名をバッククォートなしで書いた(検査の対象にしないため。内容は事実)。

## 5 検査
`scripts/check-consistency.sh`(作業の開始時: 0 件。commit 直前の実行は下の出力):
```
KIND  WHERE                                 WHAT                                  WHY
a     docs/decisions/0351-engine-bundle-workflow.md:10 .github/workflows/engine-bundle.yml   実在しない
consistency: 1 finding(s)
```
この 1 件は、この掃除の変更に由来しない(0351 の 10 行目の本文は元からあり、この掃除は本文を変えていない)。検査の対象になったのは、`.github/` に別の作業の `deploy-worker.yml`(commit `d22d557`)が入ったため。決定文書の本文は書き換えない規則のため、直していない(報告に回す)。

完了条件 2(消えたファイル名の検索)の結果:
```
grep -rnE 'engine-bundle\.yml|cloudflare-check\.yml|measure\.yml|ci\.yml' docs/ops docs/STATE.md docs/core scripts prompts AGENTS.md README.md app/README.md ci
docs/ops/environment.md:16: | workflow | 2026-11-12 の記録: `cloudflare-check`(ファイル名 cloudflare-check.yml、…)が 1 つ。…7157394 で `ci.yml`・`cloudflare-check.yml`・`engine-bundle.yml`・`measure.yml` の 4 ファイルが削除された(…)
```
残った 1 行は「削除された」の注記つき(完了条件の許容)。

## 6 別の作業の物
- `d22d557`(R6-5、`deploy-worker.yml`): 別の作業の commit。この掃除は触っていない。
- `ci/agents/`(未追跡)と `evidence/R0-6-failfast.txt`(未追跡): この掃除の物ではない。commit していない。
- `app/wrangler.jsonc:5` の `deploy.yml`(旧ツリーのファイル名)は範囲外。報告に回した。
