# 起動前の確認項目と起動手順(人間〔依頼者〕向け)

この文書の対象は、**後任を起動する人間**。後任(エージェント)は、起動後に台帳(`docs/BLOCKERS.md`)を見る。
**確認項目は、依頼者が「違う」と言えば、1 箇所の差し替えで済むように作ってある。** 何も言わなければ、この仮定のまま進む。
ハーネス(`scripts/`、`opencode.jsonc`、plugin、hook)は作成済み。この文書は、その名前を前提にしている。

## 1 起動前に済ませる項目(人間限定。R1 の H1〜H6 のうち、現在も成立している物)

| # | 項目 | 実測(R1、2026-11-09) | 済んだか確かめるコマンド | 済ませないとどうなるか | 後任が止まらずに進められる範囲 |
|---|---|---|---|---|---|
| 1 | **push 権限**: org の管理者が、後任が使う bot(GitHub App)の installation にこのリポジトリを追加し、Contents: write を付ける。`.github/workflows/` を push するなら Workflows: write も要る見込み(未試験) | `git push --dry-run` が 403(installation に無い) | `git push --dry-run origin HEAD`(rc=0 になる) | 後任は commit まで。GitHub に何も載らない | push 以外の全作業 |
| 2 | **公開範囲**: PUBLIC のままでよいか。意図と違うなら PRIVATE にする(admin のみ) | PUBLIC | `gh repo view signature-studio/signature-studio --json visibility` | PUBLIC の間、後任は push しない(下の確認項目 A)。lift の生成物が公開される事故を避けるため | commit まで |
| 3 | Actions を使うなら: Secrets の登録 5 つ(名前は旧 CI の定義にある。値は書かない) | 一覧が 403。存在は不明 | `gh secret list --repo signature-studio/signature-studio`(bot に Secrets: read を付けた場合) | CI でのビルド・デプロイが止まる | CI を前提にしない(確認項目 B)。wrangler で進める |
| 4 | R2 の S3 キー・API トークンの発行(CI を使う場合のみ) | OAuth のスコープに無い | ダッシュボードで発行 | 項目 3 と同じ | wrangler の OAuth で足りる範囲 |
| 5 | Cloudflare Access・DNS・custom domain の変更(新ホストが要る場合のみ) | 旧 Worker と Access は稼働中 | — | 新ホストの作業だけ止まる | 旧 Worker 名・旧ホストのまま進める |
| 6 | 依頼者機のブラウザ計測、実機 Windows の画面・音 | 依頼者機の値は 0 件 | — | G5・P1〜P3 の依頼者機の値が無い | この機械の headless Chrome の実測 |

- 起動してから上が未了でも、後任は止まらない。台帳(`docs/BLOCKERS.md`)に載り、後任が `scripts/preflight.sh` で確認する。**項目 1 が未了のまま起動するなら、後任は commit だけを積み続ける(push 無し)ことを承知しておく。**

## 2 確認項目(依頼者の最終確認が要る仮定。「この仮定で作った。違う場合は X を差し替える」)

