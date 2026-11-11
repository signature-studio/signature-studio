# Issue(依頼者への依頼)

依頼者に頼むのは、自分にできない物だけ。雛形は `docs/requests/TEMPLATE.md`。出す前に `docs/ops/environment.md` の該当コマンドを実行し、結果を「自分で試した記録」に貼る。

## いつ出すか
| 出す | 出さない |
|---|---|
| トークンの発行・Secrets の登録・権限の付与 | push(`git push --dry-run origin HEAD` が通れば、自分で push する) |
| Cloudflare の確認・変更(後任者は wrangler を実行しない) | `gh`・`git`・`scripts/`・headless Chrome で確かめられる物 |
| 実機の画面・音・120 fps の計測(headless では代替できない) | 決定・許可・確認を依頼者に回す物 |
| 責任範囲外の作業で、理由・対象・影響範囲を明示できる物 | 理由・対象・影響範囲が書けない物 |

## 必須項目
| 項目 | 書くこと |
|---|---|
| 依頼 ID・日付 | `NNNN`(連番)と `YYYY-MM-DD` |
| 対象 | 具体名(Worker 名・バケット名・Secrets 名・機械)。実在のアカウント ID・ホスト名・トークンは書かない(`<…>` のプレースホルダ) |
| 合理的理由 | 依頼者にしかできない理由 |
| 影響範囲 | 変わる物・変わらない物・取り消し方 |
| 自分で試した記録 | 実行したコマンドと出力(抜粋)。トークンを含む行は貼らない |
| 完了の確認コマンド | 依頼者の「終わった」の言葉に頼らず、これで確かめる。期待する出力を併記 |
| 依頼なしで続ける作業 | 依頼に依存しない次の作業 |

## 出し方
1. `docs/requests/NNNN-<slug>.md` を `docs/requests/TEMPLATE.md` から作り、コミットして `git push origin main`(禁止語・秘密の検査を通った後)。
2. 同じ内容を `gh issue create --title "[依頼] <件名>" --body-file docs/requests/NNNN-<slug>.md` でも出す。
3. Issue が拒否されたら、その出力を Issue の末尾に貼ってコミットする。
4. 報告の「依頼」の行に Issue のパスを書く。

## 例
| 件名 | 良い依頼の内容 |
|---|---|
| Secrets の登録 | 対象: Cloudflare の Worker `<Worker名>` 向けのトークンを Secret `<SECRET名>` に登録。理由: トークンの発行と Secrets の登録は依頼者の権限。影響: Secret 1 件の追加のみ。最小の権限(対象バケットの読み書き)。確認: 依頼者の連絡。Secrets と Variables の中身も一覧も、エージェントは確認しない |
| 依頼者機での計測 | 対象: 依頼者の Windows 機とブラウザ、所定プロジェクト(`~/ダウンロード/serina1-serum2.zip`)。理由: 120 fps と音切れは実機でしか測れない。手順: `<計測手順のパス>` を読み取りのみで実行し、返された結果ファイルを `evidence/` に置く。確認: `grep -c 'fps' <結果ファイル>` が 1 以上 |
| push が 403(リポジトリのディレクトリで測った場合に限る) | 対象: `git push origin main`。理由: リポジトリのディレクトリで `git push --dry-run origin HEAD` が 403。影響: push のみ停止、コミットは続ける。自分で試した記録: リポジトリのディレクトリで実行した `gh auth status`(トークンの行は除く)と `git push --dry-run origin HEAD` の出力(両方貼る)。確認: `git push --dry-run origin HEAD` が成功する |

| 悪い依頼 | 違反 | 直し方 |
|---|---|---|
| 「push できるのに push 権限をください」 | `git push --dry-run origin HEAD` が通れば自分で push できる | `git push origin main` し、`git ls-remote origin main` で確かめる |
| 依頼者に `git status` の結果を回す文 | `git` で自分で確かめられる | 実行し、出力を報告に貼る |
| 「Secrets を追加してください」(理由・対象・影響範囲なし) | 依頼者が内容を特定できない | 必須項目を書く |
| 「この方針で良いですか」 | 決定を依頼者に回している | 「決定: X。根拠: Y」を書いて実行する |
| 「別のディレクトリで測った 403 を根拠にする」 | push のアカウントは作業ディレクトリで決まる(P1)。測り方が誤り | リポジトリのディレクトリで `gh auth status` と `git push --dry-run origin HEAD` を実行し、出力を貼る |

## 出した後
- 止まらない。同じターンで依頼に依存しないサブエージェント(作業用)を起動する。
- 催促しない。結果は完了の確認コマンドで確かめる。
- 確認できたら、Issue の末尾に「確認: YYYY-MM-DD <コマンドの出力 1 行>」を追記してコミットする。
