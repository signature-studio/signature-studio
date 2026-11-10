# 依頼票 0001: GitHub Actions 用の Cloudflare 資格情報の登録(Worker とバケット限定)

## 依頼 ID・日付
0001 / 2026-11-10

## 対象
このリポジトリ(`origin`)の GitHub Actions の Secrets 5 つと Variables 1 つ。名前は秘密ではない。値はこの票に書かない。
- Secret `CLOUDFLARE_API_TOKEN`: Account の「Workers Scripts: Edit」だけの API トークン(対象 Worker `fl-studio-web` のデプロイ用)。
- Secret `R2_ARTIFACT_ACCESS_KEY_ID`・`R2_ARTIFACT_SECRET_ACCESS_KEY`: バケット `fl-web-artifacts` の読み取りだけに限定した S3 キーの組。
- Secret `R2_CACHE_ASSETS_ACCESS_KEY_ID`・`R2_CACHE_ASSETS_SECRET_ACCESS_KEY`: バケット `fl-web-cache` と `fl-web-assets` の 2 つだけに限定した読み書きの S3 キーの組。
- Variable `CLOUDFLARE_ACCOUNT_ID`: アカウント ID(依頼者が登録する。値はこの票に書かない)。

依頼者にしてほしい操作:
1. Cloudflare の API トークン作成画面で、上の権限のトークンを作る。
2. R2 の API トークン画面で、バケットを指定した S3 キーを 2 組作る(`fl-web-artifacts` の読み取りのみの 1 組、`fl-web-cache` と `fl-web-assets` の読み書きの 1 組)。
3. GitHub の Settings → Secrets and variables → Actions で、Secrets 5 つと Variable 1 つを登録する(`gh secret set` と `gh variable set` でもよい)。
返してほしい値: `gh secret list` と `gh variable list` の名前の一覧(値は貼らない)。

旧 CI(`docs/archive/decisions/0217-ci-deploy.md`・`0224-workers-status.md`、原文)との差:
- [文書] 旧 CI は Variables 0 個。今回は `CLOUDFLARE_ACCOUNT_ID` の 1 個を足す。
- [文書] 旧 CI の `wrangler deploy` は、カスタムドメインの更新で失敗した(0217。Workers Scripts: Edit だけではゾーンを引けなかった)。0217 の修正は `wrangler versions upload` と `versions deploy` で、後任もこの方法を前提にする。ドメインつきの本番 Worker への versions 操作は、旧記録では未試験。この権限だけで足りるかは未実測(R8-4 の実行で確かめる)。
- 配信バケットへの書き込みは `wrangler r2 object put` ではなく S3 API(0217。バケット限定の R2 トークンは wrangler の REST 経路で 403)。
- [文書] `fl-web-cache` の S3 キーは、プレフィックスで絞れず、`cache/` と `perf/` も読み書きできる(0224)。

## 合理的理由
後任は `wrangler`・`cf` を実行しない(依頼者の決定)。Cloudflare の確認とデプロイは GitHub Actions でしかできない。トークンが無いと、Worker `fl-studio-web` の更新、R2 の bundle の配信、Cloudflare の状態の読み取りの確認が、どれもできない。

## 影響範囲
- 変わる物: GitHub の Secrets 5 つと Variable 1 つが増える。Cloudflare 側は、トークン 3 つ(API トークン 1・S3 キー 2 組)が増える。
- 変わらない物: 他のバケット・他の Worker・Access のアプリ・ゾーン・DNS。この票の Secrets を使う最初のワークフローは読み取りだけ(R8-2)。書き込みは後続のワークフローが行い、その commit と実行ログが残る。
- 取り消し方: Cloudflare 側でトークン 3 つを失効させる。GitHub 側で Secrets 5 つと Variable 1 つを削除する。
- ログには値が出ない(ワークフローは `::add-mask::` で伏せる)。

## 自分で試した記録
実測 2026-11-10。リポジトリのディレクトリで実行。identity は `gh auth status` の active account(名前は書かない)。

```
$ gh secret list
(出力なし(0 件)、rc 0)
$ gh variable list
failed to get variables: HTTP 403: Resource not accessible by integration (https://api.github.com/repos/signature-studio/signature-studio/actions/variables?per_page=100)
Exited with code 1
```

[文書] 旧 CI の記録(0217。この票の実測ではない): Workers Scripts: Edit だけで Worker の deploy は通った。バケット限定の R2 トークンは `wrangler r2 object put` が 403、S3 API では書けた。

## 完了の確認コマンド
```
gh secret list                                        → 5 名(上の Secrets の名前と一致)
gh run list --workflow cloudflare-check.yml --limit 1 → success(Variable CLOUDFLARE_ACCOUNT_ID が無いと、この実行は失敗する)
```

注: この作業用のアカウントでは `gh variable list` が 403 になる(2026-11-10 の実測。上の「自分で試した記録」)。そのため Variable の有無は一覧では見られない。上の実行結果で確かめる。

## 依頼なしで続ける作業
R0 の全行、R1〜R3、R4-2〜R4-4、R5 の測定系(Actions を使わない物)。

## 確認の記録
2026-11-10: Issue を作成 → https://github.com/signature-studio/signature-studio/issues/1
(確認後に追記: 日付・コマンドの出力 1 行)
