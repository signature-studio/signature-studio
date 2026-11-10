# 起動の手順と決定事項(人間〔依頼者〕向け)

この文書の対象は、後任を起動する人。**依頼者が起動前に済ませる必要がある物は無い。** 後任は、起動後に Issue(`docs/requests/`)で必要な物を頼み、依頼に依存しない作業を続ける。
ハーネス(`scripts/`、`opencode.jsonc`、plugin、hook)は作成済み。この文書は、その名前を前提にしている。

## 1 起動の手順(依頼者が実行)

| # | やること | コマンド | 確かめ方 |
|---|---|---|---|
| 1 | リポジトリを最新にする(`~/work/signature-studio`) | `cd ~/work/signature-studio && git log --oneline -3` | ハーネスと規約の commit がある |
| 2 | ハーネスを設定する(identity・hook) | `scripts/setup-repo.sh` | `git config --local user.name` が出る。`git config core.hooksPath` が出る |
| 3 | モデルの構成を選ぶ(既定は `split`) | `scripts/set-model-profile.sh split`(全 Haiku なら `haiku`) | 出力が選択を示す |
| 4 | 起動前の検査 | `opencode debug agents` で解決された agent・モデルを確認。`scripts/preflight.sh` | agent は `build`・`worker`・`reviewer`。preflight の致命的な行が 0 |
| 5 | goal-nudge を有効化 | `systemctl --user daemon-reload` → `systemctl --user enable --now goal-nudge.timer`。登録簿(`~/.config/opencode/goal-nudge.sessions`)には `scripts/launch.sh` が起動後の ID を追加する | `systemctl --user list-timers goal-nudge.timer` に NEXT が出る |
| 6 | 起動 | `scripts/launch.sh`(preflight → メインセッション作成 → goal-nudge の登録簿へ追加 → `prompts/START.md` を送る → `/goal` を設定) | 出力に新しいセッション ID。`opencode api session.active` に出る |
| 7 | 最初の確認(起動後 10 分) | `opencode api session.active`、`scripts/audit-session.sh <ID>` | 稼働中の worker が 1 本以上。main の edit/write が 0 |

- goal-nudge は、goal を paused のまま放置せず再開する。人が Esc で止めても、最長 1 分後に再開する。人が止めておきたい間は、timer を止める(`systemctl --user disable --now goal-nudge.timer`)。
- 起動後、依頼者は指示と期限だけを伝えて離席してよい。後任は質問しない。
- 起動後の定期確認(任意): `scripts/audit-session.sh`(モデル別の件数、main の編集件数、60 分超の無活動)。

## 2 決定済みの事項(P1〜P8。依頼者の決定 2026-11-10。これが旧文書の記述に優先する)

| # | 決定 |
|---|---|
| P1 | push: identity は作業ディレクトリで決まる。通常の `git push origin main` で push する(scan 通過後。force 系・別 identity は使わない)。`.permissions` は根拠にしない。「push 403」は前任がリポジトリ外のディレクトリで測った誤り |
| P2 | 公開範囲: PUBLIC は意図的(GitHub Actions を最大 60 並列で数時間動かすため)。公開を理由に push を止めない。公開されて困る物は git に入れない(`.gitignore`) |
| P3 | 依頼者の協力: 責任範囲外の作業(トークン・Secrets・権限・実機の計測)に限り、Issue の形式で応じる(`docs/ops/requests.md`) |
| P4 | Cloudflare: 後任は `wrangler`・`cf` を読み取りも含め実行しない。配信と確認は GitHub Actions と、依頼者が Issue に応じて登録する Secrets で行う |
| P5 | 実機の計測(依頼者機のブラウザ・実機 Windows の画面・音・120 fps)は headless で代替できない。Issue で依頼者に計測を頼む。返事を待たず headless の実測で進め、依頼者機の値は「依頼中」と書く |
| P6 | 作業の場: このマシンのリポジトリ。重い実行(ビルド・測定・並列 worker)は GitHub Actions を使う想定 |
| P7 | 依頼の区分と一覧の文書は廃止。依頼は Issue(`docs/requests/`)へ |
| P8 | preflight の行名: `agents git-identity git-hooks gh-identity repo-api push-dry-run toolchain disk scan goal goal-nudge` |

## 3 後任が起動後に Issue で頼むと予想される物

| 依頼 | 理由 | 参照 |
|---|---|---|
| Actions で使う Secrets の登録(Cloudflare の API トークン、R2 の S3 キー) | 配信を GitHub Actions で行う設計(R8)。トークンの発行と登録は依頼者の権限(P3) | `docs/requests/`(起票済みの Issue の一覧と状態) |
| Access・DNS・custom domain の変更(新ホストが要る場合のみ) | 後任は Cloudflare を実行しない(P4) | 同上 |
| 実機・依頼者機の計測(P5) | headless では代替できない | 同上 |
| `.github/workflows/` を push するための権限(足りない場合のみ) | org の権限は依頼者の権限(P3) | 同上 |

## 4 残る仮定(依頼者が違うと言えば差し替える)

| # | 仮定 | 違う場合 |
|---|---|---|
| G | ワークツリー(worktree)は `gwq add` で作る。`git worktree add` は使わない | `AGENTS.md` §4 のワークツリーの項目 |
| H | モデルの既定は main Sonnet 5.5 + worker/reviewer Haiku 5.5 | `scripts/set-model-profile.sh haiku` を依頼者が実行 |
| I | goal-nudge は、人の Esc で止まっても最長 1 分後に再開する | timer を止める(§1 の 5) |

- `docs/archive/` は、近く git に入れる方針に変わった。git の内(追跡する)。公開されて困る物は無い(依頼者の判断。P2)。`.gitignore` の `docs/archive/*` の 2 行は、入れる作業の commit で外す。

## 5 登録簿との関係

起動後の状態は、`scripts/preflight.sh` の行(P8)で確かめる。Issue の確認の記録は、Issue の末尾に書く(`docs/ops/requests.md`)。
