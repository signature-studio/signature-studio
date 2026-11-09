# ブロッカー台帳(人間限定の項目)

- 載せる物: 人間にしかできないと**実測した**項目だけ(`docs/ops/authority.md` §3)。できる物・自分で試していない物を載せない。
- **`scripts/preflight.sh` が通ったら、該当行を消す。** 行を消すのは worker(main は書けない)。依頼者が済ませた場合も、確認コマンドの出力を根拠に消す。
- 後任は、この台帳に載った行があっても止まらない。右端の「止まらずに進められる範囲」の作業を、同じターンで起動する。
- 行の形式: 状態 / 確認コマンド / 確認日 / 誰がやるか / 影響 / 止まらずに進められる範囲。値(アカウント ID・ホスト名・トークン)は書かない。
- 初期内容は、起動前の調査(R1、2026-11-09 13:09〜13:20 JST の実測)による。起動後に、最初の worker が実コマンドで再確認し、確認日を更新する。

| # | 状態(実測) | 確認コマンド | 確認日 | 誰がやるか | 影響 | 後任が止まらずに進められる範囲 |
|---|---|---|---|---|---|---|
| H1 | **push 権限が無い**。`git push --dry-run` が 403(bot の installation にこのリポジトリが無い)。`.permissions` は全 false | `git push --dry-run origin HEAD`(rc=0 で消す)。`gh api repos/signature-studio/signature-studio --jq .permissions` | 2026-11-09 | 人間(org の管理者が、bot の installation にこのリポジトリを追加し、Contents: write を付ける。workflow を push するなら Workflows: write も) | GitHub に何も載らない。CI が動かない | commit まで。push 以外の全作業(ビルド・測定・docs・テスト) |
| H2 | **公開範囲が PUBLIC**。意図した公開か不明(旧規約は非公開の運用) | `gh repo view signature-studio/signature-studio --json visibility`(`PRIVATE` なら消す。意図した公開なら、人間が `AGENTS.md` §4 を差し替えてから消す) | 2026-11-09 | 人間(admin のみ変更可) | PUBLIC の間は push しない(`AGENTS.md` §4)。生成物・DLL・インストーラ・調査の抜粋が公開される事故を避ける | commit まで。`scripts/scan-forbidden.sh` を毎回通す |
| H3 | **Actions の Secrets を bot は見られない・置けない**(`secret list` が 403)。旧 5 secret は旧リポジトリと共に消え、新リポジトリにあるか不明 | `gh secret list --repo signature-studio/signature-studio`(一覧が出て、必要な 5 名が揃えば消す) | 2026-11-09 | 人間(CI を使う場合のみ必要) | CI でのビルド・デプロイが止まる | CI を前提にしない。wrangler(OAuth)で Worker を確認・デプロイ |
| H4 | **R2 の S3 キー・Cloudflare API トークンをこの機械で発行できる保証が無い**(wrangler の OAuth に API トークン管理のスコープが無い) | `wrangler whoami`(スコープ表)。発行できたかは、人間が値を登録した後の H3 の確認 | 2026-11-09 | 人間(CI を使う場合のみ必要) | H3 と同じ | wrangler の OAuth で足りる範囲(Workers・R2 の読み取り) |
| H5 | **Cloudflare Access・DNS・custom domain の変更は wrangler の OAuth のスコープに無い**。旧 Worker と Access は稼働中 | `wrangler whoami`(スコープ表)。旧ホストの稼働は `scripts/preflight.sh` | 2026-11-09 | 人間(新ホストが要る場合のみ) | 新ホスト・Access の再設定が要る作業だけ止まる | 旧 Worker 名・旧ホストのまま進める。新ホストが要る作業は、ここに載せて他を進める |
| H6 | **依頼者機のブラウザ計測、実機 Windows の画面・音の値が 0 件**。この機械では代替できない | (計測の経路を後任が整備した後)読み戻して件数を確認 | 2026-11-09 | 人間(依頼者機でしか取れない値) | 依頼者機の P1〜P3 の値が無い。G5 相当 | この機械の headless Chrome の実測。報告の未確認欄に「依頼者機の値: 0 件」を毎回書く |

## 台帳に載せてはいけない物

| 物 | 理由 | 代わりにやること |
|---|---|---|
| 「Cloudflare を確認してほしい」 | wrangler で自分で確認できる | `wrangler whoami`・`wrangler deployments list --name fl-studio-web`・`wrangler r2 bucket list` |
| 「push できない」(`--dry-run` が通るのに) | 権限が実測であるときだけ H1 | `git push --dry-run origin HEAD` の出力を貼る |
| 「ブラウザで確認してほしい」 | headless Chrome で自分で確認できる | `docs/ops/honesty.md` §3 |
| 「判断してほしい」 | 判断は main が行う | 「決定: X。根拠: Y」 |
| 「リポジトリの再作成・復元」 | 依頼者に求めない | 台帳には載せず、読める物から作業を続ける |

## 追記の手順(worker)

| きっかけ | やること | 確かめ方 |
|---|---|---|
| 「できない」と書きかけた | `docs/ops/authority.md` §5 の確認を実行 | 出力 |
| 人間限定と実測した | 上の表に 1 行追記(番号は次の H7 …)。確認コマンド・確認日を必ず書く | `grep -c '^| H' docs/BLOCKERS.md` が増えた |
| `scripts/preflight.sh` が通った | 該当行を消す(消した行は commit メッセージに番号だけ書く) | preflight の該当行が OK、台帳に行が無い |
