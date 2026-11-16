# Issue 0004: Actions でエージェントを動かすためのモデルの認証情報

## 依頼 ID・日付
0004 / 2026-11-16

## 対象
GitHub の Secrets(このリポジトリ `signature-studio` の Actions 用)。依頼者にしてほしい操作:

1. AWS の IAM で、この用途だけの利用者(IAM ユーザー)を 1 つ作る。コンソールへのサインインは不要。
2. その利用者に、下の権限を付ける。
3. 次のどちらかの方式で認証情報を作り、Secrets に登録する(依頼者が選ぶ。名前は下のとおり)。
   - 方式 A: Amazon Bedrock の API キーを作り、Secret `AWS_BEARER_TOKEN_BEDROCK` に登録する。
   - 方式 B: アクセスキーを作り、Secret `AWS_ACCESS_KEY_ID` と `AWS_SECRET_ACCESS_KEY` に登録する。
4. Secret `AWS_REGION` に、Haiku 5.5 の推論プロファイルを使う地域の名前を登録する(例: `<地域>`)。
5. 値は、この Issue、コミット、チャットのどれにも書かない。

権限の文案(`<…>` は依頼者が埋める。他の行動・他のモデルは許可しない):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "InvokeOneHaikuModelOnly",
      "Effect": "Allow",
      "Action": [
        "bedrock:InvokeModel",
        "bedrock:InvokeModelWithResponseStream"
      ],
      "Resource": [
        "<Haiku 5.5 の推論プロファイルの ARN(地域は <地域>)>",
        "<その推論プロファイルが指す基になるモデルの ARN(推論プロファイルの説明に従う)>"
      ]
    }
  ]
}
```

Secret の名前: `AWS_REGION` と、方式 A の `AWS_BEARER_TOKEN_BEDROCK`、または方式 B の `AWS_ACCESS_KEY_ID` と `AWS_SECRET_ACCESS_KEY`。Cloudflare の Secrets(`CLOUDFLARE_API_TOKEN`・`R2_*`)は使わない。

## 合理的理由
- Amazon Bedrock の認証情報は、依頼者の AWS アカウントの権限そのもの。後任のエージェントは、AWS の認証情報の発行も、GitHub Secrets の登録もできない(依頼者の権限。`docs/ops/requests.md`)。
- 登録が無いと、`ci/tasks/<名前>.md` の push で動く作業用サブエージェントは、1 回も動けない。並列 60 の実行は、この登録に依存する。
- ワークフローの配線(設定の組み立て・タスクの読み込み・opencode の導入・認証情報が無いときの停止)は、登録が無くても検査できる。それは済ませてある(決定 0380)。

## 影響範囲
- 変わる物: Secrets が、方式 A なら 2 件、方式 B なら 3 件増える。AWS 側に、IAM の利用者 1 人と、キー 1 組(方式 A なら API キー 1 つ)が増える。
- 変わらない物: Cloudflare の Secrets と Variables は、このワークフローに渡さない。他の Secrets・Variables、他のワークフロー、他の AWS の資源には触れない(この利用者が呼べるのは、上の 2 つの資源だけ)。
- 料金: `ci/tasks/<名前>.md` を push するたびに、Bedrock の呼び出しの料金が発生する。1 push の上限は 60 件、1 件の上限は 350 分。依頼者は、IAM の権限と AWS Budgets で上限を決められる。
- 取り消し方: AWS で、キーを無効にして削除する(または利用者を削除する)。GitHub の Secrets から、登録した名前を削除する。

## 自分で試した記録
エージェントは `gh secret`・`gh variable` を実行しない(依頼者の決定。worker の拒否の規則)。ワークフローの認証情報の検査は、次の出力で確かめた(2026-11-16、この機械。値は読まない。HAS_* は真偽だけ)。

```
$ env -i HAS_REGION=false HAS_BEARER=false HAS_KEY_ID=false HAS_SECRET=false bash ci/agents/check-credentials.sh
::error::Secret AWS_REGION が登録されていない(docs/requests/0004-actions-model-credentials.md を参照)
rc=1

$ env -i HAS_REGION=true HAS_BEARER=true HAS_KEY_ID=false HAS_SECRET=false bash ci/agents/check-credentials.sh
モデルの認証: 方式 A(API キー)
rc=0

$ opencode --version
opencode v2.2.6
```

## 完了の確認コマンド
- 依頼者が登録した後、名前だけを一覧する(値は表示されない):
  `gh secret list --repo <所有者>/signature-studio` → 出力に `AWS_REGION` と、選んだ方式の名前(方式 A は `AWS_BEARER_TOKEN_BEDROCK`、方式 B は `AWS_ACCESS_KEY_ID` と `AWS_SECRET_ACCESS_KEY`)が出る。
- 登録の後、エージェントが小さなテスト用のタスク 1 件を push して確かめる:
  `gh run list --workflow agent.yml --limit 1` → 結論が `success`(run ジョブが通る)。

## 依頼なしで続ける作業
- 登録が無い間、`ci/tasks/<名前>.md` のタスクのファイルは置かない(認証情報が来るまで run を動かさない)。
- ワークフローの検査(設定の組み立て・タスクの一覧・認証情報の停止)は、登録が無くても続けて確かめる。

## 確認の記録
(登録後に追記: 日付・`gh secret list` の出力の名前の行・`gh run list` の 1 行)