| # | 仮定 | 違う場合の差し替え先 |
|---|---|---|
| A | **PUBLIC の間は push しない**(commit は続ける)。旧規約と旧 Worker の注記は、非公開の運用を示す(依頼者は過去に公開範囲の限定を求めた)。意図した公開か、PRIVATE に変えるかは人間の決定 | 意図した公開なら、`AGENTS.md` §4 の 4 つ目の項目(「`scripts/preflight.sh` が公開範囲 PUBLIC と報告する間は…」)を「PUBLIC でも push する。ただし push 前に `scripts/scan-forbidden.sh` を通す」に差し替える。PRIVATE に変えるなら、変えるだけ(後任は preflight が PRIVATE と出した時点で push する) |
| B | **作業の場はこのマシンのリポジトリ。CI は後で足してよいが前提にしない**。前任の「ローカルでの作業は禁止、作業はすべて GitHub Actions で」(2026-11-08)は、取り消しの記録が無い。ただし、新リポジトリの起動先はこのマシンで、旧 CI の secrets は消えている | `AGENTS.md` §4 の 1 つ目の項目(作業の場)。CI を前提にするなら、この節 1 の項目 1〜4 が全部要る |
| C | **モデルの既定は main Sonnet 5.5 + worker/reviewer Haiku 5.5**。全 Haiku でも成立するよう、依頼は小さく具体的にする | `scripts/set-model-profile.sh haiku` を人間が実行 |
| D | **goal-nudge の登録簿に、後任のセッション ID を `scripts/launch.sh` が追加する**。前任の ID は入れない(入れると 1 分以内に起こす) | `~/.config/opencode/goal-nudge.sessions`(1 行 1 ID) |
| E | **Cloudflare で触ってよいのは、Worker `fl-studio-web` と R2 `fl-web-*` だけ**。同じアカウントに無関係な資源がある | `AGENTS.md` §4 の最後の項目、`docs/ops/dangerous-ops.md` §2 |
| G | **作業木(worktree)は `gwq add` で作る**。前任の削除通知の理由の 1 つ(「worktree 管理には gwq」)。指示の初出は前任の記録で確認できなかった(依頼者が別の場で述べた可能性)。`gwq` は `~/.local/bin` にある | `AGENTS.md` §4 の 3 つ目の項目、`prompts/worker-rules.md` の禁止事項 |
| F | **旧本番(Worker・R2 の旧 bundle・Access)は稼働中**。後任は、実エンジンの実行を確認するまで、既定の入口(旧 Worker の既定の manifest)を差し替えない。代役を既定の入口に置かない(`docs/ops/honesty.md`) | `docs/ops/honesty.md` §1 の「既定の入口」 |
| H | **`docs/archive/` は git の外**(旧文書の原文 879 ファイル。作業木には置くので worker は読める)。リポジトリは PUBLIC で、逆アセンブル由来の記述と依頼者の環境の記述を含む原文を公開するかは依頼者が決める(`docs/BLOCKERS.md` の公開範囲の行)。履歴に入れると、後で公開する場合に履歴の書き換え(依頼者の権限)が要る | 公開範囲が決まったら `.gitignore` の末尾 2 行(`docs/archive/*`・`!docs/archive/README.md`)を外し、`scripts/scan-forbidden.sh` を通して commit |

## 3 起動手順(人間が実行)

| # | やること | コマンド | 確かめ方 |
|---|---|---|---|
| 1 | リポジトリを最新にする(`~/work/signature-studio`。親が commit 済みの状態) | `cd ~/work/signature-studio && git log --oneline -3` | ハーネスと規約の commit がある |
| 2 | ハーネスを設定する(identity・hook) | `scripts/setup-repo.sh` | `git config --local user.name` が出る。`git config core.hooksPath` が出る |
| 3 | モデルの構成を選ぶ(既定は `split`) | `scripts/set-model-profile.sh split`(全 Haiku なら `haiku`) | 出力が選択を示す |
| 4 | 起動前の検査 | `opencode debug agents` で解決された agent・モデルを確認。`scripts/preflight.sh` | agent は `build`・`worker`・`reviewer`(`general`・`explore`・`plan` が無い)。preflight の致命的な行が 0 |
| 5 | goal-nudge を有効化(未実行なら) | `systemctl --user daemon-reload` → 登録簿に起動後の ID を追加 → `systemctl --user enable --now goal-nudge.timer`。**前任の ID を登録しない** | `systemctl --user list-timers goal-nudge.timer` に NEXT が出る。`journalctl --user -u goal-nudge.service -n 5` |
| 6 | 起動 | `scripts/launch.sh`(preflight → メインセッション作成 → goal-nudge の登録簿へ追加 → `prompts/START.md` を送る → `/goal` を設定) | 出力に新しいセッション ID。`opencode api session.active` に出る |
| 7 | 最初の確認(起動後 10 分) | `opencode api session.active`、`scripts/audit-session.sh <ID>` | 稼働中の worker が 1 本以上。main の edit/write が 0 |

- goal-nudge は、goal を paused のまま放置せず再開する。**人が Esc で止めても、最長 1 分後に再開する**(理由を見ないため)。人が止めておきたい間は、timer を止める(`systemctl --user disable --now goal-nudge.timer`)か、登録簿から外す。
- 起動後、依頼者は最初に指示と期限だけを伝え、離席してよい。後任は質問しない。
- 起動後の定期確認(任意): `scripts/audit-session.sh`(モデル別の件数、main の編集件数、60 分超の無活動)。

## 4 起動後に人間がしてよいこと・しないこと

| してよい | しない |
|---|---|
| 指示と期限を送る。台帳の行を消す(済ませた項目)。モデルの構成を切り替える | 起動後の細かな口出し(後任は質問しない前提で設計されている)。後任の書いた台帳の行を、確認コマンドの結果なしで消す |

## 5 台帳との関係

`docs/BLOCKERS.md` の行は、`scripts/preflight.sh` が通ったら消す。起動前に項目を済ませたら、人間が消してよい(確認コマンドの出力が根拠)。
