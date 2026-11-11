# Issue 0001: GitHub Actions 用の Cloudflare 資格情報の登録(Worker とバケット限定)

依頼 ID・日付: 0001 / 2026-11-10

## 依頼者にお願いしたい作業

Cloudflare で API トークン 1 つと R2 のアクセスキー 2 組を作り、GitHub の Secrets 5 つと Variables 1 つに登録してください。後任者は Cloudflare の操作を直接できないため、この登録がないと、ウェブ版の配信と Cloudflare の状態の確認ができません。

## やってほしいこと

対象は、このリポジトリ(signature-studio)の GitHub Actions の Secrets 5 つと Variables 1 つです。名前は秘密ではないので、この Issue に書いています。値は、作った画面にしか表示されないので、この Issue には書かないでください。

1. Cloudflare で API トークンを作る。
   - 権限は「Workers Scripts: Edit」だけにする(Account の権限)。
   - 対象の Worker は `fl-studio-web`。
   - 作ったトークンを、GitHub の Secret `CLOUDFLARE_API_TOKEN` に登録する。
2. Cloudflare の R2 で、バケットを指定したアクセスキーを 2 組作る。
   - 1 組目: バケット `fl-web-artifacts` の読み取りだけ。Secrets の `R2_ARTIFACT_ACCESS_KEY_ID`(Access Key ID)と `R2_ARTIFACT_SECRET_ACCESS_KEY`(Secret Access Key)に登録する。
   - 2 組目: バケット `fl-web-cache` と `fl-web-assets` の読み書き。Secrets の `R2_CACHE_ASSETS_ACCESS_KEY_ID`(Access Key ID)と `R2_CACHE_ASSETS_SECRET_ACCESS_KEY`(Secret Access Key)に登録する。
3. GitHub の Settings → Secrets and variables → Actions で、次の 5 つの Secrets と 1 つの Variable を登録する。
   - Secrets: `CLOUDFLARE_API_TOKEN`、`R2_ARTIFACT_ACCESS_KEY_ID`、`R2_ARTIFACT_SECRET_ACCESS_KEY`、`R2_CACHE_ASSETS_ACCESS_KEY_ID`、`R2_CACHE_ASSETS_SECRET_ACCESS_KEY`
   - Variable: `CLOUDFLARE_ACCOUNT_ID`(値は、Cloudflare の画面に表示される自分のアカウント ID)

## なぜ必要か

GitHub Actions で Cloudflare に配信する。後任者は Cloudflare の操作を直接できないため、この経路でしか配信と確認ができない。

## 影響

- 変わる物: GitHub の Secrets が 5 つ、Variables が 1 つ増える。Cloudflare 側に、API トークン 1 つと R2 のアクセスキー 2 組(合計 3 つ)が増える。
- 変わらない物: 他のバケット、他の Worker、ドメインの設定などの Cloudflare の他の設定。最初に動かすワークフローは読み取りだけで、Cloudflare の中身を書き換えるのは後から作るワークフローです。その変更は、GitHub の履歴と実行ログに残ります。
- 取り消し方: Cloudflare の画面で、作ったトークンとアクセスキーを削除(失効)する。GitHub の画面で、Secrets 5 つと Variable 1 つを削除する。
- GitHub Actions の実行ログには、値は表示されません。

## 登録しなくても進む作業

実エンジンの動作確認、性能の測定、規則の修正は、登録がなくても進めます。登録が必要なのは、配信と、Cloudflare の状態の確認だけです。

## 登録できたか確認する方法

登録後、エージェントが GitHub Actions のテスト実行で確認します。

---

## 技術的な詳細(エージェント向け)

### 旧 CI との差

- [文書] 旧 CI は Variables 0 個。今回は `CLOUDFLARE_ACCOUNT_ID` の 1 個を足す。
- [文書] 旧 CI の `wrangler deploy` は、カスタムドメインの更新で失敗した(0217。Workers Scripts: Edit だけではゾーンを引けなかった)。0217 の修正は `wrangler versions upload` と `versions deploy` で、エージェントもこの方法を前提にする。ドメインつきの本番 Worker への versions 操作は、旧記録では未確認。この権限だけで足りるかは未実測(R8-4 の実行で確かめる)。
- 配信バケットへの書き込みは `wrangler r2 object put` ではなく S3 API(0217。バケット限定の R2 トークンは wrangler の REST 経路で 403)。
- [文書] `fl-web-cache` の S3 キーは、プレフィックスで絞れず、`cache/` と `perf/` も読み書きできる(0224)。

### 合理的理由

エージェントは `wrangler`・`cf` を実行しない(依頼者の決定)。Cloudflare の確認とデプロイは GitHub Actions でしかできない。トークンが無いと、Worker `fl-studio-web` の更新、R2 への配信ファイルの配置、Cloudflare の状態の読み取りの確認が、どれもできない。

### 自分で試した記録

実測 2026-11-10。リポジトリのディレクトリで実行。作業用のアカウントは `gh auth status` の active account(名前は書かない)。

```
$ gh secret list
(出力なし(0 件)。コマンドは成功)
$ gh variable list
failed to get variables: HTTP 403: Resource not accessible by integration (https://api.github.com/repos/signature-studio/signature-studio/actions/variables?per_page=100)
Exited with code 1
```

[文書] 旧 CI の記録(0217。この票の実測ではない): Workers Scripts: Edit だけで Worker の deploy は通った。バケット限定の R2 トークンは `wrangler r2 object put` が 403、S3 API では書けた。

### 完了の確認コマンド

```
gh secret list                                        → 5 名(上の Secrets の名前と一致)
gh run list --workflow cloudflare-check.yml --limit 1 → success(Variable CLOUDFLARE_ACCOUNT_ID が無いと、この実行は失敗する)
```

注: `gh variable list` は作業用のアカウントでは 403 になる(2026-11-10 の実測)。そのため Variable の有無は一覧では見られない。上の実行結果で確かめる。

### 依頼なしで続ける作業

R0 の全行、R1〜R3、R4-2〜R4-4、R5 の測定系(Actions を使わない物)。

### 確認の記録

2026-11-10: Issue を作成(Issue 1)
(確認後に追記: 日付・コマンドの出力 1 行)
