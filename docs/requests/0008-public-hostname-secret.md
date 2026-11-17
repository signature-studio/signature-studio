# Issue 0008: GitHub Actions の Secret `PUBLIC_HOSTNAME` の登録(Worker の公開ホスト名)

## 依頼 ID・日付
0008 / 2026-11-16

## 対象
GitHub の Actions の Secret `PUBLIC_HOSTNAME`(このリポジトリ)。値は、Cloudflare Access で守っている Worker `fl-studio-web` のカスタムドメインの、ホスト名だけ(`https://` と `/` は付けない)。実在のホスト名はこの Issue に書かない。
依頼者にしてほしい操作: GitHub の Settings → Secrets and variables → Actions で、Secret `PUBLIC_HOSTNAME` を新規に作り、値に、Cloudflare の画面で Worker `fl-studio-web` のカスタムドメインとして設定されているホスト名を入れる。返してほしい値: なし(登録のみ)。

## 合理的理由
- Secrets の登録は依頼者の権限です。エージェントは Secrets を読まず、書きません。
- 配信の後に、公開の入口が Cloudflare Access の背後にあることを確かめるには、確かめに使うホスト名が要ります。このホスト名は、配信の自動処理が使う Cloudflare の API トークン(Workers Scripts: Edit の権限)では Cloudflare の一覧から得られません(下の「自分で試した記録」の 2 回の実行)。
- ホスト名をリポジトリに書くことは、公開リポジトリのため禁止している。

## 影響範囲
- 変わる物: GitHub の Secret が 1 つ増える(`PUBLIC_HOSTNAME`)。Secret が無い間、配信の自動処理(`deploy-worker.yml`)は「公開ホスト名を決める」の手順で止まり、Cloudflare には何も書かない(今も同じ)。
- 変わらない物: Cloudflare の設定(Worker、カスタムドメイン、Access、R2 のバケット)。ほかの Secret と Variable。
- 取り消し方: GitHub の画面で Secret `PUBLIC_HOSTNAME` を削除する。削除すると、配信の自動処理は再び同じ手順で止まる。

## 登録の後の動き(依頼者の作業ではない)
登録の後は、エージェントが、最新の変更に対する配信の自動処理を動かし直し、「公開ホスト名を決める」の手順が成功するかを確かめる。次の変更は待たない。古い変更に対する配信の自動処理は動かし直さない(古い版を配信しないため)。

## 自分で試した記録
この節は記録です。依頼者の操作は上の『対象』だけ。

1. Secret の名前の出どころ(リポジトリ内。値は含まない)。`git grep -n PUBLIC_HOSTNAME -- ci archive/ci-old/deploy.yml` の出力の要点:
   - `archive/ci-old/deploy.yml` の 33 行: `PUBLIC_HOSTNAME: <CF-VALUE-REMOVED>`(旧の値は削除済み)
   - ci/lib/wrangler_config.py(削除済み): 入力名 `PUBLIC_HOSTNAME`(Access の設定と一緒に使う)
2. 一覧からの取得を実行(run 43807505570。旧 deploy-worker の workflow(決定 0400 で deploy.yml に統合し廃止)の「今 100% の版と、この Worker のカスタムドメインの一覧を読む」の段のログ。`gh run view --job <id> --log` の該当行):
   ```
   読んだ binding(値は伏せた): 変数 ACCESS_TEAM_DOMAIN, ACCESS_AUD、R2 ASSETS_BUCKET, PERF_BUCKET
   カスタムドメインの一覧: HTTP 403、エラーの番号 10000(公開ホスト名は一覧から得られない)
   ```
   同じ結果が、前回の実行 43784397668 でも出ている(`docs/decisions/0379-deploy-worker-workflow.md` の追記)。
3. 食い違いの確認: 公開ドキュメント(List Worker Domains)は、権限 Workers Scripts Write で読めると書く。2 回の実行では 403 だった。したがって、一覧は使えないと判断した(決定 0389 の決定 6)。
4. Secret が無いときの止まり方(run 43807505570 のログ):
   ```
   公開ホスト名が無い(一覧から得られず、Secret PUBLIC_HOSTNAME も無い)。配信の後の確かめ(probe)の宛先が無いので、配信の前に止めた。Cloudflare へは書いていない
   ```

## 完了の確認コマンド
```
gh run list -w deploy-worker.yml --limit 1 --json databaseId,headSha
gh run view <上の databaseId> --json jobs --jq '.jobs[].steps[] | select(.name | startswith("公開ホスト名を決める")) | .conclusion'
```
→ 1 行目の headSha が最新の変更(`git rev-parse origin/main` の値)と同じで、2 行目の出力が success

(依頼者の「登録した」の言葉ではなく、この結果で確かめる。)

## 依頼なしで続ける作業
- この依頼に関係のない作業(以前の自動処理の機能を元に戻す作業の続き)を、エージェントが進める。
- 配信の仕組みを変えた作業の完了(配信の自動処理の成功と、公開の入口の確認の結果)は、この Secret の登録の後にだけ確かめられる。それまでは「未確認」として残す。

## 確認の記録
2026-11-16: Issue を作成(https://github.com/signature-studio/signature-studio/issues/8)。登録の確認は、上の確認コマンドの出力で行う。
(登録の後に追記する。日付と、上の確認コマンドの出力 1 行)
- 2026-11-16 08:26 UTC: 自動処理の確かめ(実行 43831017455。最新の変更 84ebbd8 に対する実行)。結果: 公開ホスト名の手順で止まった。手順の出力は「公開ホスト名が無い(一覧から得られず、Secret PUBLIC_HOSTNAME も無い)」。Secret の値は空だった。Secret の名前は、この Issue の名前(PUBLIC_HOSTNAME)と一致している。登録の有無と場所は未確認。同じ時刻帯に、この Issue にラベルが 1 件付けられ、閉じられ、ロックされた(2026-11-16 08:20 UTC。コメントは 0 件)。
- 2026-11-16: 依頼者の回答: 実施しない(08:20:14Z に「実施しない」で閉じた。「slop」のラベル。コメント 0 件)。実施しないで配信が通る形に変えた(決定 0398)。配信の自動処理の実行 43833666002 は成功。公開ホスト名が得られないときは、Access の確認を飛ばして配信を続ける。この確認は飛ばした。新しい依頼は出さない。
